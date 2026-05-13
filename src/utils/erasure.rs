use reed_solomon_erasure::{galois_8, ReedSolomon};
use thiserror::Error;

use crate::utils::encryption::ceil_div;

#[derive(Error, Debug)]
pub enum ErasureCodeError {
    #[error("erasure coding error")]
    ReedSolomonError(#[source] reed_solomon_erasure::Error),

    #[error("data and parity blocks must be > 0")]
    InvalidBlockCount,

    #[error("invalid shard count: expected {expected}, got {got}")]
    InvalidShardCount { expected: usize, got: usize },

    #[error(
        "insufficient data for reconstruction: need at least {required} shards, got {available}"
    )]
    InsufficientData { required: usize, available: usize },

    #[error("data unwrap failed: {0}")]
    UnwrapFailed(String),
}

/// Overhead bytes added by `encode` (wrap format): 8-byte size prefix + 4-byte magic suffix.
pub const WRAP_OVERHEAD: usize = 12;

const WRAP_PREFIX: usize = 8;
const MAGIC_SUFFIX: [u8; 4] = [0xDE, 0xAD, 0xBE, 0xEF];

/// ErasureCode is a wrapper around the ReedSolomon encoder, providing a more user-friendly interface.
#[derive(Clone)]
pub struct ErasureCode {
    /// Number of data blocks
    pub data_blocks: usize,
    /// Number of parity blocks
    pub parity_blocks: usize,
    enc: ReedSolomon<galois_8::Field>,
}

impl ErasureCode {
    /// Creates a new ErasureCode instance with the specified number of data and parity blocks.
    pub fn new(data_blocks: usize, parity_blocks: usize) -> Result<Self, ErasureCodeError> {
        if data_blocks == 0 || parity_blocks == 0 {
            return Err(ErasureCodeError::InvalidBlockCount);
        }

        let enc = ReedSolomon::<galois_8::Field>::new(data_blocks, parity_blocks)
            .map_err(ErasureCodeError::ReedSolomonError)?;

        Ok(Self {
            data_blocks,
            parity_blocks,
            enc,
        })
    }

    /// Encodes the input data using Reed-Solomon erasure coding, returning the encoded data
    /// with a size prefix and magic suffix (wrapped format).
    pub fn encode(&self, data: &[u8]) -> Result<Vec<u8>, ErasureCodeError> {
        let wrapped = wrap_data(data);
        let total_blocks = self.data_blocks + self.parity_blocks;
        let shard_size = wrapped.len().div_ceil(self.data_blocks);

        let mut shards = vec![vec![0u8; shard_size]; total_blocks];
        for (i, chunk) in wrapped.chunks(shard_size).enumerate() {
            if i >= self.data_blocks {
                break;
            }
            shards[i][..chunk.len()].copy_from_slice(chunk);
        }

        self.enc
            .encode(&mut shards)
            .map_err(ErasureCodeError::ReedSolomonError)?;

        let mut result = Vec::with_capacity(shard_size * total_blocks);
        for shard in shards {
            result.extend_from_slice(&shard);
        }

        Ok(result)
    }

    /// Extracts the original data from the encoded data using Reed-Solomon erasure coding.
    /// The data must have been encoded with `encode` (wrapped format); the original size is
    /// recovered from the embedded prefix.
    pub fn extract_data(
        &self,
        mut blocks: Vec<Vec<u8>>,
    ) -> Result<Vec<u8>, ErasureCodeError> {
        self.reconstruct_if_needed(&mut blocks)?;

        let shard_size = blocks[0].len();
        let out_size = self.data_blocks * shard_size;
        let mut buf = Vec::with_capacity(out_size);
        for i in 0..self.data_blocks {
            buf.extend_from_slice(&blocks[i]);
        }

        unwrap_data(&buf)
    }

    /// Encodes data without wrapping — shards are returned as-is for use with `extract_data_raw`.
    pub fn encode_raw(&self, data: &[u8]) -> Result<Vec<Vec<u8>>, ErasureCodeError> {
        let total_blocks = self.data_blocks + self.parity_blocks;
        let shard_size = data.len().div_ceil(self.data_blocks);

        let mut shards = vec![vec![0u8; shard_size]; total_blocks];
        for (i, chunk) in data.chunks(shard_size).enumerate() {
            if i >= self.data_blocks {
                break;
            }
            shards[i][..chunk.len()].copy_from_slice(chunk);
        }

        self.enc
            .encode(&mut shards)
            .map_err(ErasureCodeError::ReedSolomonError)?;

        Ok(shards)
    }

    /// Extracts the original data from raw-encoded shards (no unwrapping).
    /// `original_size` must match the byte count passed to `encode_raw`.
    pub fn extract_data_raw(
        &self,
        mut blocks: Vec<Vec<u8>>,
        original_size: usize,
    ) -> Result<Vec<u8>, ErasureCodeError> {
        self.reconstruct_if_needed(&mut blocks)?;

        let shard_size = blocks[0].len();
        let out_size = self.data_blocks * shard_size;
        let mut buf = Vec::with_capacity(out_size);
        for i in 0..self.data_blocks {
            buf.extend_from_slice(&blocks[i]);
        }
        buf.truncate(original_size);
        Ok(buf)
    }

    /// Reconstructs missing/corrupt shards if needed. Returns error if reconstruction fails.
    fn reconstruct_if_needed(&self, blocks: &mut Vec<Vec<u8>>) -> Result<(), ErasureCodeError> {
        match self.enc.verify(blocks) {
            Ok(true) => Ok(()),
            Ok(false) => {
                let null_blocks: Vec<Option<Vec<u8>>> = blocks
                    .iter()
                    .map(|b| {
                        if b.iter().all(|&x| x == 0) {
                            None
                        } else {
                            Some(b.clone())
                        }
                    })
                    .collect();
                let mut decoder_shards = null_blocks;
                self.enc
                    .reconstruct(&mut decoder_shards)
                    .map_err(ErasureCodeError::ReedSolomonError)?;
                for (i, shard) in decoder_shards.into_iter().enumerate() {
                    if let Some(s) = shard {
                        blocks[i] = s;
                    }
                }
                match self.enc.verify(blocks) {
                    Ok(true) => Ok(()),
                    Ok(false) => Err(ErasureCodeError::UnwrapFailed("data is corrupted after reconstruction".to_string())),
                    Err(e) => Err(ErasureCodeError::ReedSolomonError(e)),
                }
            }
            Err(e) => {
                if matches!(e, reed_solomon_erasure::Error::IncorrectShardSize) {
                    let mut decoder_shards: Vec<Option<Vec<u8>>> = blocks
                        .iter()
                        .map(|b| if b.is_empty() { None } else { Some(b.clone()) })
                        .collect();
                    self.enc
                        .reconstruct(&mut decoder_shards)
                        .map_err(ErasureCodeError::ReedSolomonError)?;
                    for (i, shard) in decoder_shards.into_iter().enumerate() {
                        if let Some(s) = shard {
                            blocks[i] = s;
                        }
                    }
                    Ok(())
                } else {
                    Err(ErasureCodeError::ReedSolomonError(e))
                }
            }
        }
    }
}

/// Splits `data` into stripes of at most `max_stripe_size` bytes.
/// The last stripe may be smaller.
pub fn split_stripes(data: &[u8], max_stripe_size: usize) -> Vec<&[u8]> {
    if data.is_empty() || max_stripe_size == 0 {
        return Vec::new();
    }
    let n = ceil_div(data.len(), max_stripe_size);
    let mut result = Vec::with_capacity(n);
    for i in 0..n {
        let start = i * max_stripe_size;
        let end = (start + max_stripe_size).min(data.len());
        result.push(&data[start..end]);
    }
    result
}

fn wrap_data(data: &[u8]) -> Vec<u8> {
    let mut buf = vec![0u8; WRAP_PREFIX + data.len() + MAGIC_SUFFIX.len()];
    let data_len = data.len() as u64;
    buf[..WRAP_PREFIX].copy_from_slice(&data_len.to_be_bytes());
    buf[WRAP_PREFIX..WRAP_PREFIX + data.len()].copy_from_slice(data);
    buf[WRAP_PREFIX + data.len()..].copy_from_slice(&MAGIC_SUFFIX);
    buf
}

fn unwrap_data(buf: &[u8]) -> Result<Vec<u8>, ErasureCodeError> {
    if buf.len() < WRAP_OVERHEAD {
        return Err(ErasureCodeError::UnwrapFailed("buffer too short".to_string()));
    }
    let size = u64::from_be_bytes(buf[..WRAP_PREFIX].try_into().unwrap()) as usize;
    let data_end = WRAP_PREFIX + size;
    if data_end + MAGIC_SUFFIX.len() > buf.len() {
        return Err(ErasureCodeError::UnwrapFailed("size prefix out of bounds".to_string()));
    }
    if &buf[data_end..data_end + MAGIC_SUFFIX.len()] != &MAGIC_SUFFIX {
        return Err(ErasureCodeError::UnwrapFailed("missing magic suffix or corrupted data".to_string()));
    }
    Ok(buf[WRAP_PREFIX..data_end].to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn missing_shards_idx(n: usize, k: usize) -> Vec<Vec<usize>> {
        if k == 0 || k > n {
            return Vec::new();
        }
        let mut result = Vec::new();
        let mut current = Vec::with_capacity(k);
        fn gen(start: usize, remaining: usize, n: usize, current: &mut Vec<usize>, result: &mut Vec<Vec<usize>>) {
            if remaining == 0 {
                result.push(current.clone());
                return;
            }
            for i in start..=n - remaining {
                current.push(i);
                gen(i + 1, remaining - 1, n, current, result);
                current.pop();
            }
        }
        gen(0, k, n, &mut current, &mut result);
        result
    }

    fn split_into_blocks(encoded: &[u8], shard_size: usize) -> Vec<Vec<u8>> {
        encoded.chunks(shard_size)
            .map(|chunk| {
                let mut block = vec![0u8; shard_size];
                block[..chunk.len()].copy_from_slice(chunk);
                block
            })
            .collect()
    }

    #[test]
    fn test_erasure_code_invalid_params() {
        assert!(ErasureCode::new(0, 0).is_err());
        assert!(ErasureCode::new(16, 0).is_err());
    }

    #[test]
    fn test_erasure_code_no_missing_shards() {
        let data = b"Quick brown fox jumps over the lazy dog";
        let data_shards = 5;
        let parity_shards = 3;
        let encoder = ErasureCode::new(data_shards, parity_shards).unwrap();
        let encoded = encoder.encode(data).unwrap();
        let shard_size = encoded.len() / (data_shards + parity_shards);
        let blocks = split_into_blocks(&encoded, shard_size);
        let extracted = encoder.extract_data(blocks).unwrap();
        assert_eq!(data.to_vec(), extracted);
    }

    #[test]
    fn test_erasure_code_with_missing_shards() {
        let data = b"Quick brown fox jumps over the lazy dog";
        let data_shards = 5;
        let parity_shards = 3;
        let encoder = ErasureCode::new(data_shards, parity_shards).unwrap();
        let encoded = encoder.encode(data).unwrap();
        let shard_size = encoded.len() / (data_shards + parity_shards);
        let all_combos = missing_shards_idx(data_shards + parity_shards, parity_shards);
        for missing_idxs in all_combos {
            let mut blocks = split_into_blocks(&encoded, shard_size);
            for idx in missing_idxs {
                blocks[idx] = vec![0u8; shard_size];
            }
            let extracted = encoder.extract_data(blocks).unwrap();
            assert_eq!(data.to_vec(), extracted);
        }
    }

    #[test]
    fn test_erasure_code_too_many_missing_shards() {
        let data = b"Quick brown fox jumps over the lazy dog";
        let data_shards = 5;
        let parity_shards = 3;
        let encoder = ErasureCode::new(data_shards, parity_shards).unwrap();
        let encoded = encoder.encode(data).unwrap();
        let shard_size = encoded.len() / (data_shards + parity_shards);
        let mut blocks = split_into_blocks(&encoded, shard_size);
        for i in 0..=parity_shards {
            blocks[i] = vec![0u8; shard_size];
        }
        assert!(encoder.extract_data(blocks).is_err());
    }

    #[test]
    fn test_encode_raw_extract_raw() {
        let data = b"Hello, erasure coding without wrapping!";
        let encoder = ErasureCode::new(4, 2).unwrap();
        let shards = encoder.encode_raw(data).unwrap();
        let extracted = encoder.extract_data_raw(shards, data.len()).unwrap();
        assert_eq!(data.to_vec(), extracted);
    }

    #[test]
    fn test_split_stripes_preserves_data() {
        let max_stripe = 10usize;
        for size in [0, 1, max_stripe - 1, max_stripe, max_stripe + 1, 3 * max_stripe] {
            let data: Vec<u8> = (0..size).map(|i| i as u8).collect();
            let stripes = split_stripes(&data, max_stripe);
            let rejoined: Vec<u8> = stripes.iter().flat_map(|s| s.iter().copied()).collect();
            assert_eq!(rejoined, data, "size={size}");
        }
    }
}
