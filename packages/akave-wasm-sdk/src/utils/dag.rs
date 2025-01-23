use std::iter::Peekable;

use crate::sdk::ipcnodeapi::{ipc_file_upload_create_request::IpcBlock, IpcFileBlockData};
use sha2::{Digest, Sha256};

use super::file_chunker::FileChunker;

pub struct DagBuilder {
    pub chunker: Peekable<FileChunker>,
    root_hasher: Sha256,
    pub root_cid: Option<String>,
}

impl Iterator for DagBuilder {
    type Item = (IpcBlock, IpcFileBlockData);

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

        let ipc_block = IpcBlock {
            cid: hash.clone(),
            size: chunk.len() as i64,
        };
        let block_data = IpcFileBlockData {
            data: chunk.into_vec(),
            cid: hash,
        };

        if self.chunker.peek().is_none() {
            self.root_cid = Some(hex::encode(Sha256::digest(
                self.root_hasher.clone().finalize(),
            )))
        }

        Some((ipc_block, block_data))
    }
}

impl DagBuilder {
    pub fn new(chunker: FileChunker) -> Self {
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
