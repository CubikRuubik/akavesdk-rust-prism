use cid::{
    multihash::{Code, Multihash, MultihashDigest},
    Cid,
};

use ipfs_unixfs::file::adder::{BalancedCollector, Chunker, Collector, FileAdder};

pub const DAG_PROTOBUF: u64 = 0x70;

#[derive(Debug)]
pub struct FileBlockUpload {
    pub cid: Cid,
    pub data: Vec<u8>,

    pub permit: String,
    pub node_address: String,
    pub node_id: String,
}

#[derive(Debug)]
pub struct ChunkDag {
    pub cid: Cid,
    pub raw_data_size: usize,
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

        let mut raw_data_size = 0;

        dag_blocks.iter().for_each(|(_, block_data)| {
            let hash: Multihash = Code::Sha2_256.digest(&block_data);
            let cid = Cid::new_v1(DAG_PROTOBUF, hash);
            raw_data_size += block_data.len();
            blocks.push(FileBlockUpload {
                cid,
                data: block_data.to_owned(),
                permit: "".to_string(),
                node_address: "".to_string(),
                node_id: "".to_string(),
            });
        });

        let proto_node_size = blocks.last().unwrap().data.len();
        let cid = blocks.last().unwrap().cid;

        if blocks.len() > 1 {
            let _ = blocks.pop();
        }

        return Self {
            cid,
            raw_data_size,
            proto_node_size,
            blocks,
        };
    }
}
