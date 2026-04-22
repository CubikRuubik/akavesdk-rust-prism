use cid::{
    multihash::{Code, Multihash, MultihashDigest},
    Cid,
};
use ipfs_unixfs::file::adder::{BalancedCollector, Chunker, Collector, FileAdder};

use crate::types::sdk_types::FileBlockUpload;

pub const DAG_PROTOBUF: u64 = 0x70;
// pub const RAW: u64 = 0x55;  // Unused constant

#[derive(Debug)]
pub(crate) struct ChunkDag {
    pub cid: Cid,
    pub encoded_size: usize,
    pub blocks: Vec<FileBlockUpload>,
}

impl ChunkDag {
    pub fn new(size: usize, data: Vec<u8>) -> Self {
        let dag_builder = FileAdder::builder();

        let chunker = Chunker::Size(size);
        let collector = Collector::Balanced(BalancedCollector::default());
        let mut adder = dag_builder
            .with_chunker(chunker)
            .with_collector(collector)
            .build();

        let mut total = 0;
        let mut dag_blocks = vec![];
        let data_len = data.len();

        while total < data_len {
            let mut end = total + size;
            if end > data_len {
                end = data_len
            }
            let (blocks, consumed) = adder.push(&data[total..end]);
            total += consumed;
            blocks.for_each(|block| {
                dag_blocks.push(block);
            });
        }

        adder.finish().for_each(|block| {
            dag_blocks.push(block);
        });
        let mut blocks = vec![];

        let mut total_dag_size = 0usize;

        dag_blocks.iter().for_each(|(_, block_data)| {
            total_dag_size += block_data.len();
            let hash: Multihash = Code::Sha2_256.digest(block_data);
            let cid = Cid::new_v1(DAG_PROTOBUF, hash);
            blocks.push(FileBlockUpload {
                cid,
                data: block_data.to_owned(),
                permit: "".to_string(),
                node_address: "".to_string(),
                node_id: "".to_string(),
            });
        });

        // Compute encoded size:
        // - single block: the block's full serialised length (== total_dag_size)
        // - multi-block: sum of all leaf block sizes (root node excluded)
        let encoded_size = if blocks.len() > 1 {
            blocks[..blocks.len() - 1].iter().map(|b| b.data.len()).sum()
        } else {
            total_dag_size
        };

        let cid = blocks
            .last()
            .expect("blocks should not be empty at this point")
            .cid;

        // Remove the root node from blocks if there are multiple blocks
        // The root node is not uploaded as a separate block
        if blocks.len() > 1 {
            let _ = blocks.pop();
        }

        Self {
            cid,
            encoded_size,
            blocks,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::erasure::ErasureCode;

    #[test]
    fn test_chunk_encoded_size_without_erasure() {
        let data = vec![0u8; 10 * 1024 * 1024]; // 10 MiB
        let block_size = 1024 * 1024; // 1 MiB

        let dag = ChunkDag::new(block_size, data);

        // Invariant: encoded_size must equal sum of all leaf block data lengths
        let blocks_total: usize = dag.blocks.iter().map(|b| b.data.len()).sum();
        assert_eq!(
            dag.encoded_size,
            blocks_total,
            "encoded_size must equal sum of leaf block sizes"
        );

        // Pin the concrete value: 10 blocks × (1 MiB + 14 bytes protobuf overhead)
        assert_eq!(dag.encoded_size, 10_485_900);
    }

    #[test]
    fn test_chunk_encoded_size_with_erasure() {
        let raw = vec![0u8; 10 * 1024 * 1024]; // 10 MiB
        let ec = ErasureCode::new(16, 16).unwrap();
        let encoded = ec.encode(&raw).unwrap();
        let block_size = encoded.len() / 32; // one shard per block

        let dag = ChunkDag::new(block_size, encoded);

        // Invariant: encoded_size must equal sum of all leaf block data lengths
        let blocks_total: usize = dag.blocks.iter().map(|b| b.data.len()).sum();
        assert_eq!(
            dag.encoded_size,
            blocks_total,
            "encoded_size must equal sum of leaf block sizes"
        );

        // Pin the concrete value: 32 blocks × (655360 bytes + 14 bytes protobuf overhead)
        assert_eq!(dag.encoded_size, 20_971_968);
    }
}
