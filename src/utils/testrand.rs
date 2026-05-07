/// Test utilities for generating random IPFS data.
///
/// This module is the Rust port of `private/testrand/testrand.go` from the Go SDK.
/// It is available only in test builds on native (non-WASM) targets.
use aes_gcm::aead::rand_core::{OsRng, RngCore};
use cid::{
    multihash::{Code, MultihashDigest},
    Cid,
};

/// Generate an IPFS block with `size` bytes of random data.
///
/// Returns a tuple of `(data, cid)` where `data` is the raw random bytes and `cid`
/// is a CIDv1 with the DagProtobuf codec (0x70) and a SHA2-256 multihash of `data`.
/// This mirrors the Go `testrand.Block` helper.
pub fn random_block(size: usize) -> (Vec<u8>, Cid) {
    let mut data = vec![0u8; size];
    OsRng.fill_bytes(&mut data);
    let mh = Code::Sha2_256.digest(&data);
    let cid = Cid::new_v1(0x70, mh);
    (data, cid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_random_block_size() {
        let (data, _cid) = random_block(1024);
        assert_eq!(data.len(), 1024);
    }

    #[test]
    fn test_random_block_cid_is_v1_dag_protobuf() {
        let (data, cid) = random_block(64);
        assert_eq!(cid.version(), cid::Version::V1);
        assert_eq!(cid.codec(), 0x70, "codec should be DagProtobuf (0x70)");

        // Verify the CID matches the data
        let expected_mh = Code::Sha2_256.digest(&data);
        let expected_cid = Cid::new_v1(0x70, expected_mh);
        assert_eq!(cid, expected_cid);
    }

    #[test]
    fn test_random_block_different_calls_produce_different_data() {
        let (data1, _) = random_block(32);
        let (data2, _) = random_block(32);
        assert_ne!(data1, data2, "random blocks should differ between calls");
    }
}
