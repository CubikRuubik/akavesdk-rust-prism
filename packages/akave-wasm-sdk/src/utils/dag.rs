use std::{iter::Peekable, vec::IntoIter};

use sha2::{Digest, Sha256};

pub struct DagBuilder {
    pub chunked: Peekable<IntoIter<Vec<u8>>>,
    root_hasher: Sha256,
    pub root_cid: Option<String>,
}

pub struct FileBlockUpload {
    pub cid: String,
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
        return self.chunked.len();
    }

    fn next(&mut self) -> Option<Self::Item> {
        let chunk = self.chunked.next()?;
        let mut hasher = Sha256::new();
        hasher.update(&chunk);
        let hash = hex::encode(hasher.finalize());
        // TODO: this need to be properly tested
        self.root_hasher.update(&hash);

        let ipc_block = FileBlockUpload {
            cid: hash.clone(),
            data: chunk,
            permit: "".to_string(),
            node_address: "".to_string(),
            node_id: "".to_string(),
        };

        if self.chunked.peek().is_none() {
            self.root_cid = Some(hex::encode(Sha256::digest(
                self.root_hasher.clone().finalize(),
            )))
        }

        Some(ipc_block)
    }
}

impl DagBuilder {
    pub fn new(data: Vec<u8>, chunk_size: usize) -> Self {
        Self {
            chunked: DagBuilder::split_vec(data, chunk_size)
                .into_iter()
                .peekable(),
            root_hasher: Sha256::new(),
            root_cid: None,
        }
    }

    pub fn root_cid(&self) -> Result<String, Box<dyn std::error::Error>> {
        match &self.root_cid {
            Some(cid) => Ok(cid.to_string()),
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
