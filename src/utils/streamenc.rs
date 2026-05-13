//! Block-based streaming encryption format (AES-GCM).
//!
//! Plaintext is split into fixed-size blocks and each block is independently encrypted
//! using AES-GCM with a nonce derived from a single random initial nonce incremented per block.
//! Block 0 carries a header: version (1 byte) + initial nonce (12 bytes) + plaintext size (4 bytes, big-endian).
//! The last block is zero-padded so the total ciphertext length is divisible by 16.

use crate::utils::encryption::{ceil_div, gcm_cipher, EncryptionError};
use aes_gcm::AeadInPlace;

#[cfg(not(target_arch = "wasm32"))]
use aes_gcm::aead::{rand_core::RngCore, OsRng};

const NONCE_SIZE: usize = 12;
const TAG_SIZE: usize = 16;
const EC_DATA_BLOCKS: usize = 16;
const VERSION_SIZE: usize = 1;
const PLAINTEXT_SIZE_BYTES: usize = 4;

/// Maximum ciphertext block size in bytes (32 KiB).
pub const MAX_BLOCK_SIZE: usize = 32 * 1024;

/// Size of the header stored in block 0: version + nonce + plaintext size.
pub const HEADER_SIZE: usize = VERSION_SIZE + NONCE_SIZE + PLAINTEXT_SIZE_BYTES; // 17

/// Current format version written into every ciphertext.
pub const VERSION: u8 = 1;

/// Maximum plaintext capacity of block 0 (header occupies part of it).
pub const BLOCK0_DATA_SIZE: usize = MAX_BLOCK_SIZE - HEADER_SIZE - TAG_SIZE;

/// Maximum plaintext capacity of blocks 1..N-1.
pub const BLOCK_N_DATA_SIZE: usize = MAX_BLOCK_SIZE - TAG_SIZE;

/// Errors produced by streamenc operations.
#[derive(Debug, thiserror::Error)]
pub enum StreamEncError {
    #[error("plaintext cannot be empty")]
    EmptyPlaintext,

    #[error("buffer too small for encryption")]
    BufferTooSmall,

    #[error("target size must be a multiple of 16 bytes")]
    TargetSizeNotAligned,

    #[error("target size too small")]
    TargetSizeTooSmall,

    #[error("ciphertext version mismatch: got {got}, want {want}")]
    VersionMismatch { got: u8, want: u8 },

    #[error("header too short")]
    HeaderTooShort,

    #[error("encryption error: {0}")]
    Encryption(#[from] EncryptionError),
}

/// Returns the number of ciphertext blocks needed for the given plaintext size.
pub fn num_blocks(plaintext_size: usize) -> usize {
    if plaintext_size == 0 {
        return 0;
    }
    if plaintext_size <= BLOCK0_DATA_SIZE {
        return 1;
    }
    1 + ceil_div(plaintext_size - BLOCK0_DATA_SIZE, BLOCK_N_DATA_SIZE)
}

/// Returns the actual (un-padded) plaintext byte count in the block at `block_index`.
pub fn block_data_size(plaintext_size: usize, block_index: usize) -> usize {
    let nb = num_blocks(plaintext_size);
    if block_index == 0 {
        if nb == 1 {
            return plaintext_size;
        }
        return BLOCK0_DATA_SIZE;
    }
    if block_index < nb - 1 {
        return BLOCK_N_DATA_SIZE;
    }
    // last block
    let filled = BLOCK0_DATA_SIZE + (block_index - 1) * BLOCK_N_DATA_SIZE;
    plaintext_size - filled
}

/// Returns the ciphertext bytes for the block at `block_index` within the full encrypted buffer.
pub fn encrypted_block(ciphertext: &[u8], block_index: usize) -> &[u8] {
    let start = block_index * MAX_BLOCK_SIZE;
    assert!(start < ciphertext.len(), "block_index out of bounds");
    let end = (start + MAX_BLOCK_SIZE).min(ciphertext.len());
    &ciphertext[start..end]
}

/// Returns the total overhead bytes for encrypting a plaintext of `plaintext_size` bytes.
pub fn overhead(plaintext_size: usize) -> usize {
    if plaintext_size == 0 {
        return 0;
    }
    let nb = num_blocks(plaintext_size);
    let last_idx = nb - 1;
    let last_data = block_data_size(plaintext_size, last_idx);
    let padding = last_block_padding(nb, last_data);
    let last_block_cipher = last_data + padding + TAG_SIZE;
    let total = if nb == 1 {
        HEADER_SIZE + last_block_cipher
    } else {
        (nb - 1) * MAX_BLOCK_SIZE + last_block_cipher
    };
    total - plaintext_size
}

/// Returns the maximum plaintext size whose ciphertext is exactly `target_size` bytes.
/// `target_size` must be a positive multiple of `EC_DATA_BLOCKS` (16).
pub fn max_plaintext_size_for_target(target_size: usize) -> Result<usize, StreamEncError> {
    let min_ct = min_ciphertext_size();
    if target_size < min_ct {
        return Err(StreamEncError::TargetSizeTooSmall);
    }
    if target_size % EC_DATA_BLOCKS != 0 {
        return Err(StreamEncError::TargetSizeNotAligned);
    }

    let nb = ceil_div(target_size, MAX_BLOCK_SIZE);
    if nb == 1 {
        return Ok((target_size - HEADER_SIZE - TAG_SIZE).min(BLOCK0_DATA_SIZE));
    }

    let last_block_cipher = target_size - (nb - 1) * MAX_BLOCK_SIZE;
    let last_data = (last_block_cipher - TAG_SIZE).min(BLOCK_N_DATA_SIZE);
    Ok(BLOCK0_DATA_SIZE + (nb - 2) * BLOCK_N_DATA_SIZE + last_data)
}

/// Parses the header from block 0. Returns `(version, initial_nonce, plaintext_size)`.
pub fn parse_header(data: &[u8]) -> Result<(u8, [u8; NONCE_SIZE], u32), StreamEncError> {
    if data.len() < HEADER_SIZE {
        return Err(StreamEncError::HeaderTooShort);
    }
    let ver = data[0];
    let mut nonce = [0u8; NONCE_SIZE];
    nonce.copy_from_slice(&data[VERSION_SIZE..VERSION_SIZE + NONCE_SIZE]);
    let size = u32::from_be_bytes(
        data[VERSION_SIZE + NONCE_SIZE..HEADER_SIZE]
            .try_into()
            .unwrap(),
    );
    Ok((ver, nonce, size))
}

/// Derives the nonce for a given block by incrementing the last 4 bytes of the initial nonce.
pub fn block_nonce(initial_nonce: [u8; NONCE_SIZE], block_index: usize) -> [u8; NONCE_SIZE] {
    let mut n = initial_nonce;
    let v = u32::from_be_bytes(n[8..12].try_into().unwrap());
    let incremented = v.wrapping_add(block_index as u32);
    n[8..12].copy_from_slice(&incremented.to_be_bytes());
    n
}

/// Encrypts `buf` in-place using block-based AES-GCM streaming encryption.
///
/// `buf` must have sufficient capacity: `len(buf) + overhead(len(buf))` bytes.
/// Encryption is performed right-to-left so each block's ciphertext never overwrites
/// unprocessed plaintext. The header (version + nonce + plaintext length) is packed
/// into the start of block 0. Returns the total number of bytes written.
pub fn encrypt(key: &[u8], buf: &mut Vec<u8>, info: &str) -> Result<usize, StreamEncError> {
    encrypt_with_rng(key, buf, info)
}

fn encrypt_with_rng(key: &[u8], buf: &mut Vec<u8>, info: &str) -> Result<usize, StreamEncError> {
    if buf.is_empty() {
        return Err(StreamEncError::EmptyPlaintext);
    }

    let plaintext_len = buf.len();
    let nb = num_blocks(plaintext_len);
    let last_idx = nb - 1;
    let last_data = block_data_size(plaintext_len, last_idx);
    let padding = last_block_padding(nb, last_data);
    let last_block_cipher = last_data + padding + TAG_SIZE;
    let total_size = if nb == 1 {
        HEADER_SIZE + last_block_cipher
    } else {
        (nb - 1) * MAX_BLOCK_SIZE + last_block_cipher
    };

    if buf.capacity() < total_size {
        return Err(StreamEncError::BufferTooSmall);
    }

    // Save plaintext before resizing
    let plaintext = buf.clone();

    // Extend buf to total_size (zeroed)
    buf.resize(total_size, 0);

    let gcm = gcm_cipher(key, info)?;

    // Generate random initial nonce
    let mut initial_nonce = [0u8; NONCE_SIZE];
    #[cfg(not(target_arch = "wasm32"))]
    OsRng.fill_bytes(&mut initial_nonce);
    #[cfg(target_arch = "wasm32")]
    {
        use web_sys::window;
        let crypto = window().unwrap().crypto().unwrap();
        crypto.get_random_values_with_u8_array(&mut initial_nonce).unwrap();
    }

    // Encrypt blocks right-to-left to avoid overwriting unprocessed plaintext.
    for i in (0..nb).rev() {
        let (capacity, plaintext_start) = if i == 0 {
            (BLOCK0_DATA_SIZE, 0)
        } else {
            (BLOCK_N_DATA_SIZE, BLOCK0_DATA_SIZE + (i - 1) * BLOCK_N_DATA_SIZE)
        };
        let plaintext_end = (plaintext_start + capacity).min(plaintext_len);
        let block_len = plaintext_end - plaintext_start;

        let padded_len = if i == last_idx {
            last_data + padding
        } else {
            capacity
        };

        let nonce = block_nonce(initial_nonce, i);
        let nonce_ref = aes_gcm::Nonce::from_slice(&nonce);

        let offset = if i == 0 { HEADER_SIZE } else { i * MAX_BLOCK_SIZE };

        // Copy plaintext into the output position
        let end = offset + padded_len + TAG_SIZE;
        // zero the region first (for padding)
        buf[offset..end].fill(0);
        buf[offset..offset + block_len].copy_from_slice(&plaintext[plaintext_start..plaintext_end]);

        // encrypt in place, get tag separately
        let tag = gcm.encrypt_in_place_detached(nonce_ref, b"", &mut buf[offset..offset + padded_len])
            .map_err(|e| EncryptionError::EncryptionFailed(format!("{:?}", e)))?;
        buf[offset + padded_len..end].copy_from_slice(tag.as_slice());
    }

    // Write header into start of block 0
    buf[0] = VERSION;
    buf[VERSION_SIZE..VERSION_SIZE + NONCE_SIZE].copy_from_slice(&initial_nonce);
    let plaintext_len_u32 = plaintext_len as u32;
    buf[VERSION_SIZE + NONCE_SIZE..HEADER_SIZE]
        .copy_from_slice(&plaintext_len_u32.to_be_bytes());

    Ok(total_size)
}

/// Decrypts all blocks in-place, compacting the plaintext to the start of `buf`.
/// Returns the total number of plaintext bytes.
pub fn decrypt_all_blocks(
    key: &[u8],
    buf: &mut Vec<u8>,
    info: &str,
    version: u8,
) -> Result<usize, StreamEncError> {
    let (ver, initial_nonce, plaintext_size32) = parse_header(buf)?;
    if ver != version {
        return Err(StreamEncError::VersionMismatch { got: ver, want: version });
    }
    let plaintext_size = plaintext_size32 as usize;
    let gcm = gcm_cipher(key, info)?;
    let nb = num_blocks(plaintext_size);

    // Decrypt each block and compact plaintext to start of buf
    let mut write_offset = 0usize;
    // We need a working copy since decryption modifies data in place
    let cipher_copy = buf.clone();

    for i in 0..nb {
        let block_start = i * MAX_BLOCK_SIZE;
        let block_end = (block_start + MAX_BLOCK_SIZE).min(cipher_copy.len());
        let block_buf = &cipher_copy[block_start..block_end];

        let (ct_start, nonce) = if i == 0 {
            if block_buf.len() < HEADER_SIZE {
                return Err(StreamEncError::HeaderTooShort);
            }
            (HEADER_SIZE, block_nonce(initial_nonce, 0))
        } else {
            (0, block_nonce(initial_nonce, i))
        };

        let nonce_ref = aes_gcm::Nonce::from_slice(&nonce);

        let mut ct = block_buf[ct_start..].to_vec();
        gcm.decrypt_in_place(nonce_ref, b"", &mut ct)
            .map_err(|e| EncryptionError::DecryptionFailed(format!("{:?}", e)))?;

        let actual = block_data_size(plaintext_size, i);
        buf[write_offset..write_offset + actual].copy_from_slice(&ct[..actual]);
        write_offset += actual;
    }

    buf.truncate(write_offset);
    Ok(write_offset)
}

fn last_block_padding(num_blocks_count: usize, last_data: usize) -> usize {
    if num_blocks_count == 1 {
        (EC_DATA_BLOCKS - (HEADER_SIZE + last_data) % EC_DATA_BLOCKS) % EC_DATA_BLOCKS
    } else {
        (EC_DATA_BLOCKS - last_data % EC_DATA_BLOCKS) % EC_DATA_BLOCKS
    }
}

fn min_ciphertext_size() -> usize {
    ((HEADER_SIZE + TAG_SIZE + 1 + EC_DATA_BLOCKS - 1) / EC_DATA_BLOCKS) * EC_DATA_BLOCKS
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = b"01234567890123456789012345678901"; // 32 bytes
        let info = "test-info";
        let sizes = [
            1usize,
            BLOCK0_DATA_SIZE,
            BLOCK0_DATA_SIZE + 1,
            3 * MAX_BLOCK_SIZE,
        ];
        for &size in &sizes {
            let plaintext: Vec<u8> = (0..size).map(|i| (i % 251) as u8).collect();
            let oh = overhead(size);
            let mut buf = Vec::with_capacity(size + oh);
            buf.extend_from_slice(&plaintext);
            let written = encrypt(key, &mut buf, info).expect("encrypt failed");
            assert_eq!(written, size + oh);
            assert_eq!(buf.len(), written);
            let n = decrypt_all_blocks(key, &mut buf, info, VERSION).expect("decrypt failed");
            assert_eq!(n, size);
            assert_eq!(&buf[..n], &plaintext[..], "roundtrip failed for size={size}");
        }
    }

    #[test]
    fn test_num_blocks() {
        assert_eq!(num_blocks(0), 0);
        assert_eq!(num_blocks(1), 1);
        assert_eq!(num_blocks(BLOCK0_DATA_SIZE), 1);
        assert_eq!(num_blocks(BLOCK0_DATA_SIZE + 1), 2);
    }

    #[test]
    fn test_overhead_divisible_by_16() {
        // total ciphertext must be divisible by 16 for all sizes
        for size in [1, 100, BLOCK0_DATA_SIZE, BLOCK0_DATA_SIZE + 1, 2 * MAX_BLOCK_SIZE] {
            let total = size + overhead(size);
            assert_eq!(total % 16, 0, "ciphertext not aligned for size={size}");
        }
    }
}
