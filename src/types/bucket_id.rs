use serde::{Deserialize, Serialize};
use std::fmt;

/// A strongly-typed bucket identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(tsify_next::Tsify))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(target_arch = "wasm32", tsify(into_wasm_abi, from_wasm_abi))]
pub struct BucketId([u8; 32]);

impl BucketId {
    /// Create a new BucketId from a 32-byte array
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Create a BucketId from a slice, returning None if the slice is not 32 bytes
    pub fn from_slice(slice: &[u8]) -> Option<Self> {
        if slice.len() == 32 {
            let mut bytes = [0u8; 32];
            bytes.copy_from_slice(slice);
            Some(Self(bytes))
        } else {
            None
        }
    }

    /// Get the underlying bytes as a slice
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Get the underlying bytes as an array
    pub fn to_bytes(&self) -> [u8; 32] {
        self.0
    }

    /// Convert to a Vec<u8>
    pub fn to_vec(&self) -> Vec<u8> {
        self.0.to_vec()
    }

    /// Get a hex string representation
    pub fn to_hex(&self) -> String {
        hex::encode(&self.0)
    }
}

impl From<[u8; 32]> for BucketId {
    fn from(bytes: [u8; 32]) -> Self {
        Self::new(bytes)
    }
}

impl AsRef<[u8]> for BucketId {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Display for BucketId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}
