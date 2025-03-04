use std::{iter::Peekable, vec::IntoIter};

use cid::{
    multihash::{Code, Multihash, MultihashDigest},
    Cid,
};

pub const DAG_PROTOBUF: u64 = 0x70;

pub struct DagBuilder {
    pub chunked: Peekable<IntoIter<Vec<u8>>>,
    root_hasher: Code,
    root_hash: Option<Multihash>,
    size: usize,
}

pub struct FileBlockUpload {
    pub cid: Cid,
    pub data: Vec<u8>,

    pub permit: String,
    pub node_address: String,
    pub node_id: String,
}

impl Iterator for DagBuilder {
    type Item = FileBlockUpload;

    fn count(self) -> usize
    where
        Self: Sized,
    {
        return self.size;
    }

    fn next(&mut self) -> Option<Self::Item> {
        let chunk = self.chunked.next()?;

        let hash: Multihash = Code::Sha2_256.digest(&chunk);
        let cid = Cid::new_v1(DAG_PROTOBUF, hash);
        let root_hash: Multihash = self.root_hasher.digest(&hash.to_bytes());

        let ipc_block = FileBlockUpload {
            cid,
            data: chunk,
            permit: "".to_string(),
            node_address: "".to_string(),
            node_id: "".to_string(),
        };

        if self.chunked.peek().is_none() {
            if self.size == 1 {
                self.root_hash = Some(hash);
            } else {
                self.root_hash = Some(root_hash);
            }
        }

        Some(ipc_block)
    }
}

impl DagBuilder {
    pub fn new(data: Vec<u8>, chunk_size: usize) -> Self {
        let chunked = DagBuilder::split_vec(data, chunk_size)
            .into_iter()
            .peekable();
        let size = chunked.len();
        Self {
            chunked,
            size,
            root_hasher: Code::Sha2_256,
            root_hash: None,
        }
    }

    pub fn root_cid(&self) -> Result<Cid, Box<dyn std::error::Error>> {
        match &self.root_hash {
            Some(hash) => {
                let cid = Cid::new_v1(DAG_PROTOBUF, *hash);
                Ok(cid)
            }
            None => Err("chunker need to be fully iterated to build the root_cid".into()),
        }
    }

    fn split_vec<T>(v: Vec<T>, chunk_size: usize) -> Vec<Vec<T>> {
        use std::collections::VecDeque;

        let mut v: VecDeque<T> = v.into(); // avoids reallocating when possible

        let mut acc = Vec::new();
        while v.len() > chunk_size {
            acc.push(v.drain(0..chunk_size).collect());
            v.shrink_to_fit();
        }
        acc.push(v.into());
        acc
    }
}
