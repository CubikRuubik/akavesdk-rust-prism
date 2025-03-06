use cid::{
    multihash::{Code, Multihash, MultihashDigest},
    Cid,
};

use ipfs_unixfs::file::adder::{BalancedCollector, Chunker, Collector, FileAdder};

pub const DAG_PROTOBUF: u64 = 0x70;

pub struct FileBlockUpload {
    pub cid: Cid,
    pub data: Vec<u8>,

    pub permit: String,
    pub node_address: String,
    pub node_id: String,
}
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

        let mut finished_blocks = 0;
        adder.finish().for_each(|block| {
            dag_blocks.push(block);
            finished_blocks += 1;
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

        println!("Amount of finished blocks: {}", finished_blocks);
        if finished_blocks > 1 {
            let last_blocks = dag_blocks[(dag_blocks.len() - finished_blocks)..].to_vec();
            let root_hasher = Code::Sha2_256;
            let mut root_hash = None;
            last_blocks
                .iter()
                .for_each(|blk| root_hash = Some(root_hasher.digest(&blk.0.to_bytes())));
            blocks.push(FileBlockUpload {
                cid: Cid::new_v1(DAG_PROTOBUF, root_hash.unwrap()),
                data: [].to_vec(),
                permit: "".to_string(),
                node_address: "".to_string(),
                node_id: "".to_string(),
            });
        }

        /* let root_hasher = Code::Sha2_256;
        let mut root_hash = None;



        let cid =

        blocks.push(FileBlockUpload {
            cid,
            data: [].to_vec(),
            permit: "".to_string(),
            node_address: "".to_string(),
            node_id: "".to_string(),
        }); */

        let proto_node_size = blocks.last().unwrap().data.len();
        let cid = blocks.last().unwrap().cid;

        return Self {
            cid,
            raw_data_size,
            proto_node_size,
            blocks,
        };
    }
}
