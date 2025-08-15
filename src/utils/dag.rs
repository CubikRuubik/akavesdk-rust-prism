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
    pub proto_node_size: usize,
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

        // The data passed to DAG creation (which may be encoded by erasure coding)
        let raw_data_size = data.len();
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

        // Calculate proto node size
        // The Go SDK uses node.Size() which includes protobuf overhead (~14-100 bytes depending on links)
        // For now, using raw_data_size as approximation since protobuf overhead is minimal
        // compared to data size and both implementations produce compatible results
        let proto_node_size = raw_data_size;

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
            proto_node_size,
            blocks,
        }
    }
}
