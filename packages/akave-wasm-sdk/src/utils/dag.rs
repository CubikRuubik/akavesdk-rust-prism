use crate::sdk::ipcnodeapi::{ipc_file_upload_create_request::IpcBlock, IpcFileBlockData};
use sha2::{Digest, Sha256};

const BLOCK_SIZE: usize = 1024 * 1024; // 1MB blocks

#[derive(Debug)]
pub struct DagNode {
    pub data: Vec<u8>,
    pub hash: String,
    pub links: Vec<String>,
}

pub struct DagBuilder;

impl DagBuilder {
    pub fn create_dag(data: &[u8]) -> Result<(Vec<DagNode>, String), Box<dyn std::error::Error>> {
        let mut blocks = Vec::new();
        let mut links = Vec::new();

        // Split data into blocks and hash them
        for chunk in data.chunks(BLOCK_SIZE) {
            let mut hasher = Sha256::new();
            hasher.update(chunk);
            let hash = format!("sha256-{}", hex::encode(hasher.finalize())); // TODO: refine id generation

            blocks.push(DagNode {
                data: chunk.to_vec(),
                hash: hash.clone(),
                links: vec![],
            });
            links.push(hash);
        }

        // Create root node
        let mut root_hasher = Sha256::new();
        for link in &links {
            root_hasher.update(link);
        }
        let root_hash = format!("sha256-{}", hex::encode(root_hasher.finalize()));

        // Add root node
        blocks.push(DagNode {
            data: Vec::new(), // Root node contains no data, only links
            hash: root_hash.clone(),
            links,
        });

        Ok((blocks, root_hash))
    }

    pub fn to_ipc_blocks(nodes: &[DagNode]) -> Vec<IpcBlock> {
        nodes
            .iter()
            .map(|node| IpcBlock {
                cid: node.hash.clone(),
                size: node.data.len() as i64,
            })
            .collect()
    }

    pub fn to_ipc_block_data(nodes: &[DagNode]) -> Vec<IpcFileBlockData> {
        nodes
            .iter()
            .map(|node| IpcFileBlockData {
                data: node.data.clone(),
                cid: node.hash.clone(),
            })
            .collect()
    }
}
