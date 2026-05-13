use reed_solomon_erasure::{galois_8, ReedSolomon};
use thiserror::Error;

/// Overhead bytes added by the wrap format: 8-byte size prefix + 4-byte magic suffix.
pub const WRAP_OVERHEAD: usize = 12;

const PREFIX_SIZE: usize = 8;
const MAGIC_SUFFIX: [u8; 4] = [0xDE, 0xAD, 0xBE, 0xEF];

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

    #[error("unwrap error: {0}")]
    UnwrapError(String),

    #[error("data is corrupted")]
    DataCorrupted,
}

/// ErasureCode is a wrapper around the ReedSolomon encoder, providing a more user-friendly interface.
#[derive(Clone)]
pub struct ErasureCode {
    /// Number of data blocks
    pub data_blocks: usize,
    /// Number of parity blocks
    pub parity_blocks: usize,
    // #[cfg(not(target_arch = "wasm32"))]
    enc: ReedSolomon<galois_8::Field>,
}

// #[cfg(not(target_arch = "wasm32"))]
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

    /// Encodes the input data using Reed-Solomon erasure coding with wrap overhead.
    /// The data is wrapped with an 8-byte size prefix and 4-byte magic suffix before
    /// encoding, matching Go's Encode behavior.
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

    /// Encodes the input data using Reed-Solomon erasure coding without wrapping.
    /// Returns the raw shards (data + parity), suitable for use with `extract_data_raw`.
    // used by CHANGE-9 (Upload2/Download2), which was skipped
    #[allow(dead_code)]
    pub fn encode_raw(&self, data: &[u8]) -> Result<Vec<Vec<u8>>, ErasureCodeError> {
        let shard_size = data.len().div_ceil(self.data_blocks);
        let total_blocks = self.data_blocks + self.parity_blocks;
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

    /// Extracts the original data from the encoded data using Reed-Solomon erasure coding.
    /// The original size is recovered by unwrapping the size header written by `encode`.
    pub fn extract_data(&self, blocks: Vec<Vec<u8>>) -> Result<Vec<u8>, ErasureCodeError> {
        let joined = self.reconstruct_and_join(blocks)?;
        let data = unwrap_data(&joined)?;
        Ok(data.to_vec())
    }

    /// Extracts the original data from raw shards without unwrapping.
    /// `original_size` must be the exact byte count passed to `encode_raw`.
    // used by CHANGE-9 (Upload2/Download2), which was skipped
    #[allow(dead_code)]
    pub fn extract_data_raw(
        &self,
        blocks: Vec<Vec<u8>>,
        original_size: usize,
    ) -> Result<Vec<u8>, ErasureCodeError> {
        let reconstructed = self.reconstruct_blocks(blocks)?;
        let shard_size = reconstructed[0].len();
        let out_size = self.data_blocks * shard_size;
        let mut buf = Vec::with_capacity(out_size);
        for i in 0..self.data_blocks {
            buf.extend_from_slice(&reconstructed[i]);
        }
        buf.truncate(original_size);
        Ok(buf)
    }

    /// Reconstructs any missing blocks and joins all data shards into a contiguous buffer.
    fn reconstruct_and_join(&self, blocks: Vec<Vec<u8>>) -> Result<Vec<u8>, ErasureCodeError> {
        let reconstructed = self.reconstruct_blocks(blocks)?;
        let shard_size = reconstructed[0].len();
        let out_size = self.data_blocks * shard_size;
        let mut buf = Vec::with_capacity(out_size);
        for i in 0..self.data_blocks {
            buf.extend_from_slice(&reconstructed[i]);
        }
        Ok(buf)
    }

    /// Verifies and reconstructs shards if needed. Returns all shards (data + parity).
    fn reconstruct_blocks(
        &self,
        mut blocks: Vec<Vec<u8>>,
    ) -> Result<Vec<Vec<u8>>, ErasureCodeError> {
        use reed_solomon_erasure::Error as RSError;

        let ok = match self.enc.verify(&blocks) {
            Ok(ok) => ok,
            Err(RSError::EmptyShard)
            | Err(RSError::InvalidShardFlags)
            | Err(RSError::IncorrectShardSize) => {
                // Proceed to reconstruct when shard size issues are detected
                let blocks_as_option: Vec<Option<Vec<u8>>> = blocks
                    .into_iter()
                    .map(|b| {
                        if b.iter().all(|&x| x == 0) {
                            None
                        } else {
                            Some(b)
                        }
                    })
                    .collect();
                let mut option_blocks = blocks_as_option;
                self.enc
                    .reconstruct(&mut option_blocks)
                    .map_err(ErasureCodeError::ReedSolomonError)?;
                let rebuilt: Vec<Vec<u8>> = option_blocks
                    .into_iter()
                    .map(|b| b.unwrap_or_default())
                    .collect();
                return self.verify_and_return(rebuilt);
            }
            Err(e) => return Err(ErasureCodeError::ReedSolomonError(e)),
        };

        if !ok {
            // Convert all-zero blocks to None and attempt reconstruction
            let blocks_as_option: Vec<Option<Vec<u8>>> = blocks
                .into_iter()
                .map(|b| {
                    if b.iter().all(|&x| x == 0) {
                        None
                    } else {
                        Some(b)
                    }
                })
                .collect();
            let mut option_blocks = blocks_as_option;
            self.enc
                .reconstruct(&mut option_blocks)
                .map_err(ErasureCodeError::ReedSolomonError)?;
            blocks = option_blocks
                .into_iter()
                .map(|b| b.unwrap_or_default())
                .collect();
            return self.verify_and_return(blocks);
        }

        Ok(blocks)
    }

    fn verify_and_return(&self, blocks: Vec<Vec<u8>>) -> Result<Vec<Vec<u8>>, ErasureCodeError> {
        let ok = self
            .enc
            .verify(&blocks)
            .map_err(ErasureCodeError::ReedSolomonError)?;
        if !ok {
            return Err(ErasureCodeError::DataCorrupted);
        }
        Ok(blocks)
    }
}

/// Wraps data with an 8-byte big-endian size prefix and 4-byte magic suffix.
fn wrap_data(data: &[u8]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(data.len() + WRAP_OVERHEAD);
    buf.extend_from_slice(&(data.len() as u64).to_be_bytes());
    buf.extend_from_slice(data);
    buf.extend_from_slice(&MAGIC_SUFFIX);
    buf
}

/// Unwraps data produced by `wrap_data`, validating the size prefix and magic suffix.
fn unwrap_data(buf: &[u8]) -> Result<&[u8], ErasureCodeError> {
    if buf.len() < WRAP_OVERHEAD {
        return Err(ErasureCodeError::UnwrapError(
            "buffer too short".to_string(),
        ));
    }
    let size = u64::from_be_bytes(
        buf[..PREFIX_SIZE]
            .try_into()
            .map_err(|_| ErasureCodeError::UnwrapError("failed to read size prefix".to_string()))?,
    ) as usize;
    let data_end = PREFIX_SIZE + size;
    let n = data_end + MAGIC_SUFFIX.len();
    if n > buf.len() {
        return Err(ErasureCodeError::UnwrapError(
            "buffer too short".to_string(),
        ));
    }
    if buf[data_end..n] != MAGIC_SUFFIX {
        return Err(ErasureCodeError::UnwrapError(
            "missing suffix or corrupted data".to_string(),
        ));
    }
    Ok(&buf[PREFIX_SIZE..data_end])
}

/// Splits data into stripes of at most `max_stripe_size` bytes each.
/// The last stripe may be smaller. Matches Go's `SplitStripes`.
// used by CHANGE-9 (Upload2/Download2), which was skipped
#[allow(dead_code)]
pub fn split_stripes(data: &[u8], max_stripe_size: usize) -> Vec<&[u8]> {
    if max_stripe_size == 0 || data.is_empty() {
        return Vec::new();
    }
    let n = data.len().div_ceil(max_stripe_size);
    let mut result = Vec::with_capacity(n);
    for i in 0..n {
        let start = i * max_stripe_size;
        let end = (start + max_stripe_size).min(data.len());
        result.push(&data[start..end]);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Generates all possible combinations of k indices from 0 to n-1.
    /// This is used to test all possible ways of missing k shards in our erasure coding tests.
    ///
    /// # Parameters
    /// * `n` - Total number of shards (data + parity)
    /// * `k` - Number of shards to be missing in each combination
    ///
    /// # Returns
    /// A vector of vectors, where each inner vector contains k indices representing which shards are missing.
    /// For example, if n=8 and k=2, one possible combination would be [0, 3] meaning shards 0 and 3 are missing.
    fn missing_shards_idx(n: usize, k: usize) -> Vec<Vec<usize>> {
        if k == 0 || k > n {
            return Vec::new();
        }

        let mut result = Vec::new();
        let mut current = Vec::with_capacity(k);

        /// Recursively generates combinations of indices.
        /// Uses backtracking to generate all possible combinations of k indices from start to n-1.
        fn generate_combinations(
            start: usize,                 // Starting index for the current combination
            remaining: usize,             // How many more indices we need to add
            n: usize,                     // Total number of shards
            k: usize,                     // Total number of missing shards we want
            current: &mut Vec<usize>,     // Current combination being built
            result: &mut Vec<Vec<usize>>, // All combinations found so far
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

    /// Splits encoded data into blocks of equal size, padding the last block if necessary.
    /// This simulates how the data would be split into shards in a real erasure coding system.
    ///
    /// # Parameters
    /// * `encoded` - The encoded data to split into blocks
    /// * `shard_size` - Size of each block (must be equal for all blocks)
    ///
    /// # Returns
    /// A vector of blocks, where each block is a vector of bytes of size shard_size.
    /// The last block may be padded with zeros if the input length is not divisible by shard_size.
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
        let data_shards = 5; // Number of data shards (original data split into 5 parts)
        let parity_shards = 3; // Number of parity shards (can recover up to 3 missing shards)

        let encoder = ErasureCode::new(data_shards, parity_shards).unwrap();
        assert_eq!(encoder.data_blocks, data_shards);
        assert_eq!(encoder.parity_blocks, parity_shards);

        let encoded = encoder.encode(data).unwrap();
        println!("Encoded data: {:?}", encoded);
        let shard_size = encoded.len() / (data_shards + parity_shards); // Size of each shard

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
        let data_shards = 5; // Number of data shards
        let parity_shards = 3; // Number of parity shards

        let encoder = ErasureCode::new(data_shards, parity_shards).unwrap();
        let encoded = encoder.encode(data).unwrap();
        println!("Encoded data: {:?}", encoded);
        let shard_size = encoded.len() / (data_shards + parity_shards);

        // Generate all possible combinations of missing shards (up to parity_shards)
        let all_combos = missing_shards_idx(data_shards + parity_shards, parity_shards);
        for missing_idxs in all_combos {
            let mut blocks = split_into_blocks(&encoded, shard_size);

            // Set missing blocks to empty vectors (simulating missing shards)
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
        let data_shards = 5; // Number of data shards
        let parity_shards = 3; // Number of parity shards

        let encoder = ErasureCode::new(data_shards, parity_shards).unwrap();
        let encoded = encoder.encode(data).unwrap();
        println!("Encoded data: {:?}", encoded);
        let shard_size = encoded.len() / (data_shards + parity_shards);

        let mut blocks = split_into_blocks(&encoded, shard_size);
        // Try to recover with more missing shards than parity shards (should fail)
        for i in 0..=parity_shards {
            blocks[i] = vec![0u8; shard_size];
        }
        println!("Blocks before extraction (too many missing): {:?}", blocks);
        assert!(encoder.extract_data(blocks).is_err());
    }
}
