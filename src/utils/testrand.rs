/// Test utilities for generating random data in tests.
///
/// This module mirrors the Go `private/testrand` package and is available
/// only when compiling test code on non-WASM targets.
#[cfg(all(test, not(target_arch = "wasm32")))]
pub mod rand {
    use aes_gcm::aead::{rand_core::RngCore, OsRng};
    use cid::Cid;
    use multihash::{Code, MultihashDigest};

    /// An IPFS block holding raw bytes together with its CIDv1 (dag-pb, sha2-256).
    pub struct IpfsBlock {
        /// Raw block data.
        pub data: Vec<u8>,
        /// CIDv1 with DagProtobuf codec (0x70) computed over `data`.
        pub cid: Cid,
    }

    /// Generates an IPFS block with `size` bytes of cryptographically random data.
    ///
    /// Fills a buffer with random bytes, computes a SHA2-256 multihash, creates a
    /// CIDv1 with DagProtobuf codec (0x70), and returns both together.
    pub fn block(size: usize) -> IpfsBlock {
        let mut data = vec![0u8; size];
        OsRng.fill_bytes(&mut data);
        let mh = Code::Sha2_256.digest(&data);
        // DagProtobuf codec = 0x70
        let cid = Cid::new_v1(0x70, mh);
        IpfsBlock { data, cid }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::rand::block;

    #[test]
    fn test_block_size() {
        let b = block(256);
        assert_eq!(b.data.len(), 256);
    }

    #[test]
    fn test_block_cid_is_dag_protobuf() {
        let b = block(64);
        // DagProtobuf codec = 0x70
        assert_eq!(b.cid.codec(), 0x70);
    }

    #[test]
    fn test_block_cid_is_v1() {
        let b = block(64);
        assert_eq!(b.cid.version(), cid::Version::V1);
    }

    #[test]
    fn test_blocks_are_unique() {
        let b1 = block(128);
        let b2 = block(128);
        assert_ne!(b1.data, b2.data, "consecutive random blocks should differ");
        assert_ne!(
            b1.cid, b2.cid,
            "consecutive random block CIDs should differ"
        );
    }
}
