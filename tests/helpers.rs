/// Test helper utilities shared across integration tests.
use cid::{
    multihash::{Code, MultihashDigest},
    Cid,
};

/// Generates a random block of `size` bytes and returns its CIDv1/SHA2-256/DagProtobuf content
/// identifier together with the raw bytes.
///
/// This mirrors the Go SDK `testrand.Block(t, size)` helper added in v0.4.4.
/// The CID uses:
/// - CID version 1
/// - DagProtobuf codec (`0x70`)
/// - SHA2-256 multihash of the raw data
pub fn random_block(size: usize) -> (String, Vec<u8>) {
    let mut data = vec![0u8; size];
    getrandom::getrandom(&mut data).expect("failed to generate random bytes");
    let mh = Code::Sha2_256.digest(&data);
    // DagProtobuf codec = 0x70
    let c = Cid::new_v1(0x70, mh);
    (c.to_string(), data)
}
