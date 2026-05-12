/// Block-based streaming encryption format.
///
/// Each block is at most `MAX_BLOCK_SIZE` bytes after encryption. Block 0 carries a header
/// (version + nonce + plaintext-size); subsequent blocks reuse the same layout minus the header.
///
/// Wire layout per block:
///   Block 0:  [version u8 | nonce [u8;12] | plaintext_size u32-be | ciphertext | AES-GCM tag]
///   Block N:  [ciphertext | AES-GCM tag]
use aes_gcm::{AeadInPlace, Aes256Gcm, Nonce};
use thiserror::Error;

use crate::utils::encryption::{ceil_div, EncryptionError};

pub const MAX_BLOCK_SIZE: usize = 32 * 1024; // 32 KiB
pub const HEADER_SIZE: usize = 1 + 12 + 4; // version + nonce + plaintext_size_bytes
pub const VERSION: u8 = 1;
const TAG_SIZE: usize = 16;
const NONCE_SIZE: usize = 12;
/// Maximum plaintext bytes that fit in the first block (after header and AES-GCM tag).
pub const BLOCK_0_DATA_SIZE: usize = MAX_BLOCK_SIZE - HEADER_SIZE - TAG_SIZE;
/// Maximum plaintext bytes that fit in subsequent blocks.
pub const BLOCK_N_DATA_SIZE: usize = MAX_BLOCK_SIZE - TAG_SIZE;
const EC_DATA_BLOCKS: usize = 16;

#[derive(Error, Debug)]
pub enum StreamEncError {
    #[error("buffer too small for encryption")]
    BufferTooSmall,
    #[error("invalid version: expected {expected}, got {got}")]
    InvalidVersion { expected: u8, got: u8 },
    #[error("data too short: {0}")]
    DataTooShort(String),
    #[error("target size {0} is too small")]
    TargetTooSmall(usize),
    #[error("encryption error: {0}")]
    Encryption(#[from] EncryptionError),
    #[error("aead error")]
    Aead,
}

/// Number of encrypted blocks needed to hold `plaintext_size` bytes.
pub fn num_blocks(plaintext_size: usize) -> usize {
    if plaintext_size == 0 {
        return 0;
    }
    if plaintext_size <= BLOCK_0_DATA_SIZE {
        return 1;
    }
    1 + ceil_div(plaintext_size - BLOCK_0_DATA_SIZE, BLOCK_N_DATA_SIZE)
}

/// How many plaintext bytes are stored in block `block_index` for a file of `plaintext_size`.
pub fn block_data_size(plaintext_size: usize, block_index: usize) -> usize {
    if block_index == 0 {
        plaintext_size.min(BLOCK_0_DATA_SIZE)
    } else {
        let remaining = plaintext_size
            .saturating_sub(BLOCK_0_DATA_SIZE + (block_index - 1) * BLOCK_N_DATA_SIZE);
        remaining.min(BLOCK_N_DATA_SIZE)
    }
}

/// Total encrypted bytes (including headers and tags) for `plaintext_size` bytes of input.
pub fn overhead(plaintext_size: usize) -> usize {
    let n = num_blocks(plaintext_size);
    if n == 0 {
        return 0;
    }
    n * TAG_SIZE + HEADER_SIZE
}

/// Return the slice of `ciphertext` that corresponds to encrypted block `block_index`.
pub fn encrypted_block(ciphertext: &[u8], block_index: usize) -> &[u8] {
    if block_index == 0 {
        let end = (HEADER_SIZE + BLOCK_0_DATA_SIZE + TAG_SIZE).min(ciphertext.len());
        &ciphertext[..end]
    } else {
        let start = (HEADER_SIZE + BLOCK_0_DATA_SIZE + TAG_SIZE)
            + (block_index - 1) * (BLOCK_N_DATA_SIZE + TAG_SIZE);
        let end = (start + BLOCK_N_DATA_SIZE + TAG_SIZE).min(ciphertext.len());
        &ciphertext[start..end]
    }
}

/// Compute the maximum plaintext size that fits within `target_size` encrypted bytes.
pub fn max_plaintext_size_for_target(target_size: usize) -> Result<usize, StreamEncError> {
    if target_size < HEADER_SIZE + TAG_SIZE {
        return Err(StreamEncError::TargetTooSmall(target_size));
    }
    let after_header = target_size - HEADER_SIZE;
    // block 0 holds up to BLOCK_0_DATA_SIZE plaintext + TAG_SIZE
    if after_header <= BLOCK_0_DATA_SIZE + TAG_SIZE {
        return Ok(after_header - TAG_SIZE);
    }
    let remaining_after_block0 = after_header - (BLOCK_0_DATA_SIZE + TAG_SIZE);
    let extra_blocks = remaining_after_block0 / (BLOCK_N_DATA_SIZE + TAG_SIZE);
    let leftover = remaining_after_block0 % (BLOCK_N_DATA_SIZE + TAG_SIZE);
    let last_block_plain = if leftover >= TAG_SIZE {
        leftover - TAG_SIZE
    } else {
        0
    };
    Ok(BLOCK_0_DATA_SIZE + extra_blocks * BLOCK_N_DATA_SIZE + last_block_plain)
}

/// Parse the block-0 header.  Returns `(version, nonce, plaintext_size)`.
pub fn parse_header(data: &[u8]) -> Result<(u8, [u8; NONCE_SIZE], u32), StreamEncError> {
    if data.len() < HEADER_SIZE {
        return Err(StreamEncError::DataTooShort(format!(
            "header needs {} bytes, got {}",
            HEADER_SIZE,
            data.len()
        )));
    }
    let version = data[0];
    let nonce: [u8; NONCE_SIZE] = data[1..1 + NONCE_SIZE].try_into().unwrap();
    let plaintext_size = u32::from_be_bytes(data[1 + NONCE_SIZE..HEADER_SIZE].try_into().unwrap());
    Ok((version, nonce, plaintext_size))
}

/// Derive a per-block nonce from the initial nonce by incrementing the last 4 bytes.
pub fn block_nonce(initial_nonce: [u8; NONCE_SIZE], block_index: usize) -> [u8; NONCE_SIZE] {
    let mut n = initial_nonce;
    let counter = block_index as u32;
    let pos = NONCE_SIZE - 4;
    let base = u32::from_be_bytes(n[pos..].try_into().unwrap());
    n[pos..].copy_from_slice(&base.wrapping_add(counter).to_be_bytes());
    n
}

/// Encrypt `buf` in-place using the streaming block format.
///
/// `buf` must already contain the plaintext and must have capacity ≥ `plaintext_size + overhead(plaintext_size)`.
/// Returns the number of bytes written (the full encrypted length).
pub fn encrypt(key: &[u8], buf: &mut Vec<u8>, info: &str) -> Result<usize, StreamEncError> {
    use crate::utils::encryption::Encryption;
    #[cfg(not(target_arch = "wasm32"))]
    use aes_gcm::aead::rand_core::RngCore;
    #[cfg(not(target_arch = "wasm32"))]
    use aes_gcm::aead::OsRng;

    let plaintext_size = buf.len();
    let n = num_blocks(plaintext_size);
    if n == 0 {
        return Ok(0);
    }
    let total_size = plaintext_size + overhead(plaintext_size);
    if buf.capacity() < total_size {
        return Err(StreamEncError::BufferTooSmall);
    }

    // Derive the cipher from the key+info
    let enc = Encryption::new(key, info)?;
    let gcm = enc.gcm_cipher(info)?;

    // Generate a random initial nonce
    #[cfg(not(target_arch = "wasm32"))]
    let initial_nonce = {
        let mut n = [0u8; NONCE_SIZE];
        OsRng.fill_bytes(&mut n);
        n
    };
    #[cfg(target_arch = "wasm32")]
    let initial_nonce = {
        use web_sys::window;
        let mut n = [0u8; NONCE_SIZE];
        window()
            .unwrap()
            .crypto()
            .unwrap()
            .get_random_values_with_u8_array(&mut n)
            .unwrap();
        n
    };

    // Encrypt blocks right-to-left so we don't overwrite unread plaintext.
    // Reserve space first.
    buf.resize(total_size, 0);

    // Offsets of blocks in the output buffer (right to left)
    for bi in (0..n).rev() {
        let plain_start = if bi == 0 {
            0
        } else {
            BLOCK_0_DATA_SIZE + (bi - 1) * BLOCK_N_DATA_SIZE
        };
        let plain_len = block_data_size(plaintext_size, bi);

        let enc_start = if bi == 0 {
            0
        } else {
            (HEADER_SIZE + BLOCK_0_DATA_SIZE + TAG_SIZE) + (bi - 1) * (BLOCK_N_DATA_SIZE + TAG_SIZE)
        };

        // Copy plaintext to the right position before encrypting in place
        // For block 0: dest = HEADER_SIZE; for others: dest = enc_start
        let dest_plain_start = if bi == 0 { HEADER_SIZE } else { enc_start };
        buf.copy_within(plain_start..plain_start + plain_len, dest_plain_start);

        let nonce_arr = block_nonce(initial_nonce, bi);
        let nonce = Nonce::from_slice(&nonce_arr);

        let tag = gcm
            .encrypt_in_place_detached(
                nonce,
                b"",
                &mut buf[dest_plain_start..dest_plain_start + plain_len],
            )
            .map_err(|_| StreamEncError::Aead)?;

        // Append tag
        let tag_dest = dest_plain_start + plain_len;
        buf[tag_dest..tag_dest + TAG_SIZE].copy_from_slice(&tag);

        if bi == 0 {
            // Write header at the very start
            buf[0] = VERSION;
            buf[1..1 + NONCE_SIZE].copy_from_slice(&initial_nonce);
            let ps = plaintext_size as u32;
            buf[1 + NONCE_SIZE..HEADER_SIZE].copy_from_slice(&ps.to_be_bytes());
        }
    }

    Ok(total_size)
}

/// Decrypt all blocks from `buf`, writing plaintext in-place from offset 0.
/// Returns the number of plaintext bytes written.
pub fn decrypt_all_blocks(
    key: &[u8],
    buf: &mut [u8],
    info: &str,
    version: u8,
) -> Result<usize, StreamEncError> {
    use crate::utils::encryption::Encryption;

    if version != VERSION {
        return Err(StreamEncError::InvalidVersion {
            expected: VERSION,
            got: version,
        });
    }

    let (file_version, initial_nonce, plaintext_size_u32) = parse_header(buf)?;
    if file_version != VERSION {
        return Err(StreamEncError::InvalidVersion {
            expected: VERSION,
            got: file_version,
        });
    }
    let plaintext_size = plaintext_size_u32 as usize;
    let n = num_blocks(plaintext_size);

    let enc = Encryption::new(key, info)?;
    let gcm = enc.gcm_cipher(info)?;

    let mut out_pos = 0usize;
    for bi in 0..n {
        let plain_len = block_data_size(plaintext_size, bi);
        decrypt_block(&gcm, buf, initial_nonce, VERSION, bi, plain_len)?;
        // Copy decrypted plaintext to front
        let src = if bi == 0 {
            HEADER_SIZE
        } else {
            (HEADER_SIZE + BLOCK_0_DATA_SIZE + TAG_SIZE) + (bi - 1) * (BLOCK_N_DATA_SIZE + TAG_SIZE)
        };
        // Safe: src ≥ out_pos for forward iteration because header/tag overhead is positive
        buf.copy_within(src..src + plain_len, out_pos);
        out_pos += plain_len;
    }

    Ok(plaintext_size)
}

/// Decrypt a single block in-place.
/// The block's ciphertext+tag must reside at the correct offset within `buf`.
pub fn decrypt_block(
    gcm: &Aes256Gcm,
    buf: &mut [u8],
    initial_nonce: [u8; NONCE_SIZE],
    _version: u8,
    block_index: usize,
    plaintext_size: usize,
) -> Result<usize, StreamEncError> {
    let offset = if block_index == 0 {
        HEADER_SIZE
    } else {
        (HEADER_SIZE + BLOCK_0_DATA_SIZE + TAG_SIZE)
            + (block_index - 1) * (BLOCK_N_DATA_SIZE + TAG_SIZE)
    };
    let ct_end = offset + plaintext_size + TAG_SIZE;
    if buf.len() < ct_end {
        return Err(StreamEncError::DataTooShort(format!(
            "block {} needs {} bytes at offset {}",
            block_index,
            plaintext_size + TAG_SIZE,
            offset
        )));
    }

    let nonce_arr = block_nonce(initial_nonce, block_index);
    let nonce = Nonce::from_slice(&nonce_arr);

    let tag_bytes: [u8; TAG_SIZE] = buf[offset + plaintext_size..ct_end]
        .try_into()
        .map_err(|_| StreamEncError::Aead)?;
    let tag = aes_gcm::aead::generic_array::GenericArray::<
        u8,
        aes_gcm::aead::generic_array::typenum::U16,
    >::from(tag_bytes);

    gcm.decrypt_in_place_detached(nonce, b"", &mut buf[offset..offset + plaintext_size], &tag)
        .map_err(|_| StreamEncError::Aead)?;

    Ok(plaintext_size)
}
