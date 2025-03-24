use super::erasure::ErasureCode;

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
        start: usize,      // Starting index for the current combination
        remaining: usize,  // How many more indices we need to add
        n: usize,         // Total number of shards
        k: usize,         // Total number of missing shards we want
        current: &mut Vec<usize>,  // Current combination being built
        result: &mut Vec<Vec<usize>>,  // All combinations found so far
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
    let data_shards = 5;    // Number of data shards (original data split into 5 parts)
    let parity_shards = 3;  // Number of parity shards (can recover up to 3 missing shards)

    let encoder = ErasureCode::new(data_shards, parity_shards).unwrap();
    assert_eq!(encoder.data_blocks, data_shards);
    assert_eq!(encoder.parity_blocks, parity_shards);

    let encoded = encoder.encode(data).unwrap();
    println!("Encoded data: {:?}", encoded);
    let shard_size = encoded.len() / (data_shards + parity_shards);  // Size of each shard

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
    let data_shards = 5;    // Number of data shards
    let parity_shards = 3;  // Number of parity shards

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
    let data_shards = 5;    // Number of data shards
    let parity_shards = 3;  // Number of parity shards

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