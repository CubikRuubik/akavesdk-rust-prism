// Copyright (C) 2026 Akave
// See LICENSE for copying information.

//! Block-based streaming encryption format (analogous to Go's `private/encryption/streamenc` package).
//!
//! Plaintext is split into fixed-size blocks, each encrypted independently with AES-GCM.
//! The nonce for each block is derived from a single random initial nonce by incrementing
//! the last 4 bytes by the block index.  Block 0 carries a header:
//!   [ version (1 B) | initial_nonce (12 B) | plaintext_size (4 B big-endian) ]
//! The last block is zero-padded to a multiple of [`EC_DATA_BLOCKS`] bytes so that the
//! total ciphertext length is always divisible by 16 (required for downstream erasure coding).

#[cfg(not(target_arch = "wasm32"))]
use aes_gcm::aead::rand_core::RngCore;
#[cfg(not(target_arch = "wasm32"))]
use aes_gcm::aead::OsRng;
use aes_gcm::{
    aead::{AeadInPlace, KeyInit},
    Aes256Gcm, Key,
};
use thiserror::Error;

use crate::utils::encryption::{ceil_div, EncryptionError, KEY_LEN};

// Number of erasure-coding data blocks; used for ciphertext alignment.
const EC_DATA_BLOCKS: usize = 16;

const NONCE_SIZE: usize = 12;
const TAG_SIZE: usize = 16;
const VERSION_SIZE: usize = 1;
const PLAINTEXT_SIZE_BYTES: usize = 4;

/// Size of the per-block header stored in block 0 (version + nonce + plaintext length).
pub const HEADER_SIZE: usize = VERSION_SIZE + NONCE_SIZE + PLAINTEXT_SIZE_BYTES; // 17

/// Maximum total ciphertext block size (including overhead).
pub const MAX_BLOCK_SIZE: usize = 32 * 1024; // 32 KiB

/// Current format version written into every ciphertext.
pub const VERSION: u8 = 1;

/// Maximum plaintext capacity of block 0 (header + tag occupy part of the block).
pub const BLOCK0_DATA_SIZE: usize = MAX_BLOCK_SIZE - HEADER_SIZE - TAG_SIZE; // 32735

/// Maximum plaintext capacity of blocks 1..N-1.
pub const BLOCKN_DATA_SIZE: usize = MAX_BLOCK_SIZE - TAG_SIZE; // 32752

/// Minimum valid ciphertext size: one aligned block that fits at least 1 byte of plaintext.
pub const MIN_CIPHERTEXT_SIZE: usize =
    ((HEADER_SIZE + TAG_SIZE + 1 + EC_DATA_BLOCKS - 1) / EC_DATA_BLOCKS) * EC_DATA_BLOCKS;

/// Errors returned by streamenc operations.
#[derive(Error, Debug)]
pub enum StreamEncError {
    #[error("plaintext cannot be empty")]
    EmptyPlaintext,

    #[error("buffer too small for encryption")]
    BufferTooSmall,

    #[error("ciphertext version mismatch: got {got}, want {want}")]
    VersionMismatch { got: u8, want: u8 },

    #[error("header too short")]
    HeaderTooShort,

    #[error("block {index} too short to contain header")]
    Block0TooShort { index: usize },

    #[error("target size must be a multiple of {EC_DATA_BLOCKS} bytes, got {got}")]
    TargetNotAligned { got: usize },

    #[error("target size must be at least {MIN_CIPHERTEXT_SIZE} bytes, got {got}")]
    TargetTooSmall { got: usize },

    #[error("key derivation error: {0}")]
    KeyError(#[from] EncryptionError),

    #[error("AES-GCM error")]
    AesGcm,
}

/// Returns the number of ciphertext blocks required for `plaintext_size` bytes.
pub fn num_blocks(plaintext_size: usize) -> usize {
    if plaintext_size == 0 {
        return 0;
    }
    if plaintext_size <= BLOCK0_DATA_SIZE {
        return 1;
    }
    1 + ceil_div(plaintext_size - BLOCK0_DATA_SIZE, BLOCKN_DATA_SIZE)
}

/// Returns the actual (un-padded) byte count of plaintext in the block at `block_index`.
pub fn block_data_size(plaintext_size: usize, block_index: usize) -> usize {
    let n = num_blocks(plaintext_size);
    if block_index == 0 {
        return if n == 1 {
            plaintext_size
        } else {
            BLOCK0_DATA_SIZE
        };
    }
    if block_index < n - 1 {
        return BLOCKN_DATA_SIZE;
    }
    // Last block
    let filled = BLOCK0_DATA_SIZE + (block_index - 1) * BLOCKN_DATA_SIZE;
    plaintext_size - filled
}

/// Returns the slice of ciphertext bytes for the block at `block_index`.
/// `ciphertext` is the full encrypted buffer returned by [`encrypt`].
pub fn encrypted_block(ciphertext: &[u8], block_index: usize) -> &[u8] {
    let start = block_index * MAX_BLOCK_SIZE;
    assert!(block_index < ciphertext.len(), "blockIndex out of bounds");
    let end = (start + MAX_BLOCK_SIZE).min(ciphertext.len());
    &ciphertext[start..end]
}

/// Returns the total encryption overhead (in bytes) for `plaintext_size` bytes of plaintext.
/// Total ciphertext size = `plaintext_size` + `overhead(plaintext_size)`.
pub fn overhead(plaintext_size: usize) -> usize {
    if plaintext_size == 0 {
        return 0;
    }
    let n = num_blocks(plaintext_size);
    let last_data = block_data_size(plaintext_size, n - 1);
    let padding = last_block_padding(n, last_data);
    let last_cipher = last_data + padding + TAG_SIZE;
    let total = if n == 1 {
        HEADER_SIZE + last_cipher
    } else {
        (n - 1) * MAX_BLOCK_SIZE + last_cipher
    };
    total - plaintext_size
}

/// Returns the maximum plaintext size whose ciphertext is exactly `target_size` bytes.
/// `target_size` must be a positive multiple of [`EC_DATA_BLOCKS`].
pub fn max_plaintext_size_for_target(target_size: usize) -> Result<usize, StreamEncError> {
    if target_size < MIN_CIPHERTEXT_SIZE {
        return Err(StreamEncError::TargetTooSmall { got: target_size });
    }
    if target_size % EC_DATA_BLOCKS != 0 {
        return Err(StreamEncError::TargetNotAligned { got: target_size });
    }
    let n = ceil_div(target_size, MAX_BLOCK_SIZE);
    if n == 1 {
        return Ok((target_size - HEADER_SIZE - TAG_SIZE).min(BLOCK0_DATA_SIZE));
    }
    let last_cipher = target_size - (n - 1) * MAX_BLOCK_SIZE;
    let last_data = (last_cipher - TAG_SIZE).min(BLOCKN_DATA_SIZE);
    Ok(BLOCK0_DATA_SIZE + (n - 2) * BLOCKN_DATA_SIZE + last_data)
}

/// Parses the header from block 0.
///
/// Returns `(version, initial_nonce, plaintext_size)`.
/// `data` must be at least [`HEADER_SIZE`] bytes.
pub fn parse_header(data: &[u8]) -> Result<(u8, [u8; NONCE_SIZE], u32), StreamEncError> {
    if data.len() < HEADER_SIZE {
        return Err(StreamEncError::HeaderTooShort);
    }
    let ver = data[0];
    let mut nonce = [0u8; NONCE_SIZE];
    nonce.copy_from_slice(&data[VERSION_SIZE..VERSION_SIZE + NONCE_SIZE]);
    let plaintext_size = u32::from_be_bytes(
        data[VERSION_SIZE + NONCE_SIZE..HEADER_SIZE]
            .try_into()
            .unwrap(),
    );
    Ok((ver, nonce, plaintext_size))
}

/// Derives the per-block nonce by incrementing the last 4 bytes of `initial_nonce` by `block_index`.
pub fn block_nonce(mut initial_nonce: [u8; NONCE_SIZE], block_index: usize) -> [u8; NONCE_SIZE] {
    let v = u32::from_be_bytes(initial_nonce[8..].try_into().unwrap());
    let incremented = v.wrapping_add(block_index as u32);
    initial_nonce[8..].copy_from_slice(&incremented.to_be_bytes());
    initial_nonce
}

/// Encrypts `buf` in-place using block-based AES-GCM streaming encryption.
///
/// `buf` must have capacity for `len(buf) + overhead(len(buf))` bytes.
/// Returns the number of bytes written into `buf` on success.
///
/// The algorithm encrypts blocks right-to-left so each block's ciphertext never overwrites
/// unprocessed plaintext.
#[cfg(not(target_arch = "wasm32"))]
pub fn encrypt(key: &[u8], buf: &mut Vec<u8>, info: &str) -> Result<usize, StreamEncError> {
    if buf.is_empty() {
        return Err(StreamEncError::EmptyPlaintext);
    }

    let plaintext_len = buf.len();
    let n = num_blocks(plaintext_len);
    let last_block_idx = n - 1;
    let last_data = block_data_size(plaintext_len, last_block_idx);
    let padding = last_block_padding(n, last_data);
    let last_cipher = last_data + padding + TAG_SIZE;

    let total_size = if n == 1 {
        HEADER_SIZE + last_cipher
    } else {
        (n - 1) * MAX_BLOCK_SIZE + last_cipher
    };

    if buf.capacity() < total_size {
        return Err(StreamEncError::BufferTooSmall);
    }

    let gcm = make_gcm(key, info)?;

    // Generate random initial nonce
    let mut initial_nonce = [0u8; NONCE_SIZE];
    OsRng.fill_bytes(&mut initial_nonce);

    // Extend buf to total_size so we can write into it
    // Safety: we already verified capacity >= total_size above
    unsafe { buf.set_len(total_size) };

    // Encrypt right-to-left to avoid overwriting unprocessed plaintext
    let mut tmp = vec![0u8; BLOCKN_DATA_SIZE];
    for i in (0..n).rev() {
        let (capacity, plaintext_start) = if i == 0 {
            (BLOCK0_DATA_SIZE, 0usize)
        } else {
            (
                BLOCKN_DATA_SIZE,
                BLOCK0_DATA_SIZE + (i - 1) * BLOCKN_DATA_SIZE,
            )
        };
        let plaintext_end = (plaintext_start + capacity).min(plaintext_len);
        let block_len = plaintext_end - plaintext_start;

        let padded_len = if i == last_block_idx {
            last_data + padding
        } else {
            capacity
        };

        tmp[..block_len].copy_from_slice(&buf[plaintext_start..plaintext_end]);
        if block_len < padded_len {
            tmp[block_len..padded_len].fill(0);
        }

        let nonce_bytes = block_nonce(initial_nonce, i);
        let nonce = aes_gcm::Nonce::from_slice(&nonce_bytes);

        let offset = if i == 0 {
            HEADER_SIZE
        } else {
            i * MAX_BLOCK_SIZE
        };

        let out_slice = &mut buf[offset..offset + padded_len + TAG_SIZE];
        out_slice[..padded_len].copy_from_slice(&tmp[..padded_len]);
        gcm.encrypt_in_place_detached(nonce, b"", &mut out_slice[..padded_len])
            .map(|tag| {
                out_slice[padded_len..padded_len + TAG_SIZE].copy_from_slice(&tag);
            })
            .map_err(|_| StreamEncError::AesGcm)?;
    }

    // Write header into the start of block 0
    buf[0] = VERSION;
    buf[VERSION_SIZE..VERSION_SIZE + NONCE_SIZE].copy_from_slice(&initial_nonce);
    buf[VERSION_SIZE + NONCE_SIZE..HEADER_SIZE]
        .copy_from_slice(&(plaintext_len as u32).to_be_bytes());

    Ok(total_size)
}

/// Decrypts a single ciphertext block in-place.
///
/// For block 0, `buf` must include the header prefix.  `plaintext_size` is the total
/// unencrypted chunk size (read from the header) and is used to strip trailing zero-padding
/// from the last block.  Returns the number of plaintext bytes written to `buf[0..n]`.
pub fn decrypt_block(
    gcm: &Aes256Gcm,
    buf: &mut [u8],
    initial_nonce: [u8; NONCE_SIZE],
    version: u8,
    block_index: usize,
    plaintext_size: usize,
) -> Result<usize, StreamEncError> {
    let (ciphertext_start, nonce_bytes) = if block_index == 0 {
        if buf.len() < HEADER_SIZE {
            return Err(StreamEncError::Block0TooShort { index: block_index });
        }
        if buf[0] != version {
            return Err(StreamEncError::VersionMismatch {
                got: buf[0],
                want: version,
            });
        }
        (HEADER_SIZE, block_nonce(initial_nonce, 0))
    } else {
        (0, block_nonce(initial_nonce, block_index))
    };

    let nonce = aes_gcm::Nonce::from_slice(&nonce_bytes);
    let (header, cipher_and_tag) = buf.split_at_mut(ciphertext_start);
    gcm.decrypt_in_place(nonce, b"", cipher_and_tag)
        .map_err(|_| StreamEncError::AesGcm)?;

    let actual = block_data_size(plaintext_size, block_index);
    let plaintext_len = cipher_and_tag.len() - TAG_SIZE;
    if plaintext_len > actual {
        cipher_and_tag[actual..plaintext_len].fill(0);
    }
    // For block 0: move decrypted bytes over the header
    if block_index == 0 && ciphertext_start > 0 {
        let _ = header; // already split off
                        // Copy actual plaintext bytes to start of buf
        let plain_bytes: Vec<u8> = cipher_and_tag[..actual].to_vec();
        buf[..actual].copy_from_slice(&plain_bytes);
    }

    Ok(actual)
}

/// Decrypts all blocks in `buf` in-place, compacting plaintext to the start of `buf`.
///
/// The plaintext size is read from the header in block 0.
/// Returns the number of recovered plaintext bytes.
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

    let gcm = make_gcm(key, info)?;

    let n = num_blocks(plaintext_size);
    let mut offset = 0usize;
    for i in 0..n {
        let start = i * MAX_BLOCK_SIZE;
        let end = (start + MAX_BLOCK_SIZE).min(buf.len());
        let block_slice = &mut buf[start..end];
        // Decrypt into a temporary buffer then copy to compacted position
        let plain_n = {
            let nonce_bytes = block_nonce(initial_nonce, i);
            let nonce = aes_gcm::Nonce::from_slice(&nonce_bytes);
            let (ct_start, ct_slice): (usize, &mut [u8]) = if i == 0 {
                (HEADER_SIZE, &mut block_slice[HEADER_SIZE..])
            } else {
                (0, block_slice)
            };
            gcm.decrypt_in_place(nonce, b"", ct_slice)
                .map_err(|_| StreamEncError::AesGcm)?;
            let actual = block_data_size(plaintext_size, i);
            // Strip padding: the decrypted plaintext may have trailing zeros
            let _ = ct_start;
            actual
        };
        // Compact: copy block i's plaintext to buf[offset..offset+plain_n]
        let src_start = if i == 0 {
            HEADER_SIZE
        } else {
            i * MAX_BLOCK_SIZE
        };
        // We need to use copy_within since src and dst may overlap for later blocks
        if offset != src_start {
            buf.copy_within(src_start..src_start + plain_n, offset);
        }
        offset += plain_n;
    }

    buf[offset..].fill(0);
    Ok(offset)
}

// ────────────────────────────────────────────────────────────────────────────
// Internal helpers
// ────────────────────────────────────────────────────────────────────────────

/// Number of zero-padding bytes appended to the last block so total ciphertext length
/// is a multiple of [`EC_DATA_BLOCKS`].
fn last_block_padding(num_blocks: usize, last_data: usize) -> usize {
    if num_blocks == 1 {
        (EC_DATA_BLOCKS - (HEADER_SIZE + last_data) % EC_DATA_BLOCKS) % EC_DATA_BLOCKS
    } else {
        (EC_DATA_BLOCKS - last_data % EC_DATA_BLOCKS) % EC_DATA_BLOCKS
    }
}

/// Derives an AES-256-GCM cipher from `key` + `info` using HKDF.
fn make_gcm(key: &[u8], info: &str) -> Result<Aes256Gcm, StreamEncError> {
    use hkdf::Hkdf;
    use sha2::Sha256;

    let hk = Hkdf::<Sha256>::new(None, key);
    let mut derived = [0u8; KEY_LEN];
    hk.expand(info.as_bytes(), &mut derived)
        .map_err(|e| EncryptionError::KeyDerivation(format!("HKDF failed: {:?}", e)))?;
    Ok(Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&derived)))
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = b"a32bytekeya32bytekeya32bytekey00";
        let info = "test/info";
        let plaintext = b"Hello, streaming encryption! This is a test of the block-based format.";

        let required_cap = plaintext.len() + overhead(plaintext.len());
        let mut buf = Vec::with_capacity(required_cap);
        buf.extend_from_slice(plaintext);

        let n = encrypt(key, &mut buf, info).expect("encrypt failed");
        assert!(
            n > plaintext.len(),
            "ciphertext should be larger than plaintext"
        );

        let recovered = decrypt_all_blocks(key, &mut buf, info, VERSION).expect("decrypt failed");
        assert_eq!(recovered, plaintext.len());
        assert_eq!(&buf[..recovered], plaintext);
    }

    #[test]
    fn test_header_round_trip() {
        let key = b"a32bytekeya32bytekeya32bytekey00";
        let info = "bucket/file";
        let plaintext: Vec<u8> = (0u8..200).collect();

        let required_cap = plaintext.len() + overhead(plaintext.len());
        let mut buf = Vec::with_capacity(required_cap);
        buf.extend_from_slice(&plaintext);

        encrypt(key, &mut buf, info).unwrap();

        let (ver, _nonce, plen) = parse_header(&buf).unwrap();
        assert_eq!(ver, VERSION);
        assert_eq!(plen as usize, plaintext.len());
    }

    #[test]
    fn test_num_blocks() {
        assert_eq!(num_blocks(0), 0);
        assert_eq!(num_blocks(1), 1);
        assert_eq!(num_blocks(BLOCK0_DATA_SIZE), 1);
        assert_eq!(num_blocks(BLOCK0_DATA_SIZE + 1), 2);
    }

    #[test]
    fn test_max_plaintext_size_for_target_aligned() {
        let target = 16 * MAX_BLOCK_SIZE;
        let max_pt = max_plaintext_size_for_target(target).unwrap();
        let enc_overhead = overhead(max_pt);
        assert_eq!(
            max_pt + enc_overhead,
            target,
            "ciphertext size must equal target"
        );
    }
}
