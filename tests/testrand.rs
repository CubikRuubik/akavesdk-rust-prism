/// Test utility helpers for generating random test data.
///
/// Ported from Go's `private/testrand/testrand.go` at akavesdk-prism@8b66e30.
#[cfg(not(target_arch = "wasm32"))]
pub mod testrand {
    use cid::{
        multihash::{Code, MultihashDigest},
        Cid,
    };

    /// Generates a random IPFS block of the given byte size.
    ///
    /// Fills a buffer with cryptographically random bytes, computes a SHA2-256
    /// multihash over them, wraps the digest in a CIDv1 with the DagProtobuf
    /// codec (`0x70`), and returns the raw bytes together with the CID.
    pub fn block(size: usize) -> (Vec<u8>, Cid) {
        let mut data = vec![0u8; size];
        getrandom::getrandom(&mut data).expect("failed to generate random bytes");
        let digest = Code::Sha2_256.digest(&data);
        let c = Cid::new_v1(0x70, digest);
        (data, c)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_block_size() {
            let (data, _cid) = block(256);
            assert_eq!(data.len(), 256);
        }

        #[test]
        fn test_block_cid_v1_dag_protobuf() {
            let (_data, c) = block(64);
            assert_eq!(c.version(), cid::Version::V1);
            assert_eq!(c.codec(), 0x70);
        }

        #[test]
        fn test_block_unique() {
            let (a, _) = block(32);
            let (b, _) = block(32);
            // Cryptographically random data should differ with overwhelming probability.
            assert_ne!(a, b);
        }
    }
}
