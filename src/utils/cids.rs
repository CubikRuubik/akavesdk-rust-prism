use std::str::FromStr;

use cid::{
    multihash::{Code, MultihashDigest},
    Cid, Version,
};

use crate::types::sdk_types::AkaveError;

// CIDv1, dag-pb (0x70), sha2-256 (0x12), 32-byte digest length
const CID_PREFIX: [u8; 4] = [1, 112, 18, 32];

pub fn verify_raw(cid_str: &str, data: &[u8]) -> Result<(), AkaveError> {
    let c = Cid::from_str(cid_str)
        .map_err(|e| AkaveError::BlockError(format!("failed to decode CID '{}': {}", cid_str, e)))?;
    verify(c, data)
}

pub fn verify(c: Cid, data: &[u8]) -> Result<(), AkaveError> {
    let codec = c.codec();
    let hash_code = c.hash().code();
    let digest = Code::try_from(hash_code)
        .map_err(|e| AkaveError::BlockError(format!("unsupported hash code {}: {}", hash_code, e)))?
        .digest(data);
    let calculated = if c.version() == Version::V0 {
        Cid::new_v0(digest)
            .map_err(|e| AkaveError::BlockError(format!("CIDv0 construction failed: {}", e)))?
    } else {
        Cid::new_v1(codec, digest)
    };
    if calculated != c {
        return Err(AkaveError::BlockError(format!(
            "CID mismatch: provided {}, calculated {}",
            c, calculated
        )));
    }
    Ok(())
}

pub fn from_byte_array_cid(data: [u8; 32]) -> Result<Cid, AkaveError> {
    let mut cid_bytes = Vec::with_capacity(36);
    cid_bytes.extend_from_slice(&CID_PREFIX);
    cid_bytes.extend_from_slice(&data);
    Cid::try_from(cid_bytes.as_slice())
        .map_err(|e| AkaveError::BlockError(format!("from_byte_array_cid failed: {}", e)))
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use cid::{
        multihash::{Code, MultihashDigest},
        Cid,
    };

    use super::*;

    #[test]
    fn test_verify_raw_valid_cid_v1() {
        let data = b"hello world";
        let digest = Code::Sha2_256.digest(data);
        let c = Cid::new_v1(0x70, digest);
        verify_raw(&c.to_string(), data).expect("should verify valid CIDv1");
    }

    #[test]
    fn test_verify_raw_cid_mismatch() {
        let data = b"hello world";
        let digest = Code::Sha2_256.digest(data);
        let c = Cid::new_v1(0x70, digest);
        let err = verify_raw(&c.to_string(), b"wrong data").unwrap_err();
        assert!(
            err.to_string().contains("CID mismatch"),
            "unexpected error: {}",
            err
        );
    }

    #[test]
    fn test_verify_raw_invalid_cid_format() {
        let err = verify_raw("invalid-cid", b"data").unwrap_err();
        assert!(
            err.to_string().contains("failed to decode"),
            "unexpected error: {}",
            err
        );
    }

    #[test]
    fn test_verify_raw_empty_data() {
        let data = b"";
        let digest = Code::Sha2_256.digest(data);
        let c = Cid::new_v1(0x70, digest);
        verify_raw(&c.to_string(), data).expect("empty data should verify");
    }

    #[test]
    fn test_from_byte_array_cid_roundtrip() {
        let data = b"test block data";
        let digest = Code::Sha2_256.digest(data);
        let original = Cid::new_v1(0x70, digest);

        let bytes = original.to_bytes();
        assert!(bytes.len() >= 36, "CID bytes must be at least 36 bytes");
        let mut hash_bytes = [0u8; 32];
        hash_bytes.copy_from_slice(&bytes[4..36]);

        let reconstructed = from_byte_array_cid(hash_bytes).expect("should reconstruct CID");
        assert_eq!(reconstructed, original);
        assert_eq!(reconstructed.version(), cid::Version::V1);
        assert_eq!(reconstructed.codec(), 0x70);
    }
}
