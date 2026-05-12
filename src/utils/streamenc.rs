// This module is used by CHANGE-9 (Upload2/Download2), which was skipped.
#![allow(dead_code)]

use aes_gcm::{AeadInPlace, Aes256Gcm, Nonce};

use crate::utils::encryption::{gcm_cipher, EncryptionError};

const NONCE_SIZE: usize = 12;
const TAG_SIZE: usize = 16;
const EC_DATA_BLOCKS: usize = 16;
const VERSION_SIZE: usize = 1;
const PLAINTEXT_SIZE_BYTES: usize = 4;

pub const MAX_BLOCK_SIZE: usize = 32 * 1024;
pub const HEADER_SIZE: usize = VERSION_SIZE + NONCE_SIZE + PLAINTEXT_SIZE_BYTES; // = 17
pub const VERSION: u8 = 1;
pub const BLOCK0_DATA_SIZE: usize = MAX_BLOCK_SIZE - HEADER_SIZE - TAG_SIZE; // = 32735
pub const BLOCKN_DATA_SIZE: usize = MAX_BLOCK_SIZE - TAG_SIZE; // = 32752
pub const MIN_CIPHERTEXT_SIZE: usize =
    ((HEADER_SIZE + TAG_SIZE + 1 + EC_DATA_BLOCKS - 1) / EC_DATA_BLOCKS) * EC_DATA_BLOCKS;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum StreamEncError {
    #[error("targetSize must be a multiple of 16 bytes")]
    TargetSizeNotAligned,

    #[error("target size must be at least {MIN_CIPHERTEXT_SIZE} bytes")]
    CiphertextSizeTooSmall,

    #[error("ciphertext version mismatch")]
    VersionMismatch,

    #[error("encryption error: {0}")]
    Encryption(#[from] EncryptionError),

    #[error("buffer too small")]
    BufferTooSmall,
}

pub fn parse_header(data: &[u8]) -> Result<(u8, [u8; NONCE_SIZE], u32), StreamEncError> {
    if data.len() < HEADER_SIZE {
        return Err(StreamEncError::BufferTooSmall);
    }
    let version = data[0];
    let mut nonce = [0u8; NONCE_SIZE];
    nonce.copy_from_slice(&data[1..1 + NONCE_SIZE]);
    let plaintext_size = u32::from_be_bytes(
        data[1 + NONCE_SIZE..HEADER_SIZE]
            .try_into()
            .map_err(|_| StreamEncError::BufferTooSmall)?,
    );
    Ok((version, nonce, plaintext_size))
}

pub fn num_blocks(plaintext_size: usize) -> usize {
    if plaintext_size == 0 {
        return 0;
    }
    if plaintext_size <= BLOCK0_DATA_SIZE {
        return 1;
    }
    1 + (plaintext_size.saturating_sub(BLOCK0_DATA_SIZE) + BLOCKN_DATA_SIZE - 1) / BLOCKN_DATA_SIZE
}

pub fn block_data_size(plaintext_size: usize, block_index: usize) -> usize {
    if block_index == 0 {
        return plaintext_size.min(BLOCK0_DATA_SIZE);
    }
    let already = BLOCK0_DATA_SIZE + (block_index - 1) * BLOCKN_DATA_SIZE;
    if already >= plaintext_size {
        return 0;
    }
    (plaintext_size - already).min(BLOCKN_DATA_SIZE)
}

pub fn encrypted_block(ciphertext: &[u8], block_index: usize) -> &[u8] {
    let start = block_index * MAX_BLOCK_SIZE;
    if start >= ciphertext.len() {
        return &[];
    }
    let end = (start + MAX_BLOCK_SIZE).min(ciphertext.len());
    &ciphertext[start..end]
}

fn last_block_padding(nb: usize, last_data: usize) -> usize {
    if nb == 1 {
        (EC_DATA_BLOCKS - (HEADER_SIZE + last_data + TAG_SIZE) % EC_DATA_BLOCKS) % EC_DATA_BLOCKS
    } else {
        (EC_DATA_BLOCKS - (last_data + TAG_SIZE) % EC_DATA_BLOCKS) % EC_DATA_BLOCKS
    }
}

pub fn overhead(plaintext_size: usize) -> usize {
    if plaintext_size == 0 {
        return 0;
    }
    let nb = num_blocks(plaintext_size);
    let last_data = block_data_size(plaintext_size, nb - 1);
    let padding = last_block_padding(nb, last_data);
    HEADER_SIZE + nb * TAG_SIZE + padding
}

pub fn max_plaintext_size_for_target(target_size: usize) -> Result<usize, StreamEncError> {
    if target_size % EC_DATA_BLOCKS != 0 {
        return Err(StreamEncError::TargetSizeNotAligned);
    }
    if target_size < MIN_CIPHERTEXT_SIZE {
        return Err(StreamEncError::CiphertextSizeTooSmall);
    }

    let mut lo = 0usize;
    let mut hi = target_size;
    while lo < hi {
        let mid = lo + (hi - lo + 1) / 2;
        if mid + overhead(mid) <= target_size {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    Ok(lo)
}

pub fn block_nonce(initial_nonce: [u8; NONCE_SIZE], block_index: usize) -> [u8; NONCE_SIZE] {
    let mut nonce = initial_nonce;
    let last4 = u32::from_be_bytes(nonce[8..12].try_into().unwrap());
    let new_last4 = last4.wrapping_add(block_index as u32);
    nonce[8..12].copy_from_slice(&new_last4.to_be_bytes());
    nonce
}

/// Encrypts `buf` in-place. `buf` should contain the plaintext; after the call it will
/// contain the full ciphertext (buf is extended to fit).
/// Returns the total ciphertext size.
pub fn encrypt(key: &[u8], buf: &mut Vec<u8>, info: &str) -> Result<usize, StreamEncError> {
    let plaintext_size = buf.len();
    let ciphertext_size = plaintext_size + overhead(plaintext_size);

    buf.resize(ciphertext_size, 0);

    let gcm = gcm_cipher(key, info)?;

    #[cfg(not(target_arch = "wasm32"))]
    let initial_nonce: [u8; NONCE_SIZE] = {
        use aes_gcm::aead::rand_core::RngCore;
        use aes_gcm::aead::OsRng;
        let mut n = [0u8; NONCE_SIZE];
        OsRng.fill_bytes(&mut n);
        n
    };
    #[cfg(target_arch = "wasm32")]
    let initial_nonce: [u8; NONCE_SIZE] = {
        use web_sys::window;
        let mut n = [0u8; NONCE_SIZE];
        let crypto = window().unwrap().crypto().unwrap();
        crypto.get_random_values_with_u8_array(&mut n).unwrap();
        n
    };

    let nb = num_blocks(plaintext_size);

    for block_idx in (0..nb).rev() {
        let data_size = block_data_size(plaintext_size, block_idx);

        let (src_start, dst_start) = if block_idx == 0 {
            (0usize, HEADER_SIZE)
        } else {
            (
                BLOCK0_DATA_SIZE + (block_idx - 1) * BLOCKN_DATA_SIZE,
                block_idx * MAX_BLOCK_SIZE,
            )
        };

        buf.copy_within(src_start..src_start + data_size, dst_start);

        let nonce_bytes = block_nonce(initial_nonce, block_idx);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let enc_end = dst_start + data_size + TAG_SIZE;

        let mut block_buf = buf[dst_start..dst_start + data_size].to_vec();
        gcm.encrypt_in_place(nonce, b"", &mut block_buf)
            .map_err(|e| EncryptionError::EncryptionFailed(format!("{e:?}")))?;
        buf[dst_start..enc_end].copy_from_slice(&block_buf);
    }

    buf[0] = VERSION;
    buf[1..1 + NONCE_SIZE].copy_from_slice(&initial_nonce);
    buf[1 + NONCE_SIZE..HEADER_SIZE].copy_from_slice(&(plaintext_size as u32).to_be_bytes());

    Ok(ciphertext_size)
}

pub fn decrypt_block(
    gcm: &Aes256Gcm,
    buf: &mut [u8],
    initial_nonce: [u8; NONCE_SIZE],
    version: u8,
    block_index: usize,
    plaintext_size: usize,
) -> Result<usize, StreamEncError> {
    let data_size = block_data_size(plaintext_size, block_index);
    let encrypted_size = data_size + TAG_SIZE;

    let read_start = if block_index == 0 {
        if buf[0] != version {
            return Err(StreamEncError::VersionMismatch);
        }
        HEADER_SIZE
    } else {
        block_index * MAX_BLOCK_SIZE
    };

    if buf.len() < read_start + encrypted_size {
        return Err(StreamEncError::BufferTooSmall);
    }

    let nonce_bytes = block_nonce(initial_nonce, block_index);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let mut block_buf = buf[read_start..read_start + encrypted_size].to_vec();
    gcm.decrypt_in_place(nonce, b"", &mut block_buf)
        .map_err(|e| EncryptionError::DecryptionFailed(format!("{e:?}")))?;
    buf[read_start..read_start + data_size].copy_from_slice(&block_buf);

    Ok(data_size)
}

pub fn decrypt_all_blocks(
    key: &[u8],
    buf: &mut [u8],
    info: &str,
    version: u8,
) -> Result<usize, StreamEncError> {
    let (hdr_version, initial_nonce, plaintext_size_u32) = parse_header(buf)?;
    if hdr_version != version {
        return Err(StreamEncError::VersionMismatch);
    }

    let plaintext_size = plaintext_size_u32 as usize;
    let gcm = gcm_cipher(key, info)?;
    let nb = num_blocks(plaintext_size);

    let mut output = vec![0u8; plaintext_size];
    let mut written = 0usize;
    for block_idx in 0..nb {
        let data_size = block_data_size(plaintext_size, block_idx);
        let read_start = if block_idx == 0 {
            HEADER_SIZE
        } else {
            block_idx * MAX_BLOCK_SIZE
        };
        let encrypted_size = data_size + TAG_SIZE;

        let nonce_bytes = block_nonce(initial_nonce, block_idx);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let mut block_buf = buf[read_start..read_start + encrypted_size].to_vec();
        gcm.decrypt_in_place(nonce, b"", &mut block_buf)
            .map_err(|e| EncryptionError::DecryptionFailed(format!("{e:?}")))?;
        output[written..written + data_size].copy_from_slice(&block_buf);
        written += data_size;
    }

    buf[..plaintext_size].copy_from_slice(&output);

    Ok(plaintext_size)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert_eq!(HEADER_SIZE, 17);
        assert_eq!(BLOCK0_DATA_SIZE, MAX_BLOCK_SIZE - HEADER_SIZE - TAG_SIZE);
        assert_eq!(BLOCKN_DATA_SIZE, MAX_BLOCK_SIZE - TAG_SIZE);
    }

    #[test]
    fn test_num_blocks() {
        assert_eq!(num_blocks(0), 1);
        assert_eq!(num_blocks(1), 1);
        assert_eq!(num_blocks(BLOCK0_DATA_SIZE), 1);
        assert_eq!(num_blocks(BLOCK0_DATA_SIZE + 1), 2);
    }

    #[test]
    fn test_overhead() {
        let ps = 1000;
        let oh = overhead(ps);
        assert!(oh > HEADER_SIZE + TAG_SIZE);
        assert_eq!((ps + oh) % EC_DATA_BLOCKS, 0);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = b"testkey_32bytes_testkey_32bytes__";
        let plaintext = b"Hello, streaming encryption world!";
        let info = "test/info";

        let mut buf = plaintext.to_vec();
        let ct_size = encrypt(key, &mut buf, info).unwrap();
        assert_eq!(ct_size, buf.len());

        let result = decrypt_all_blocks(key, &mut buf, info, VERSION).unwrap();
        assert_eq!(result, plaintext.len());
        assert_eq!(&buf[..result], plaintext);
    }
}
