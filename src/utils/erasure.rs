use reed_solomon_erasure::{galois_8, ReedSolomon};
use thiserror::Error;

const WRAP_PREFIX: usize = 8;
const MAGIC_SUFFIX: [u8; 4] = [0xDE, 0xAD, 0xBE, 0xEF];
pub const WRAP_OVERHEAD: usize = WRAP_PREFIX + MAGIC_SUFFIX.len(); // = 12

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

    #[error("invalid magic suffix")]
    InvalidMagic,

    #[error("data too short to unwrap")]
    DataTooShort,
}

fn wrap_data(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(WRAP_PREFIX + data.len() + MAGIC_SUFFIX.len());
    out.extend_from_slice(&(data.len() as u64).to_be_bytes());
    out.extend_from_slice(data);
    out.extend_from_slice(&MAGIC_SUFFIX);
    out
}

fn unwrap_data(buf: &[u8]) -> Result<Vec<u8>, ErasureCodeError> {
    if buf.len() < WRAP_OVERHEAD {
        return Err(ErasureCodeError::DataTooShort);
    }
    let len = u64::from_be_bytes(buf[..8].try_into().unwrap()) as usize;
    let data_end = 8 + len;
    if buf.len() < data_end + MAGIC_SUFFIX.len() {
        return Err(ErasureCodeError::DataTooShort);
    }
    let magic = &buf[data_end..data_end + MAGIC_SUFFIX.len()];
    if magic != &MAGIC_SUFFIX {
        return Err(ErasureCodeError::InvalidMagic);
    }
    Ok(buf[8..data_end].to_vec())
}

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

    /// Encodes the input data using Reed-Solomon erasure coding, returning the encoded data.
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
    pub fn extract_data(&self, blocks: Vec<Vec<u8>>) -> Result<Vec<u8>, ErasureCodeError> {
        let shard_size = blocks.first().map(|b| b.len()).unwrap_or(0);
        let total_data_size = shard_size * self.data_blocks;

        if !self
            .enc
            .verify(&blocks)
            .map_err(ErasureCodeError::ReedSolomonError)?
        {
            let mut decoder_shards = blocks
                .into_iter()
                .map(|block| {
                    if block.iter().all(|&x| x == 0) {
                        None
                    } else {
                        Some(block)
                    }
                })
                .collect::<Vec<_>>();
            self.enc
                .reconstruct_data(&mut decoder_shards)
                .map_err(ErasureCodeError::ReedSolomonError)?;
            let mut buffer = Vec::with_capacity(total_data_size);
            for i in 0..self.data_blocks {
                if i < decoder_shards.len() {
                    if let Some(block) = &decoder_shards[i] {
                        buffer.extend_from_slice(block);
                    }
                }
            }
            return unwrap_data(&buffer);
        }

        let mut buffer = Vec::with_capacity(total_data_size);
        for i in 0..self.data_blocks {
            if i < blocks.len() {
                buffer.extend_from_slice(&blocks[i]);
            }
        }
        unwrap_data(&buffer)
    }

    /// Encodes data into raw shards without wrap overhead.
    // used by CHANGE-9 (Upload2/Download2), which was skipped
    #[allow(dead_code)]
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

    /// Extracts raw data (no unwrap) truncating to original_size.
    // used by CHANGE-9 (Upload2/Download2), which was skipped
    #[allow(dead_code)]
    pub fn extract_data_raw(
        &self,
        blocks: Vec<Vec<u8>>,
        original_size: usize,
    ) -> Result<Vec<u8>, ErasureCodeError> {
        if !self
            .enc
            .verify(&blocks)
            .map_err(ErasureCodeError::ReedSolomonError)?
        {
            let mut decoder_shards = blocks
                .into_iter()
                .map(|block| {
                    if block.iter().all(|&x| x == 0) {
                        None
                    } else {
                        Some(block)
                    }
                })
                .collect::<Vec<_>>();
            self.enc
                .reconstruct_data(&mut decoder_shards)
                .map_err(ErasureCodeError::ReedSolomonError)?;
            let mut buffer = Vec::new();
            for i in 0..self.data_blocks {
                if i < decoder_shards.len() {
                    if let Some(block) = &decoder_shards[i] {
                        buffer.extend_from_slice(block);
                    }
                }
            }
            buffer.truncate(original_size);
            return Ok(buffer);
        }
        let mut buffer = Vec::new();
        for i in 0..self.data_blocks {
            if i < blocks.len() {
                buffer.extend_from_slice(&blocks[i]);
            }
        }
        buffer.truncate(original_size);
        Ok(buffer)
    }

    /// Split data into stripes of at most max_stripe_size bytes.
    // used by CHANGE-9 (Upload2/Download2), which was skipped
    #[allow(dead_code)]
    pub fn split_stripes(data: &[u8], max_stripe_size: usize) -> Vec<&[u8]> {
        let n = (data.len() + max_stripe_size - 1) / max_stripe_size;
        (0..n)
            .map(|i| {
                let start = i * max_stripe_size;
                let end = ((i + 1) * max_stripe_size).min(data.len());
                &data[start..end]
            })
            .collect()
    }
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

        fn generate_combinations(
            start: usize,
            remaining: usize,
            n: usize,
            k: usize,
            current: &mut Vec<usize>,
            result: &mut Vec<Vec<usize>>,
        ) {
            if remaining == 0 {
                let combination = current.clone();
                result.push(combination);
                return;
            }

            for i in start..=n - remaining {
                if current.is_empty() || i > *current.last().unwrap() {
                    current.push(i);
                    generate_combinations(i + 1, remaining - 1, n, k, current, result);
                    current.pop();
                }
            }
        }

        generate_combinations(0, k, n, k, &mut current, &mut result);
        result
    }

    fn split_into_blocks(encoded: &[u8], shard_size: usize) -> Vec<Vec<u8>> {
        let mut blocks = Vec::new();
        for chunk in encoded.chunks(shard_size) {
            let mut block = vec![0u8; shard_size];
            block[..chunk.len()].copy_from_slice(chunk);
            blocks.push(block);
        }
        blocks
    }

    #[test]
    fn test_erasure_code_invalid_params() {
        assert!(ErasureCode::new(0, 0).is_err());
        assert!(ErasureCode::new(16, 0).is_err());
    }

    #[test]
    fn test_erasure_code_no_missing_shards() {
        let data = b"Quick brown fox jumps over the lazy dog";
        println!("\nOriginal data: {:?}", data);
        let data_shards = 5;
        let parity_shards = 3;

        let encoder = ErasureCode::new(data_shards, parity_shards).unwrap();
        assert_eq!(encoder.data_blocks, data_shards);
        assert_eq!(encoder.parity_blocks, parity_shards);

        let encoded = encoder.encode(data).unwrap();
        println!("Encoded data: {:?}", encoded);
        let shard_size = encoded.len() / (data_shards + parity_shards);

        let blocks = split_into_blocks(&encoded, shard_size);
        println!("Blocks before extraction: {:?}", blocks);

        let extracted = encoder.extract_data(blocks).unwrap();
        println!("Extracted data: {:?}", extracted);
        assert_eq!(data.to_vec(), extracted);
    }

    #[test]
    fn test_erasure_code_with_missing_shards() {
        let data = b"Quick brown fox jumps over the lazy dog";
        println!("\nOriginal data: {:?}", data);
        let data_shards = 5;
        let parity_shards = 3;

        let encoder = ErasureCode::new(data_shards, parity_shards).unwrap();
        let encoded = encoder.encode(data).unwrap();
        println!("Encoded data: {:?}", encoded);
        let shard_size = encoded.len() / (data_shards + parity_shards);

        let all_combos = missing_shards_idx(data_shards + parity_shards, parity_shards);
        for missing_idxs in all_combos {
            let mut blocks = split_into_blocks(&encoded, shard_size);

            for idx in missing_idxs {
                blocks[idx] = vec![0u8; shard_size];
            }
            println!("Blocks before extraction (with missing): {:?}", blocks);

            let extracted = encoder.extract_data(blocks).unwrap();
            println!("Extracted data (with missing): {:?}", extracted);
            assert_eq!(data.to_vec(), extracted);
        }
    }

    #[test]
    fn test_erasure_code_too_many_missing_shards() {
        let data = b"Quick brown fox jumps over the lazy dog";
        println!("\nOriginal data: {:?}", data);
        let data_shards = 5;
        let parity_shards = 3;

        let encoder = ErasureCode::new(data_shards, parity_shards).unwrap();
        let encoded = encoder.encode(data).unwrap();
        println!("Encoded data: {:?}", encoded);
        let shard_size = encoded.len() / (data_shards + parity_shards);

        let mut blocks = split_into_blocks(&encoded, shard_size);
        for i in 0..=parity_shards {
            blocks[i] = vec![0u8; shard_size];
        }
        println!("Blocks before extraction (too many missing): {:?}", blocks);
        assert!(encoder.extract_data(blocks).is_err());
    }
}
