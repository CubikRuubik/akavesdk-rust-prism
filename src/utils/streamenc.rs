// Copyright (C) 2026 Akave
// See LICENSE for copying information.

// This module is used by CHANGE-9 (Upload2/Download2), which was skipped.
#![allow(dead_code)]

//! Block-based streaming encryption format.
//!
//! Splits plaintext into fixed-size blocks (MaxBlockSize = 32 KiB each) and
//! encrypts each block independently using AES-GCM with a nonce derived from
//! a single random initial nonce incremented per block. Block 0 carries a
//! header containing the format version (1 byte), initial nonce (12 bytes),
//! and total plaintext size (4 bytes, big-endian uint32). The last block is
//! zero-padded so that total ciphertext length is divisible by 16.

use aes_gcm::{AeadInPlace, Aes256Gcm};
use thiserror::Error;

use crate::utils::encryption::{ceil_div, gcm_cipher as make_gcm_cipher, EncryptionError};

const NONCE_SIZE: usize = 12;
const TAG_SIZE: usize = 16;
const EC_DATA_BLOCKS: usize = 16;
const VERSION_SIZE: usize = 1;
const PLAINTEXT_SIZE_BYTES: usize = 4;

/// Maximum total ciphertext block size (including overhead).
pub const MAX_BLOCK_SIZE: usize = 32 * 1024; // 32 KiB

/// Size of the header stored in block 0: version (1) + nonce (12) + plaintext size (4).
pub const HEADER_SIZE: usize = VERSION_SIZE + NONCE_SIZE + PLAINTEXT_SIZE_BYTES;

/// Current format version written into every ciphertext.
pub const VERSION: u8 = 1;

/// Maximum plaintext capacity of block 0 (header occupies part of it).
pub const BLOCK0_DATA_SIZE: usize = MAX_BLOCK_SIZE - HEADER_SIZE - TAG_SIZE;

/// Maximum plaintext capacity of blocks 1..N-1.
pub const BLOCKN_DATA_SIZE: usize = MAX_BLOCK_SIZE - TAG_SIZE;

/// Smallest valid ciphertext: one aligned block with at least 1 plaintext byte.
pub const MIN_CIPHERTEXT_SIZE: usize =
    ((HEADER_SIZE + TAG_SIZE + 1 + EC_DATA_BLOCKS - 1) / EC_DATA_BLOCKS) * EC_DATA_BLOCKS;

/// Errors produced by the `streamenc` module.
#[derive(Error, Debug)]
pub enum StreamEncError {
    #[error("targetSize must be a multiple of 16 bytes")]
    TargetSizeNotAligned,

    #[error("target size must be at least {MIN_CIPHERTEXT_SIZE} bytes")]
    TargetSizeTooSmall,

    #[error("ciphertext version mismatch: got {got}, want {want}")]
    VersionMismatch { got: u8, want: u8 },

    #[error("header too short")]
    HeaderTooShort,

    #[error("plaintext cannot be empty")]
    EmptyPlaintext,

    #[error("buffer too small for encryption")]
    BufferTooSmall,

    #[error("encryption error: {0}")]
    Encryption(#[from] EncryptionError),

    #[error("decryption error: {0}")]
    Decryption(String),
}

/// Parses the header from block 0.
/// Returns `(version, initial_nonce, plaintext_size)`.
pub fn parse_header(data: &[u8]) -> Result<(u8, [u8; NONCE_SIZE], u32), StreamEncError> {
    if data.len() < HEADER_SIZE {
        return Err(StreamEncError::HeaderTooShort);
    }
    let ver = data[0];
    let mut initial_nonce = [0u8; NONCE_SIZE];
    initial_nonce.copy_from_slice(&data[VERSION_SIZE..VERSION_SIZE + NONCE_SIZE]);
    let plaintext_size = u32::from_be_bytes(
        data[VERSION_SIZE + NONCE_SIZE..HEADER_SIZE]
            .try_into()
            .map_err(|_| StreamEncError::HeaderTooShort)?,
    );
    Ok((ver, initial_nonce, plaintext_size))
}

/// Returns the number of ciphertext blocks for a given plaintext size.
pub fn num_blocks(plaintext_size: usize) -> usize {
    if plaintext_size == 0 {
        return 0;
    }
    if plaintext_size <= BLOCK0_DATA_SIZE {
        return 1;
    }
    1 + ceil_div(plaintext_size - BLOCK0_DATA_SIZE, BLOCKN_DATA_SIZE)
}

/// Returns the actual (un-padded) count of plaintext bytes in the block at `block_index`.
pub fn block_data_size(plaintext_size: usize, block_index: usize) -> usize {
    let n = num_blocks(plaintext_size);
    if block_index == 0 {
        if n == 1 {
            return plaintext_size;
        }
        return BLOCK0_DATA_SIZE;
    }
    if block_index < n - 1 {
        return BLOCKN_DATA_SIZE;
    }
    // last block
    let filled = BLOCK0_DATA_SIZE + (block_index - 1) * BLOCKN_DATA_SIZE;
    plaintext_size - filled
}

/// Returns the ciphertext bytes for block at `block_index` within the full encrypted buffer.
pub fn encrypted_block(ciphertext: &[u8], block_index: usize) -> &[u8] {
    let start = block_index * MAX_BLOCK_SIZE;
    assert!(
        block_index >= 0 && start < ciphertext.len(),
        "blockIndex out of bounds"
    );
    let end = (start + MAX_BLOCK_SIZE).min(ciphertext.len());
    &ciphertext[start..end]
}

/// Returns the total overhead bytes added to plaintext of the given size.
pub fn overhead(plaintext_size: usize) -> usize {
    if plaintext_size == 0 {
        return 0;
    }
    let n = num_blocks(plaintext_size);
    let last_data = block_data_size(plaintext_size, n - 1);
    let padding = last_block_padding(n, last_data);
    let last_block_cipher_size = last_data + padding + TAG_SIZE;
    let total_size = if n == 1 {
        HEADER_SIZE + last_block_cipher_size
    } else {
        (n - 1) * MAX_BLOCK_SIZE + last_block_cipher_size
    };
    total_size - plaintext_size
}

/// Returns the maximum plaintext size whose ciphertext is exactly `target_size` bytes.
/// `target_size` must be a positive multiple of `EC_DATA_BLOCKS` (16).
pub fn max_plaintext_size_for_target(target_size: usize) -> Result<usize, StreamEncError> {
    if target_size < MIN_CIPHERTEXT_SIZE {
        return Err(StreamEncError::TargetSizeTooSmall);
    }
    if target_size % EC_DATA_BLOCKS != 0 {
        return Err(StreamEncError::TargetSizeNotAligned);
    }
    let n = ceil_div(target_size, MAX_BLOCK_SIZE);
    if n == 1 {
        // Single block: totalSize = HEADER_SIZE + lastData + lastPad + TAG_SIZE
        return Ok((target_size - HEADER_SIZE - TAG_SIZE).min(BLOCK0_DATA_SIZE));
    }
    // Multi-block: last block contributes target_size - (n-1)*MAX_BLOCK_SIZE
    let last_block_cipher_size = target_size - (n - 1) * MAX_BLOCK_SIZE;
    let last_data = (last_block_cipher_size - TAG_SIZE).min(BLOCKN_DATA_SIZE);
    Ok(BLOCK0_DATA_SIZE + (n - 2) * BLOCKN_DATA_SIZE + last_data)
}

/// Derives the per-block nonce by incrementing the last 4 bytes of `initial_nonce` by `block_index`.
pub fn block_nonce(mut initial_nonce: [u8; NONCE_SIZE], block_index: usize) -> [u8; NONCE_SIZE] {
    let v = u32::from_be_bytes(initial_nonce[8..12].try_into().unwrap());
    let new_v = v.wrapping_add(block_index as u32);
    initial_nonce[8..12].copy_from_slice(&new_v.to_be_bytes());
    initial_nonce
}

/// Encrypts `buf` in-place using block-based streaming AES-GCM.
///
/// `buf` must have sufficient capacity to hold the full ciphertext (use `overhead()` to size
/// the buffer). Returns the number of bytes written.
///
/// Matches Go's `streamenc.Encrypt`.
#[cfg(not(target_arch = "wasm32"))]
pub fn encrypt(key: &[u8], buf: &mut Vec<u8>, info: &str) -> Result<usize, StreamEncError> {
    use aes_gcm::aead::rand_core::RngCore;
    use aes_gcm::aead::OsRng;

    if buf.is_empty() {
        return Err(StreamEncError::EmptyPlaintext);
    }

    let plaintext_len = buf.len();
    let n = num_blocks(plaintext_len);
    let last_data = block_data_size(plaintext_len, n - 1);
    let padding = last_block_padding(n, last_data);
    let last_block_cipher_size = last_data + padding + TAG_SIZE;
    let total_size = if n == 1 {
        HEADER_SIZE + last_block_cipher_size
    } else {
        (n - 1) * MAX_BLOCK_SIZE + last_block_cipher_size
    };

    if buf.capacity() < total_size {
        return Err(StreamEncError::BufferTooSmall);
    }

    let gcm = make_gcm_cipher(key, info)?;

    let mut initial_nonce = [0u8; NONCE_SIZE];
    OsRng.fill_bytes(&mut initial_nonce);

    // Resize buffer to full ciphertext size (fill with zeros for padding)
    buf.resize(total_size, 0u8);

    // Encrypt right-to-left so each block's ciphertext never overwrites unprocessed plaintext.
    let mut tmp = vec![0u8; BLOCKN_DATA_SIZE];
    for i in (0..n).rev() {
        let (capacity, plaintext_start) = if i == 0 {
            (BLOCK0_DATA_SIZE, 0)
        } else {
            (
                BLOCKN_DATA_SIZE,
                BLOCK0_DATA_SIZE + (i - 1) * BLOCKN_DATA_SIZE,
            )
        };
        let plaintext_end = (plaintext_start + capacity).min(plaintext_len);
        let block_len = plaintext_end - plaintext_start;

        let padded_len = if i == n - 1 {
            last_data + padding
        } else {
            capacity
        };

        tmp[..block_len].copy_from_slice(&buf[plaintext_start..plaintext_end]);
        if block_len < padded_len {
            // Zero-pad the last block
            for b in tmp[block_len..padded_len].iter_mut() {
                *b = 0;
            }
        }

        let nonce = block_nonce(initial_nonce, i);
        let nonce_arr = aes_gcm::Nonce::from_slice(&nonce);

        let offset = if i == 0 {
            HEADER_SIZE
        } else {
            i * MAX_BLOCK_SIZE
        };

        let plaintext_slice = &tmp[..padded_len];
        let ciphertext_capacity = padded_len + TAG_SIZE;

        // Encrypt into a temporary buffer, then copy to output
        let mut ct_buf = plaintext_slice.to_vec();
        gcm.encrypt_in_place(nonce_arr, b"", &mut ct_buf)
            .map_err(|e| {
                StreamEncError::Encryption(EncryptionError::EncryptionFailed(e.to_string()))
            })?;
        buf[offset..offset + ciphertext_capacity].copy_from_slice(&ct_buf);
    }

    // Write header into the start of block 0
    buf[0] = VERSION;
    buf[VERSION_SIZE..VERSION_SIZE + NONCE_SIZE].copy_from_slice(&initial_nonce);
    let plaintext_u32 = plaintext_len as u32;
    buf[VERSION_SIZE + NONCE_SIZE..HEADER_SIZE].copy_from_slice(&plaintext_u32.to_be_bytes());

    Ok(total_size)
}

/// Decrypts a single ciphertext block in-place.
///
/// For block 0, `buf` must include the header prefix. Returns the actual plaintext byte count.
/// Matches Go's `streamenc.DecryptBlock`.
pub fn decrypt_block(
    gcm: &Aes256Gcm,
    buf: &mut [u8],
    initial_nonce: [u8; NONCE_SIZE],
    version: u8,
    block_index: usize,
    plaintext_size: usize,
) -> Result<usize, StreamEncError> {
    let ciphertext_start = if block_index == 0 {
        if buf.len() < HEADER_SIZE {
            return Err(StreamEncError::HeaderTooShort);
        }
        if buf[0] != version {
            return Err(StreamEncError::VersionMismatch {
                got: buf[0],
                want: version,
            });
        }
        HEADER_SIZE
    } else {
        0
    };

    let nonce = block_nonce(initial_nonce, block_index);
    let nonce_arr = aes_gcm::Nonce::from_slice(&nonce);

    let ciphertext = &mut buf[ciphertext_start..];
    let mut ct_buf = ciphertext.to_vec();
    gcm.decrypt_in_place(nonce_arr, b"", &mut ct_buf)
        .map_err(|e| StreamEncError::Decryption(e.to_string()))?;

    let actual = block_data_size(plaintext_size, block_index);

    if block_index == 0 {
        buf[..actual].copy_from_slice(&ct_buf[..actual]);
    } else {
        buf[..actual].copy_from_slice(&ct_buf[..actual]);
    }

    Ok(actual)
}

/// Decrypts all blocks in-place, compacting the plaintext to the start of `buf`.
///
/// Reads the plaintext size from the block-0 header. Returns the number of plaintext bytes.
/// Matches Go's `streamenc.DecryptAllBlocks`.
pub fn decrypt_all_blocks(
    key: &[u8],
    buf: &mut Vec<u8>,
    info: &str,
    version: u8,
) -> Result<usize, StreamEncError> {
    let (ver, initial_nonce, plaintext_size32) = parse_header(buf)?;
    if ver != version {
        return Err(StreamEncError::VersionMismatch {
            got: ver,
            want: version,
        });
    }
    let plaintext_size = plaintext_size32 as usize;

    let gcm = make_gcm_cipher(key, info)?;

    let n = num_blocks(plaintext_size);
    let mut offset = 0usize;
    let buf_len = buf.len();

    for i in 0..n {
        let start = i * MAX_BLOCK_SIZE;
        let end = (start + MAX_BLOCK_SIZE).min(buf_len);
        // Decrypt the block in-place using a temporary copy
        let mut block = buf[start..end].to_vec();
        let actual = decrypt_block(&gcm, &mut block, initial_nonce, version, i, plaintext_size)?;
        buf[offset..offset + actual].copy_from_slice(&block[..actual]);
        offset += actual;
    }

    // Zero out remaining bytes after the plaintext
    for b in buf[offset..].iter_mut() {
        *b = 0;
    }

    Ok(offset)
}

/// Computes zero-padding bytes appended to the last block.
fn last_block_padding(num_blocks: usize, last_data: usize) -> usize {
    if num_blocks == 1 {
        (EC_DATA_BLOCKS - (HEADER_SIZE + last_data) % EC_DATA_BLOCKS) % EC_DATA_BLOCKS
    } else {
        (EC_DATA_BLOCKS - last_data % EC_DATA_BLOCKS) % EC_DATA_BLOCKS
    }
}
