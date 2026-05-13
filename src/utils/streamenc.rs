use aes_gcm::aead::AeadInPlace;
use thiserror::Error;

use crate::utils::encryption::{self, EncryptionError};

// Stream encryption constants
const NONCE_SIZE: usize = 12;
const TAG_SIZE: usize = 16;
const EC_DATA_BLOCKS: usize = 16;
const VERSION_SIZE: usize = 1;
const PLAINTEXT_SIZE_BYTES: usize = 4;

pub const MAX_BLOCK_SIZE: usize = 32 * 1024; // 32 KiB
pub const HEADER_SIZE: usize = VERSION_SIZE + NONCE_SIZE + PLAINTEXT_SIZE_BYTES; // 17
pub const VERSION: u8 = 1;
pub const BLOCK0_DATA_SIZE: usize = MAX_BLOCK_SIZE - HEADER_SIZE - TAG_SIZE; // 32735
pub const BLOCKN_DATA_SIZE: usize = MAX_BLOCK_SIZE - TAG_SIZE; // 32752
pub const MIN_CIPHER_TEXT_SIZE: usize =
    ((HEADER_SIZE + TAG_SIZE + 1 + EC_DATA_BLOCKS - 1) / EC_DATA_BLOCKS) * EC_DATA_BLOCKS;

#[derive(Error, Debug)]
pub enum StreamEncError {
    #[error(transparent)]
    EncryptionError(#[from] EncryptionError),

    #[error("buffer too small")]
    BufferTooSmall,

    #[error("header too short")]
    HeaderTooShort,

    #[error("version mismatch: got {got}, want {want}")]
    VersionMismatch { got: u8, want: u8 },

    #[error("target size not aligned")]
    TargetSizeNotAligned,

    #[error("target size too small")]
    TargetSizeTooSmall,
}

/// Returns the number of ciphertext blocks for a given plaintext size.
pub fn num_blocks(plaintext_size: usize) -> usize {
    if plaintext_size == 0 {
        return 1;
    }
    if plaintext_size <= BLOCK0_DATA_SIZE {
        return 1;
    }
    let remaining = plaintext_size - BLOCK0_DATA_SIZE;
    1 + encryption::ceil_div(remaining, BLOCKN_DATA_SIZE)
}

/// Returns the un-padded byte count of plaintext in the block at `block_index`.
pub fn block_data_size(plaintext_size: usize, block_index: usize) -> usize {
    let n = num_blocks(plaintext_size);
    if block_index >= n {
        return 0;
    }
    if block_index == 0 {
        return plaintext_size.min(BLOCK0_DATA_SIZE);
    }
    let already = BLOCK0_DATA_SIZE + (block_index - 1) * BLOCKN_DATA_SIZE;
    let remaining = plaintext_size.saturating_sub(already);
    remaining.min(BLOCKN_DATA_SIZE)
}

/// Returns the slice of `ciphertext` that belongs to block at `block_index`.
pub fn encrypted_block(ciphertext: &[u8], block_index: usize) -> &[u8] {
    if block_index == 0 {
        let end = MAX_BLOCK_SIZE.min(ciphertext.len());
        return &ciphertext[..end];
    }
    let start = MAX_BLOCK_SIZE + (block_index - 1) * MAX_BLOCK_SIZE;
    if start >= ciphertext.len() {
        return &[];
    }
    let end = (start + MAX_BLOCK_SIZE).min(ciphertext.len());
    &ciphertext[start..end]
}

/// Returns the total overhead bytes added by stream encryption.
pub fn overhead(plaintext_size: usize) -> usize {
    let n = num_blocks(plaintext_size);
    let raw = HEADER_SIZE + n * TAG_SIZE;
    // Align to EC_DATA_BLOCKS
    encryption::ceil_div(raw, EC_DATA_BLOCKS) * EC_DATA_BLOCKS - raw
        + HEADER_SIZE
        + n * TAG_SIZE
}

/// Returns the maximum plaintext size that, when encrypted, fits within `target_size` bytes.
pub fn max_plaintext_size_for_target(target_size: usize) -> Result<usize, StreamEncError> {
    if target_size % EC_DATA_BLOCKS != 0 {
        return Err(StreamEncError::TargetSizeNotAligned);
    }
    if target_size < MIN_CIPHER_TEXT_SIZE {
        return Err(StreamEncError::TargetSizeTooSmall);
    }
    // Binary search for the largest plaintext_size such that encrypted size <= target_size
    let mut lo = 0usize;
    let mut hi = target_size;
    while lo < hi {
        let mid = lo + (hi - lo + 1) / 2;
        let enc_size = encrypted_size(mid);
        if enc_size <= target_size {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    Ok(lo)
}

fn encrypted_size(plaintext_size: usize) -> usize {
    let n = num_blocks(plaintext_size);
    let raw = HEADER_SIZE + plaintext_size + n * TAG_SIZE;
    // Pad last block to align to EC_DATA_BLOCKS
    encryption::ceil_div(raw, EC_DATA_BLOCKS) * EC_DATA_BLOCKS
}

/// Parse the stream encryption header from `data`.
/// Returns (version, initial_nonce, plaintext_size).
pub fn parse_header(data: &[u8]) -> Result<(u8, [u8; NONCE_SIZE], u32), StreamEncError> {
    if data.len() < HEADER_SIZE {
        return Err(StreamEncError::HeaderTooShort);
    }
    let version = data[0];
    if version != VERSION {
        return Err(StreamEncError::VersionMismatch {
            got: version,
            want: VERSION,
        });
    }
    let mut nonce = [0u8; NONCE_SIZE];
    nonce.copy_from_slice(&data[VERSION_SIZE..VERSION_SIZE + NONCE_SIZE]);
    let pt_bytes: [u8; 4] = data[VERSION_SIZE + NONCE_SIZE..HEADER_SIZE]
        .try_into()
        .unwrap();
    let plaintext_size = u32::from_be_bytes(pt_bytes);
    Ok((version, nonce, plaintext_size))
}

/// Derives the per-block nonce by incrementing the last 4 bytes of the initial nonce.
pub fn block_nonce(initial_nonce: [u8; NONCE_SIZE], block_index: usize) -> [u8; NONCE_SIZE] {
    let mut nonce = initial_nonce;
    let counter = block_index as u32;
    let last4 = u32::from_be_bytes(nonce[NONCE_SIZE - 4..].try_into().unwrap());
    let new_last4 = last4.wrapping_add(counter);
    nonce[NONCE_SIZE - 4..].copy_from_slice(&new_last4.to_be_bytes());
    nonce
}

/// Encrypt `buf` in-place. `buf` must have sufficient capacity for the overhead.
/// Returns the number of bytes written.
pub fn encrypt(key: &[u8], buf: &mut Vec<u8>, info: &str) -> Result<usize, StreamEncError> {
    use aes_gcm::{Aes256Gcm, KeyInit, Key};
    #[cfg(not(target_arch = "wasm32"))]
    use aes_gcm::aead::rand_core::RngCore;
    #[cfg(not(target_arch = "wasm32"))]
    use aes_gcm::aead::OsRng;

    let plaintext_size = buf.len();
    let n = num_blocks(plaintext_size);
    let required_capacity = encrypted_size(plaintext_size);

    if buf.capacity() < required_capacity {
        return Err(StreamEncError::BufferTooSmall);
    }

    // Derive key
    let derived = encryption::Encryption::derive_key(key, info)?
        .ok_or(EncryptionError::NoKeyAvailable)?;

    // Generate initial nonce
    #[cfg(not(target_arch = "wasm32"))]
    let mut initial_nonce = [0u8; NONCE_SIZE];
    #[cfg(not(target_arch = "wasm32"))]
    OsRng.fill_bytes(&mut initial_nonce);

    #[cfg(target_arch = "wasm32")]
    let initial_nonce = {
        use web_sys::window;
        let mut n = [0u8; NONCE_SIZE];
        let crypto = window().unwrap().crypto().unwrap();
        crypto.get_random_values_with_u8_array(&mut n).unwrap();
        n
    };

    let gcm_key = Key::<Aes256Gcm>::from_slice(&derived);
    let gcm = Aes256Gcm::new(gcm_key);

    // Encrypt right-to-left so we don't overwrite unprocessed plaintext.
    // First extend buf to the full required size (zero-filled for padding).
    buf.resize(required_capacity, 0);

    // We need to work right-to-left. Collect block boundaries first.
    let mut block_ranges: Vec<(usize, usize)> = Vec::with_capacity(n); // (pt_start, pt_end) in original plaintext
    {
        let mut pt_offset = 0usize;
        for i in 0..n {
            let bds = block_data_size(plaintext_size, i);
            block_ranges.push((pt_offset, pt_offset + bds));
            pt_offset += bds;
        }
    }

    // Process blocks in reverse order (last block first)
    for i in (0..n).rev() {
        let (pt_start, pt_end) = block_ranges[i];
        let bn = block_nonce(initial_nonce, i);
        let aes_nonce = aes_gcm::Nonce::from_slice(&bn);

        // Destination in output: block i starts at:
        //   block 0: HEADER_SIZE
        //   block k: HEADER_SIZE + BLOCK0_DATA_SIZE + TAG_SIZE + (k-1)*(BLOCKN_DATA_SIZE + TAG_SIZE)
        let ct_start = if i == 0 {
            HEADER_SIZE
        } else {
            HEADER_SIZE + (BLOCK0_DATA_SIZE + TAG_SIZE) + (i - 1) * (BLOCKN_DATA_SIZE + TAG_SIZE)
        };

        // Copy plaintext into the right position (may already be there for first encryption pass)
        // For safety, move plaintext forward to ct_start
        if pt_start != ct_start {
            buf.copy_within(pt_start..pt_end, ct_start);
        }

        // Encrypt in place: the ciphertext replaces plaintext at ct_start..ct_end+TAG_SIZE
        let pt_len = pt_end - pt_start;
        let block_slice = &mut buf[ct_start..ct_start + pt_len];
        // We need to convert block_slice to a Vec for in-place encryption
        // aes_gcm encrypt_in_place appends the tag
        let mut block_buf = block_slice.to_vec();
        gcm.encrypt_in_place(aes_nonce, b"", &mut block_buf)
            .map_err(|e| EncryptionError::EncryptionFailed(format!("{:?}", e)))?;
        buf[ct_start..ct_start + pt_len + TAG_SIZE].copy_from_slice(&block_buf);
    }

    // Write header at the front: [VERSION | initial_nonce | plaintext_size_be]
    buf[0] = VERSION;
    buf[1..1 + NONCE_SIZE].copy_from_slice(&initial_nonce);
    let pt_size_u32 = plaintext_size as u32;
    buf[1 + NONCE_SIZE..HEADER_SIZE].copy_from_slice(&pt_size_u32.to_be_bytes());

    Ok(required_capacity)
}

/// Decrypt all blocks in `buf`, returns the plaintext byte count.
pub fn decrypt_all_blocks(
    key: &[u8],
    buf: &mut [u8],
    info: &str,
    version: u8,
) -> Result<usize, StreamEncError> {
    use aes_gcm::{Aes256Gcm, KeyInit, Key};

    if buf.len() < HEADER_SIZE {
        return Err(StreamEncError::HeaderTooShort);
    }

    let (ver, initial_nonce, plaintext_size_u32) = parse_header(buf)?;
    if ver != version {
        return Err(StreamEncError::VersionMismatch {
            got: ver,
            want: version,
        });
    }
    let plaintext_size = plaintext_size_u32 as usize;

    let derived = encryption::Encryption::derive_key(key, info)?
        .ok_or(EncryptionError::NoKeyAvailable)?;
    let gcm_key = Key::<Aes256Gcm>::from_slice(&derived);
    let gcm = Aes256Gcm::new(gcm_key);

    let n = num_blocks(plaintext_size);
    let mut total_pt = 0usize;

    for i in 0..n {
        let written = decrypt_block(&gcm, buf, initial_nonce, version, i, plaintext_size)?;
        total_pt += written;
    }

    Ok(total_pt)
}

/// Decrypt a single block. Returns the number of plaintext bytes written.
pub fn decrypt_block<G>(
    gcm: &G,
    buf: &mut [u8],
    initial_nonce: [u8; NONCE_SIZE],
    _version: u8,
    block_index: usize,
    plaintext_size: usize,
) -> Result<usize, StreamEncError>
where
    G: AeadInPlace,
{
    let bn = block_nonce(initial_nonce, block_index);
    let aes_nonce = aes_gcm::Nonce::from_slice(&bn);

    let bds = block_data_size(plaintext_size, block_index);

    // Ciphertext (with tag) start in buf
    let ct_start = if block_index == 0 {
        HEADER_SIZE
    } else {
        HEADER_SIZE + (BLOCK0_DATA_SIZE + TAG_SIZE) + (block_index - 1) * (BLOCKN_DATA_SIZE + TAG_SIZE)
    };
    let ct_end = ct_start + bds + TAG_SIZE;

    if buf.len() < ct_end {
        return Err(StreamEncError::BufferTooSmall);
    }

    let mut block_buf = buf[ct_start..ct_end].to_vec();
    gcm.decrypt_in_place(aes_nonce, b"", &mut block_buf)
        .map_err(|e| EncryptionError::DecryptionFailed(format!("{:?}", e)))?;

    // Write plaintext back to start of its position
    let pt_start = if block_index == 0 {
        0
    } else {
        BLOCK0_DATA_SIZE + (block_index - 1) * BLOCKN_DATA_SIZE
    };
    buf[pt_start..pt_start + bds].copy_from_slice(&block_buf[..bds]);

    Ok(bds)
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    fn test_num_blocks() {
        assert_eq!(num_blocks(0), 1);
        assert_eq!(num_blocks(1), 1);
        assert_eq!(num_blocks(BLOCK0_DATA_SIZE), 1);
        assert_eq!(num_blocks(BLOCK0_DATA_SIZE + 1), 2);
        assert_eq!(num_blocks(BLOCK0_DATA_SIZE + BLOCKN_DATA_SIZE), 2);
        assert_eq!(num_blocks(BLOCK0_DATA_SIZE + BLOCKN_DATA_SIZE + 1), 3);
    }

    #[test]
    fn test_block_nonce_increments() {
        let nonce = [0u8; 12];
        let n0 = block_nonce(nonce, 0);
        let n1 = block_nonce(nonce, 1);
        assert_eq!(&n0[..8], &[0u8; 8]);
        assert_eq!(u32::from_be_bytes(n1[8..].try_into().unwrap()), 1);
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = b"01234567890123456789012345678901"; // 32 bytes
        let plaintext = b"Hello, streaming encryption world!";
        let mut buf = plaintext.to_vec();
        let required = encrypted_size(plaintext.len());
        buf.reserve(required);

        let written = encrypt(key, &mut buf, "test-info").unwrap();
        assert_eq!(written, required);
        assert_eq!(buf.len(), required);

        // Decrypt
        let (_, initial_nonce, _) = parse_header(&buf).unwrap();
        let pt_size = decrypt_all_blocks(key, &mut buf, "test-info", VERSION).unwrap();
        assert_eq!(pt_size, plaintext.len());
        assert_eq!(&buf[..pt_size], plaintext);
        let _ = initial_nonce;
    }
}
