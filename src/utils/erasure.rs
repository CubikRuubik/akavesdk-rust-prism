// Copyright (C) 2024 Akave
// See LICENSE for copying information.

use reed_solomon_erasure::{galois_8, ReedSolomon};
use thiserror::Error;
use web3::block_on;

#[derive(Error, Debug)]
pub enum ErasureCodeError {
    #[error("erasure coding error: {0}")]
    ReedSolomonError(String),
    
    #[error("data and parity blocks must be > 0")]
    InvalidBlockCount,
}

/// ErasureCode is a wrapper around the ReedSolomon encoder, providing a more user-friendly interface.
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
        self.enc.encode(&mut shards)
            .map_err(|e| ErasureCodeError::ReedSolomonError(e.to_string()))?;

        // Concatenate all shards into a single byte vector
        let mut result = Vec::with_capacity(shard_size * total_blocks);
        for shard in shards {
            result.extend_from_slice(&shard);
        }

        Ok(result)
    }

    /// Extracts the original data from the encoded data using Reed-Solomon erasure coding.
    pub fn extract_data(&self, blocks: Vec<Vec<u8>>, original_data_size: usize) -> Result<Vec<u8>, ErasureCodeError> {        
        // Verify and reconstruct if needed
        if !self.enc.verify(&blocks).map_err(|e| ErasureCodeError::ReedSolomonError(e.to_string()))? {
            // Convert empty vectors to None for reconstruction
            let mut decoder_shards = blocks.into_iter()
                .map(|block| if block.iter().all(|&x| x == 0) { None } else { Some(block) })
                .collect::<Vec<_>>();
            
            // Reconstruct the shards
            self.enc.reconstruct_data(&mut decoder_shards)
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