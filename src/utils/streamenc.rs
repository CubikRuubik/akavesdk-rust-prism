use aes_gcm::aead::rand_core::RngCore;
use aes_gcm::{
    aead::{Aead, OsRng, Payload},
    Nonce,
};
use thiserror::Error;

use crate::utils::encryption::{ceil_div, gcm_cipher};

pub const NONCE_SIZE: usize = 12;
pub const TAG_SIZE: usize = 16;
pub const EC_DATA_BLOCKS: usize = 16;
pub const VERSION_SIZE: usize = 1;
pub const PLAINTEXT_SIZE_BYTES: usize = 4;
pub const MAX_BLOCK_SIZE: usize = 32 * 1024;
pub const HEADER_SIZE: usize = VERSION_SIZE + NONCE_SIZE + PLAINTEXT_SIZE_BYTES; // 17
pub const VERSION: u8 = 1;
pub const BLOCK0_DATA_SIZE: usize = MAX_BLOCK_SIZE - HEADER_SIZE - TAG_SIZE; // 32735
pub const BLOCKN_DATA_SIZE: usize = MAX_BLOCK_SIZE - TAG_SIZE; // 32752
pub const MIN_CIPHERTEXT_SIZE: usize =
    ((HEADER_SIZE + TAG_SIZE + 1 + EC_DATA_BLOCKS - 1) / EC_DATA_BLOCKS) * EC_DATA_BLOCKS;

#[derive(Error, Debug)]
pub enum StreamEncError {
    #[error("targetSize must be a multiple of 16 bytes")]
    ErrTargetSizeNotAligned,
    #[error("target size must be at least {MIN_CIPHERTEXT_SIZE} bytes")]
    ErrCiphertextSizeTooSmall,
    #[error("ciphertext version mismatch")]
    ErrVersionMismatch,
    #[error("buffer too small")]
    ErrBufferTooSmall,
    #[error("decryption failed: {0}")]
    ErrDecryptionFailed(String),
    #[error("header too short")]
    ErrHeaderTooShort,
    #[error("encryption error: {0}")]
    EncryptionError(String),
}

pub fn parse_header(data: &[u8]) -> Result<(u8, [u8; NONCE_SIZE], u32), StreamEncError> {
    if data.len() < HEADER_SIZE {
        return Err(StreamEncError::ErrHeaderTooShort);
    }
    let version = data[0];
    let mut nonce = [0u8; NONCE_SIZE];
    nonce.copy_from_slice(&data[1..1 + NONCE_SIZE]);
    let plaintext_size = u32::from_be_bytes(data[1 + NONCE_SIZE..HEADER_SIZE].try_into().unwrap());
    Ok((version, nonce, plaintext_size))
}

pub fn num_blocks(plaintext_size: usize) -> usize {
    if plaintext_size == 0 {
        return 0;
    }
    if plaintext_size <= BLOCK0_DATA_SIZE {
        return 1;
    }
    1 + ceil_div(plaintext_size - BLOCK0_DATA_SIZE, BLOCKN_DATA_SIZE)
}

pub fn block_data_size(plaintext_size: usize, block_index: usize) -> usize {
    let n = num_blocks(plaintext_size);
    if n == 1 {
        return plaintext_size;
    }
    if block_index == 0 {
        return BLOCK0_DATA_SIZE;
    }
    if block_index < n - 1 {
        return BLOCKN_DATA_SIZE;
    }
    // last block
    let used = BLOCK0_DATA_SIZE + (block_index - 1) * BLOCKN_DATA_SIZE;
    plaintext_size - used
}

pub fn encrypted_block(ciphertext: &[u8], block_index: usize) -> &[u8] {
    let start = block_index * MAX_BLOCK_SIZE;
    let end = (start + MAX_BLOCK_SIZE).min(ciphertext.len());
    &ciphertext[start..end]
}

pub fn overhead(plaintext_size: usize) -> usize {
    if plaintext_size == 0 {
        return 0;
    }
    let n = num_blocks(plaintext_size);
    let last_block_plaintext = block_data_size(plaintext_size, n - 1);
    let last_block_ciphertext_raw =
        last_block_plaintext + TAG_SIZE + if n == 1 { HEADER_SIZE } else { 0 };
    let last_block_padded = ceil_div(last_block_ciphertext_raw, EC_DATA_BLOCKS) * EC_DATA_BLOCKS;
    let last_block_overhead = last_block_padded - last_block_plaintext;

    let middle_overhead = if n > 2 { (n - 2) * TAG_SIZE } else { 0 };
    let first_overhead = if n > 1 { HEADER_SIZE + TAG_SIZE } else { 0 };

    first_overhead + middle_overhead + last_block_overhead
}

pub fn max_plaintext_size_for_target(target_size: usize) -> Result<usize, StreamEncError> {
    if target_size == 0 || target_size % EC_DATA_BLOCKS != 0 {
        return Err(StreamEncError::ErrTargetSizeNotAligned);
    }
    if target_size < MIN_CIPHERTEXT_SIZE {
        return Err(StreamEncError::ErrCiphertextSizeTooSmall);
    }
    let single_block_max_ciphertext =
        ceil_div(BLOCK0_DATA_SIZE + HEADER_SIZE + TAG_SIZE, EC_DATA_BLOCKS) * EC_DATA_BLOCKS;
    if target_size <= single_block_max_ciphertext {
        let p = target_size as isize - HEADER_SIZE as isize - TAG_SIZE as isize;
        if p <= 0 {
            return Err(StreamEncError::ErrCiphertextSizeTooSmall);
        }
        return Ok(p as usize);
    }
    let remaining = target_size - MAX_BLOCK_SIZE;
    if remaining == 0 {
        return Ok(BLOCK0_DATA_SIZE);
    }
    let full_blocks = remaining / MAX_BLOCK_SIZE;
    let last_block_ct = remaining % MAX_BLOCK_SIZE;
    let last_block_pt = if last_block_ct == 0 {
        BLOCKN_DATA_SIZE as isize
    } else {
        last_block_ct as isize - TAG_SIZE as isize
    };
    if last_block_pt <= 0 {
        return Err(StreamEncError::ErrCiphertextSizeTooSmall);
    }
    Ok(BLOCK0_DATA_SIZE + full_blocks * BLOCKN_DATA_SIZE + last_block_pt as usize)
}

pub fn block_nonce(initial_nonce: [u8; NONCE_SIZE], block_index: usize) -> [u8; NONCE_SIZE] {
    let mut n = initial_nonce;
    let counter = u32::from_be_bytes(n[NONCE_SIZE - 4..].try_into().unwrap());
    let new_counter = counter.wrapping_add(block_index as u32);
    n[NONCE_SIZE - 4..].copy_from_slice(&new_counter.to_be_bytes());
    n
}

/// Encrypt `buf` in-place using block-based AES-GCM streaming encryption.
///
/// Layout per block:
/// - Block 0: `[HEADER (17 bytes unencrypted)] + [AES-GCM ciphertext of block0_data]`
/// - Block N: `[AES-GCM ciphertext of blockN_data]`
/// - Last block is zero-padded to a multiple of `EC_DATA_BLOCKS` bytes.
///
/// Returns the total ciphertext length on success.
pub fn encrypt(key: &[u8], buf: &mut Vec<u8>, info: &str) -> Result<usize, StreamEncError> {
    let plaintext_size = buf.len();
    if plaintext_size == 0 {
        return Ok(0);
    }
    let n = num_blocks(plaintext_size);
    let total_overhead = overhead(plaintext_size);
    let total_size = plaintext_size + total_overhead;

    if buf.capacity() < total_size {
        return Err(StreamEncError::ErrBufferTooSmall);
    }

    let gcm = gcm_cipher(key, info).map_err(|e| StreamEncError::EncryptionError(e.to_string()))?;

    let mut initial_nonce = [0u8; NONCE_SIZE];
    OsRng.fill_bytes(&mut initial_nonce);

    let plaintext = buf.clone();
    buf.clear();

    for block_idx in 0..n {
        let data_size = block_data_size(plaintext_size, block_idx);
        let data_offset = if block_idx == 0 {
            0
        } else {
            BLOCK0_DATA_SIZE + (block_idx - 1) * BLOCKN_DATA_SIZE
        };
        let block_plain = &plaintext[data_offset..data_offset + data_size];

        let nonce_bytes = block_nonce(initial_nonce, block_idx);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let aad = info.as_bytes();

        if block_idx == 0 {
            // Write unencrypted header as prefix of block 0
            buf.push(VERSION);
            buf.extend_from_slice(&initial_nonce);
            buf.extend_from_slice(&(plaintext_size as u32).to_be_bytes());
        }

        // Encrypt only the block data (header is unencrypted prefix)
        let ciphertext = gcm
            .encrypt(
                nonce,
                Payload {
                    msg: block_plain,
                    aad,
                },
            )
            .map_err(|e| StreamEncError::ErrDecryptionFailed(e.to_string()))?;

        if block_idx == n - 1 {
            let current_len = buf.len();
            let ct_len = ciphertext.len();
            let total_so_far = current_len + ct_len;
            let padded_size = ceil_div(total_so_far, EC_DATA_BLOCKS) * EC_DATA_BLOCKS;
            buf.extend_from_slice(&ciphertext);
            buf.resize(padded_size, 0);
        } else {
            buf.extend_from_slice(&ciphertext);
        }
    }

    Ok(buf.len())
}

/// Decrypt all blocks from `buf` using block-based AES-GCM streaming encryption.
///
/// Block 0 has an unencrypted 17-byte header prefix containing:
/// version (1 byte) + initial_nonce (12 bytes) + plaintext_size (4 bytes, big-endian).
/// The rest of block 0 and all subsequent blocks are AES-GCM ciphertexts.
///
/// Returns the total plaintext length on success.
pub fn decrypt_all_blocks(
    key: &[u8],
    buf: &mut Vec<u8>,
    info: &str,
    _version: u8,
) -> Result<usize, StreamEncError> {
    if buf.is_empty() {
        return Ok(0);
    }

    // Read unencrypted header from the start of block 0
    let (ver, initial_nonce, plaintext_size_u32) = parse_header(buf)?;
    if ver != VERSION {
        return Err(StreamEncError::ErrVersionMismatch);
    }
    let plaintext_size = plaintext_size_u32 as usize;
    if plaintext_size == 0 {
        buf.clear();
        return Ok(0);
    }

    let n = num_blocks(plaintext_size);
    let gcm = gcm_cipher(key, info).map_err(|e| StreamEncError::EncryptionError(e.to_string()))?;
    let aad = info.as_bytes().to_vec();

    let ciphertext = buf.clone();
    buf.clear();
    buf.reserve(plaintext_size);

    for block_idx in 0..n {
        let block_start = block_idx * MAX_BLOCK_SIZE;
        let block_end = (block_start + MAX_BLOCK_SIZE).min(ciphertext.len());
        let block = &ciphertext[block_start..block_end];

        let nonce_bytes = block_nonce(initial_nonce, block_idx);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let expected_data_size = block_data_size(plaintext_size, block_idx);

        if block_idx == 0 {
            // Block 0: first HEADER_SIZE bytes are unencrypted header, rest is ciphertext
            // ciphertext = bytes[HEADER_SIZE .. HEADER_SIZE + expected_data_size + TAG_SIZE]
            let ct_start = HEADER_SIZE;
            let ct_len = expected_data_size + TAG_SIZE;
            if block.len() < ct_start + ct_len {
                return Err(StreamEncError::ErrBufferTooSmall);
            }
            let ct = &block[ct_start..ct_start + ct_len];
            let plaintext = gcm
                .decrypt(nonce, Payload { msg: ct, aad: &aad })
                .map_err(|e| StreamEncError::ErrDecryptionFailed(e.to_string()))?;
            buf.extend_from_slice(&plaintext);
        } else {
            let ct_len = expected_data_size + TAG_SIZE;
            if block.len() < ct_len {
                return Err(StreamEncError::ErrBufferTooSmall);
            }
            let ct = &block[..ct_len];
            let plaintext = gcm
                .decrypt(nonce, Payload { msg: ct, aad: &aad })
                .map_err(|e| StreamEncError::ErrDecryptionFailed(e.to_string()))?;
            buf.extend_from_slice(&plaintext);
        }
    }

    buf.truncate(plaintext_size);
    Ok(plaintext_size)
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    fn test_num_blocks_zero() {
        assert_eq!(num_blocks(0), 0);
    }

    #[test]
    fn test_num_blocks_single() {
        assert_eq!(num_blocks(1), 1);
        assert_eq!(num_blocks(BLOCK0_DATA_SIZE), 1);
    }

    #[test]
    fn test_num_blocks_multi() {
        assert_eq!(num_blocks(BLOCK0_DATA_SIZE + 1), 2);
        assert_eq!(num_blocks(BLOCK0_DATA_SIZE + BLOCKN_DATA_SIZE), 2);
        assert_eq!(num_blocks(BLOCK0_DATA_SIZE + BLOCKN_DATA_SIZE + 1), 3);
    }

    #[test]
    fn test_encrypt_decrypt_small() {
        let key = b"test_key_for_streamenc_testing__";
        let info = "test/bucket/file";
        let plaintext = b"Hello, World!";

        let mut buf = plaintext.to_vec();
        let required_capacity = plaintext.len() + overhead(plaintext.len());
        buf.reserve(required_capacity);

        let ct_len = encrypt(key, &mut buf, info).unwrap();
        assert!(ct_len > plaintext.len());
        assert_eq!(ct_len % EC_DATA_BLOCKS, 0);

        let mut ct_buf = buf.clone();
        let pt_len = decrypt_all_blocks(key, &mut ct_buf, info, VERSION).unwrap();
        assert_eq!(pt_len, plaintext.len());
        assert_eq!(&ct_buf[..pt_len], plaintext);
    }

    #[test]
    fn test_encrypt_decrypt_large() {
        let key = b"test_key_for_streamenc_testing__";
        let info = "test/large";
        // Use more than BLOCK0_DATA_SIZE to trigger multi-block
        let plaintext: Vec<u8> = (0..100_000).map(|i| (i % 251) as u8).collect();

        let mut buf = plaintext.clone();
        let required_capacity = plaintext.len() + overhead(plaintext.len());
        buf.reserve(required_capacity);

        let ct_len = encrypt(key, &mut buf, info).unwrap();
        assert!(ct_len > plaintext.len());

        let mut ct_buf = buf.clone();
        let pt_len = decrypt_all_blocks(key, &mut ct_buf, info, VERSION).unwrap();
        assert_eq!(pt_len, plaintext.len());
        assert_eq!(ct_buf, plaintext);
    }

    #[test]
    fn test_overhead_single_block() {
        let size = 100;
        let n = num_blocks(size);
        assert_eq!(n, 1);
        let oh = overhead(size);
        // single block: ceil((HEADER_SIZE + size + TAG_SIZE) / 16) * 16 - size
        let raw = HEADER_SIZE + size + TAG_SIZE;
        let padded = ceil_div(raw, EC_DATA_BLOCKS) * EC_DATA_BLOCKS;
        assert_eq!(oh, padded - size);
    }

    #[test]
    fn test_parse_header() {
        let mut data = vec![0u8; HEADER_SIZE];
        data[0] = VERSION;
        for i in 0..NONCE_SIZE {
            data[1 + i] = i as u8;
        }
        let size: u32 = 12345;
        data[1 + NONCE_SIZE..].copy_from_slice(&size.to_be_bytes());

        let (ver, nonce, sz) = parse_header(&data).unwrap();
        assert_eq!(ver, VERSION);
        assert_eq!(sz, 12345);
        for i in 0..NONCE_SIZE {
            assert_eq!(nonce[i], i as u8);
        }
    }
}
