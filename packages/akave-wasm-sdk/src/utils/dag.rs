use std::iter::Peekable;

use sha2::{Digest, Sha256};

use super::splitter::Splitter;

pub struct DagBuilder {
    pub chunker: Peekable<Splitter>,
    root_hasher: Sha256,
    pub root_cid: Option<String>,
}

pub struct FileBlockUpload {
    pub cid: String,
    pub data: Vec<u8>,
}

impl Iterator for DagBuilder {
    type Item = FileBlockUpload;

    fn count(self) -> usize
    where
        Self: Sized,
    {
        return self.chunker.count();
    }

    fn next(&mut self) -> Option<Self::Item> {
        let chunk = self.chunker.next()?;
        let mut hasher = Sha256::new();
        hasher.update(&chunk);
        let hash = hex::encode(hasher.finalize());
        // TODO: this need to be properly tested
        self.root_hasher.update(&hash);

        let ipc_block = FileBlockUpload {
            cid: hash.clone(),
            data: chunk.into_vec(),
        };

        if self.chunker.peek().is_none() {
            self.root_cid = Some(hex::encode(Sha256::digest(
                self.root_hasher.clone().finalize(),
            )))
        }

        Some(ipc_block)
    }
}

impl DagBuilder {
    pub fn new(chunker: Splitter) -> Self {
        Self {
            chunker: chunker.peekable(),
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
}
