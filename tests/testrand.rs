// Test utility helpers for generating random test data.

use cid::{
    multihash::{Code, MultihashDigest},
    Cid,
};

/// Codec value for dag-pb (DagProtobuf).
const DAG_PROTOBUF: u64 = 0x70;

/// Generates an IPFS block with `size` random bytes.
///
/// Fills a buffer with cryptographically random data, computes a SHA2-256
/// multihash, and wraps it in a CIDv1 with the DagProtobuf codec.
/// Returns the raw byte payload and its corresponding CID.
pub fn block(size: usize) -> (Vec<u8>, Cid) {
    let mut data = vec![0u8; size];
    getrandom::getrandom(&mut data).expect("failed to generate random bytes");
    let mh = Code::Sha2_256.digest(&data);
    let cid = Cid::new_v1(DAG_PROTOBUF, mh);
    (data, cid)
}

#[test]
fn test_block_generates_correct_size() {
    let size = 256;
    let (data, cid) = block(size);
    assert_eq!(data.len(), size);
    // CIDv1 with DagProtobuf codec (0x70)
    assert_eq!(cid.version(), cid::Version::V1);
    assert_eq!(cid.codec(), DAG_PROTOBUF);
}

#[test]
fn test_block_generates_unique_data() {
    let (data1, cid1) = block(64);
    let (data2, cid2) = block(64);
    // Statistically near-impossible for two random 64-byte buffers to be equal
    assert_ne!(data1, data2);
    assert_ne!(cid1, cid2);
}
