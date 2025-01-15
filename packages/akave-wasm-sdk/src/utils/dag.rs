use crate::sdk::ipcnodeapi::{ipc_file_upload_create_request::IpcBlock, IpcFileBlockData};
use sha2::{Digest, Sha256};

use super::file_chunker::FileChunker;

pub struct DagBuilder {
    pub chunker: FileChunker,
    pub root_hasher: Sha256,
}

impl Iterator for DagBuilder {
    type Item = (IpcBlock, IpcFileBlockData, String);

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
        let hash = format!("sha256-{}", hex::encode(hasher.finalize()));
        self.root_hasher.update(&hash);
        let root_cid = format!(
            "sha256-{}",
            hex::encode(Sha256::digest(self.root_hasher.clone().finalize()))
        );

        let ipc_block = IpcBlock {
            cid: hash.clone(),
            size: chunk.len() as i64,
        };
        let block_data = IpcFileBlockData {
            data: chunk.into_vec(),
            cid: hash,
        };

        Some((ipc_block, block_data, root_cid))
    }
}

impl DagBuilder {
    pub fn new(chunker: FileChunker) -> Self {
        Self {
            chunker,
            root_hasher: Sha256::new(),
        }
    }
}
