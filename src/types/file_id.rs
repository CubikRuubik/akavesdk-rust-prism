use std::fmt;

use serde::{Deserialize, Serialize};

/// A strongly-typed file identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(tsify_next::Tsify))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(target_arch = "wasm32", tsify(into_wasm_abi, from_wasm_abi))]
pub struct FileId([u8; 32]);

impl FileId {
    /// Create a new FileId from a 32-byte array
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Create a FileId from a slice, returning None if the slice is not 32 bytes
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

impl From<[u8; 32]> for FileId {
    fn from(bytes: [u8; 32]) -> Self {
        Self::new(bytes)
    }
}

impl AsRef<[u8]> for FileId {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Display for FileId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_id_creation() {
        let bytes = [42u8; 32];
        let file_id = FileId::new(bytes);
        assert_eq!(file_id.to_bytes(), bytes);
    }

    #[test]
    fn test_file_id_from_slice() {
        let bytes = [55u8; 32];
        let file_id = FileId::from_slice(&bytes).unwrap();
        assert_eq!(file_id.to_bytes(), bytes);
    }

    #[test]
    fn test_file_id_display() {
        let bytes = [0xAAu8; 32];
        let file_id = FileId::new(bytes);
        let display = format!("{}", file_id);
        assert_eq!(display.len(), 64);
        assert!(display.starts_with("aa"));
    }
}
