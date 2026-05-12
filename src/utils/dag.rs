use cid::{
    multihash::{Code, Multihash, MultihashDigest},
    Cid,
};
use ipfs_unixfs::file::adder::{BalancedCollector, Chunker, Collector, FileAdder};
use quick_protobuf::{MessageWrite, Writer};

use crate::types::sdk_types::FileBlockUpload;
use crate::utils::pb_data::{mod_Data, PbData};

pub const DAG_PROTOBUF: u64 = 0x70;
// pub const RAW: u64 = 0x55;  // Unused constant

/// Builds the canonical dag-pb / UnixFS root CID for a multi-chunk file, matching
/// Go's DAGRoot (boxo/ipld/merkledag + unixfs.TFile node).
///
/// For a single chunk the chunk's own CID is the root. For multiple chunks a
/// PBNode is constructed whose links point to the chunk CIDs and whose Data
/// field is a serialised UnixFS File message carrying each chunk's raw size.
pub struct DagRoot {
    /// (chunk_cid, raw_data_size, encoded_size)
    links: Vec<(Cid, u64, u64)>,
}

impl DagRoot {
    pub fn new() -> Self {
        Self { links: Vec::new() }
    }

    /// Record one chunk. `raw_data_size` is the unencoded/un-erasure-coded size
    /// of the chunk data (used for the UnixFS blocksizes field). `encoded_size`
    /// is the size stored in the dag-pb link's `Tsize` field.
    pub fn add_link(&mut self, cid: Cid, raw_data_size: u64, encoded_size: u64) {
        self.links.push((cid, raw_data_size, encoded_size));
    }

    /// Compute the root CID.
    ///
    /// If there is only one chunk, returns that chunk's CID directly (matching
    /// Go's `if len(root.node.Links()) == 1 { return root.node.Links()[0].Cid }`).
    ///
    /// For multiple chunks, builds and hashes the canonical dag-pb node.
    pub fn build(self) -> Result<Cid, String> {
        match self.links.len() {
            0 => Err("DagRoot: no chunks added".into()),
            1 => Ok(self.links[0].0),
            _ => {
                let node_bytes = Self::encode_pbnode(&self.links)?;
                let mh = Code::Sha2_256.digest(&node_bytes);
                Ok(Cid::new_v1(DAG_PROTOBUF, mh))
            }
        }
    }

    /// Encode a canonical dag-pb PBNode with UnixFS File data.
    ///
    /// Canonical dag-pb requires Links (field 2) to be written **before** Data
    /// (field 1) in the serialised bytes.
    fn encode_pbnode(links: &[(Cid, u64, u64)]) -> Result<Vec<u8>, String> {
        // 1. Serialise the UnixFS Data message (TFile with per-chunk block sizes).
        let block_sizes: Vec<u64> = links.iter().map(|(_, raw, _)| *raw).collect();
        let total_filesize: u64 = block_sizes.iter().sum();
        let pb_data = PbData {
            data_type: mod_Data::DataType::File,
            file_size: Some(total_filesize),
            block_sizes,
            ..Default::default()
        };
        let mut data_bytes: Vec<u8> = Vec::new();
        let mut w = Writer::new(&mut data_bytes);
        pb_data
            .write_message(&mut w)
            .map_err(|e| format!("PbData encode: {e}"))?;

        // 2. Build the PBNode manually so we can write Links (field 2) before
        //    Data (field 1) — required for canonical dag-pb hashing.
        //
        //    Wire types: length-delimited (2) → tag = (field_number << 3) | 2
        //    PBNode.Links  field 2  → tag varint = 18  (0x12)
        //    PBNode.Data   field 1  → tag varint = 10  (0x0A)
        //
        //    PBLink fields (written in order Hash, Name, Tsize):
        //    Hash   field 1 bytes   → tag = 10
        //    Name   field 2 string  → tag = 18
        //    Tsize  field 3 uint64  → tag = 24  (varint wire type 0)
        let mut node_bytes: Vec<u8> = Vec::new();

        for (cid, _, tsize) in links {
            let cid_bytes = cid.to_bytes();
            let link_bytes = Self::encode_pblink(&cid_bytes, *tsize);
            // Write as length-delimited field 2 of PBNode
            Self::write_bytes_field(&mut node_bytes, 2, &link_bytes);
        }

        // Data field (field 1 of PBNode)
        Self::write_bytes_field(&mut node_bytes, 1, &data_bytes);

        Ok(node_bytes)
    }

    /// Encode a single PBLink as a length-delimited message payload.
    fn encode_pblink(hash: &[u8], tsize: u64) -> Vec<u8> {
        let mut out: Vec<u8> = Vec::new();
        Self::write_bytes_field(&mut out, 1, hash); // Hash
        Self::write_string_field(&mut out, 2, ""); // Name (empty)
        Self::write_varint_field(&mut out, 3, tsize); // Tsize
        out
    }

    /// Append a length-delimited (wire type 2) protobuf field.
    pub(crate) fn write_bytes_field(buf: &mut Vec<u8>, field_number: u64, value: &[u8]) {
        Self::write_varint(buf, (field_number << 3) | 2);
        Self::write_varint(buf, value.len() as u64);
        buf.extend_from_slice(value);
    }

    /// Append a string field (same encoding as bytes in protobuf).
    fn write_string_field(buf: &mut Vec<u8>, field_number: u64, value: &str) {
        Self::write_bytes_field(buf, field_number, value.as_bytes());
    }

    /// Append a varint (wire type 0) protobuf field.
    fn write_varint_field(buf: &mut Vec<u8>, field_number: u64, value: u64) {
        Self::write_varint(buf, (field_number << 3) | 0);
        Self::write_varint(buf, value);
    }

    /// Encode a u64 as a protobuf varint into `buf`.
    fn write_varint(buf: &mut Vec<u8>, mut value: u64) {
        loop {
            let byte = (value & 0x7F) as u8;
            value >>= 7;
            if value == 0 {
                buf.push(byte);
                break;
            } else {
                buf.push(byte | 0x80);
            }
        }
    }
}

// used by CHANGE-9 (Upload2/Download2), which was skipped
#[allow(dead_code)]
pub fn build_cid(data: &[u8]) -> Cid {
    let mh = Code::Sha2_256.digest(data);
    Cid::new_v1(DAG_PROTOBUF, mh)
}

// used by CHANGE-9 (Upload2/Download2), which was skipped
#[allow(dead_code)]
pub fn build_leaf_node(data: &[u8]) -> Result<(Cid, Vec<u8>), String> {
    use crate::utils::pb_data::{mod_Data, PbData};
    use std::borrow::Cow;

    let pb_data = PbData {
        data_type: mod_Data::DataType::File,
        data: Some(Cow::Borrowed(data)),
        file_size: Some(data.len() as u64),
        block_sizes: vec![],
        ..Default::default()
    };
    let mut data_bytes: Vec<u8> = Vec::new();
    let mut w = quick_protobuf::Writer::new(&mut data_bytes);
    pb_data
        .write_message(&mut w)
        .map_err(|e| format!("PbData encode: {e}"))?;

    let mut node_bytes: Vec<u8> = Vec::new();
    DagRoot::write_bytes_field(&mut node_bytes, 1, &data_bytes);

    let cid = build_cid(&node_bytes);
    Ok((cid, node_bytes))
}

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
            blocks[..blocks.len() - 1]
                .iter()
                .map(|b| b.data.len())
                .sum()
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
            dag.encoded_size, blocks_total,
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
            dag.encoded_size, blocks_total,
            "encoded_size must equal sum of leaf block sizes"
        );

        // Pin the concrete value: 32 blocks × (655360 bytes + 14 bytes protobuf overhead)
        assert_eq!(dag.encoded_size, 20_971_968);
    }

    #[test]
    fn test_root_cid_builder() {
        let data = vec![0u8; 2 * 1024 * 1024];
        let chunk_size = 1024 * 1024;

        let dag = ChunkDag::new(chunk_size, data);

        assert_eq!(dag.blocks.len(), 2, "expected 2 leaf blocks");

        println!("root CID: {}", dag.cid);

        assert_eq!(
            dag.cid.to_string(),
            "bafybeig5m3nullnds6gh6yriwk6yyn6h476kb4idns5nl5k7pstyhbcuii"
        );
    }
}
