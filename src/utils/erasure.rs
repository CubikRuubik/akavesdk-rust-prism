use reed_solomon_erasure::{galois_8, ReedSolomon};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ErasureCodeError {
    #[error("erasure coding error: {0}")]
    ReedSolomonError(String),

    #[error("data and parity blocks must be > 0")]
    InvalidBlockCount,
    
    #[cfg(target_arch = "wasm32")]
    #[error("erasure coding not supported in WASM")]
    WasmNotSupported,
}

/// ErasureCode is a wrapper around the ReedSolomon encoder, providing a more user-friendly interface.
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
            .map_err(|e| ErasureCodeError::ReedSolomonError(e.to_string()))?;

        Ok(Self {
            data_blocks,
            parity_blocks,
            enc,
        })
    }

    /// Encodes the input data using Reed-Solomon erasure coding, returning the encoded data.
    pub fn encode(&self, data: &[u8]) -> Result<Vec<u8>, ErasureCodeError> {
        // Split the data into shards
        let total_blocks = self.data_blocks + self.parity_blocks;
        let shard_size = (data.len() + self.data_blocks - 1) / self.data_blocks;

        // Create shards
        let mut shards = vec![vec![0u8; shard_size]; total_blocks];

        // Fill data shards
        for (i, chunk) in data.chunks(shard_size).enumerate() {
            if i >= self.data_blocks {
                break;
            }
            let shard = &mut shards[i];
            shard[..chunk.len()].copy_from_slice(chunk);
        }

        // Encode the shards
        self.enc
            .encode(&mut shards)
            .map_err(|e| ErasureCodeError::ReedSolomonError(e.to_string()))?;

        // Concatenate all shards into a single byte vector
        let mut result = Vec::with_capacity(shard_size * total_blocks);
        for shard in shards {
            result.extend_from_slice(&shard);
        }

        Ok(result)
    }

    /// Extracts the original data from the encoded data using Reed-Solomon erasure coding.
    pub fn extract_data(
        &self,
        blocks: Vec<Vec<u8>>,
        original_data_size: usize,
    ) -> Result<Vec<u8>, ErasureCodeError> {
        // Verify and reconstruct if needed
        if !self
            .enc
            .verify(&blocks)
            .map_err(|e| ErasureCodeError::ReedSolomonError(e.to_string()))?
        {
            // Convert empty vectors to None for reconstruction
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

            // Reconstruct the shards
            self.enc
                .reconstruct_data(&mut decoder_shards)
                .map_err(|e| ErasureCodeError::ReedSolomonError(e.to_string()))?;

            // Join the data blocks
            let mut buffer = Vec::with_capacity(original_data_size);

            // Only take from data blocks (not parity blocks)
            for i in 0..self.data_blocks {
                if i < decoder_shards.len() {
                    if let Some(block) = &decoder_shards[i] {
                        buffer.extend_from_slice(block);
                    }
                }
            }

            // Trim to original size
            buffer.truncate(original_data_size);

            return Ok(buffer);
        }

        // If no reconstruction needed, just join the original blocks
        let mut buffer = Vec::with_capacity(original_data_size);
        for i in 0..self.data_blocks {
            if i < blocks.len() {
                buffer.extend_from_slice(&blocks[i]);
            }
        }

        // Trim to original size
        buffer.truncate(original_data_size);

        Ok(buffer)
    }
}

// #[cfg(target_arch = "wasm32")]
// impl ErasureCode {
//     /// Creates a new ErasureCode instance with the specified number of data and parity blocks.
//     pub fn new(data_blocks: usize, parity_blocks: usize) -> Result<Self, ErasureCodeError> {
//         if data_blocks == 0 || parity_blocks == 0 {
//             return Err(ErasureCodeError::InvalidBlockCount);
//         }

//         Ok(Self {
//             data_blocks,
//             parity_blocks,
//         })
//     }

//     /// Encodes the input data using Reed-Solomon erasure coding, returning the encoded data.
//     /// In WASM environment, this just passes through the original data.
//     pub fn encode(&self, data: &[u8]) -> Result<Vec<u8>, ErasureCodeError> {
//         // Pass through original data in WASM environment
//         Ok(data.to_vec())
//     }

//     /// Extracts the original data from the encoded data.
//     /// In WASM environment, this function combines all blocks into one.
//     pub fn extract_data(&self, blocks: Vec<Vec<u8>>, original_data_size: usize) -> Result<Vec<u8>, ErasureCodeError> {        
//         let mut buffer = Vec::with_capacity(original_data_size);
//         for i in 0..self.data_blocks {
//             if i < blocks.len() {
//                 buffer.extend_from_slice(&blocks[i]);
//             }
//         }
        
//         // Trim to original size
//         buffer.truncate(original_data_size);
        
//         Ok(buffer)
//     }
// }

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

        let extracted = encoder.extract_data(blocks, data.len()).unwrap();
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

            let extracted = encoder.extract_data(blocks, data.len()).unwrap();
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
        assert!(encoder.extract_data(blocks, data.len()).is_err());
    }
}
