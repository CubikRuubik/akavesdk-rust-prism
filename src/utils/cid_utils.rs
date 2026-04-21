use cid::{
    multihash::{Code, MultihashDigest},
    Cid,
};

use crate::utils::dag::DAG_PROTOBUF;

/// Create a CID v1 with DAG-PB codec from the SHA-256 digest of `data`.
pub fn cid_from_data(data: &[u8]) -> Cid {
    let hash = Code::Sha2_256.digest(data);
    Cid::new_v1(DAG_PROTOBUF, hash)
}

/// Extract the raw 32-byte SHA-256 digest from a DAG-PB CID.
///
/// The first 4 bytes of the serialised CID encode the version and codec
/// (varint) as well as the multihash function and digest length.  For the
/// SHA-256 / DAG-PB combination used throughout the SDK those 4 bytes are
/// always present, so `bytes[4..36]` yields the 32-byte digest.
///
/// Returns `None` if the CID is shorter than expected.
pub fn cid_to_hash_bytes(cid: &Cid) -> Option<[u8; 32]> {
    let bytes = cid.to_bytes();
    if bytes.len() >= 36 {
        let mut result = [0u8; 32];
        result.copy_from_slice(&bytes[4..36]);
        Some(result)
    } else {
        None
    }
}

/// Parse a CID from its string representation.
pub fn parse_cid(s: &str) -> Result<Cid, cid::Error> {
    s.parse()
}

/// Return `true` if both CIDs refer to the same content.
pub fn cids_equal(a: &Cid, b: &Cid) -> bool {
    a == b
}

/// Encode a raw byte slice as a CID string (base-32 lower, the default for v1).
pub fn bytes_to_cid_string(data: &[u8]) -> String {
    cid_from_data(data).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cid_from_data_roundtrip() {
        let data = b"hello akave";
        let cid = cid_from_data(data);
        assert_eq!(cid.codec(), DAG_PROTOBUF);
        let hash_bytes = cid_to_hash_bytes(&cid);
        assert!(hash_bytes.is_some());
    }

    #[test]
    fn test_cids_equal() {
        let data = b"same data";
        let a = cid_from_data(data);
        let b = cid_from_data(data);
        assert!(cids_equal(&a, &b));
    }

    #[test]
    fn test_parse_cid_valid() {
        let cid = cid_from_data(b"test");
        let s = cid.to_string();
        let parsed = parse_cid(&s).expect("should parse valid CID string");
        assert!(cids_equal(&cid, &parsed));
    }

    #[test]
    fn test_parse_cid_invalid() {
        assert!(parse_cid("not-a-cid").is_err());
    }
}
