use std::fmt;
use std::str::FromStr;
use thiserror::Error;

/// A lightweight implementation of PeerId without libp2p dependency
/// This implementation only handles parsing base58-encoded peer IDs
/// and converting them to bytes for use in signatures
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerId {
    bytes: Vec<u8>,
}

#[derive(Error, Debug)]
pub enum PeerIdError {
    #[error("invalid base58 encoding: {0}")]
    InvalidBase58(String),
    #[error("invalid peer id format")]
    InvalidFormat,
}

impl PeerId {
    /// Get the raw bytes of the peer ID
    pub fn to_bytes(&self) -> Vec<u8> {
        self.bytes.clone()
    }
}

impl FromStr for PeerId {
    type Err = PeerIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Decode base58 string
        let decoded = bs58::decode(s)
            .into_vec()
            .map_err(|e| PeerIdError::InvalidBase58(e.to_string()))?;

        // Basic validation - peer IDs should have at least a few bytes
        if decoded.len() < 2 {
            return Err(PeerIdError::InvalidFormat);
        }

        Ok(PeerId { bytes: decoded })
    }
}

impl fmt::Display for PeerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", bs58::encode(&self.bytes).into_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peer_id_from_str() {
        let peer_id_str = "12D3KooWBPkG43Vjb3Rp2PFHYRgKkhAaMZCXAMRVb3M7PrQN2fC5";
        let peer_id = PeerId::from_str(peer_id_str).unwrap();

        // Should be able to convert to bytes
        let bytes = peer_id.to_bytes();
        assert!(!bytes.is_empty());

        // Should be able to display back as string
        let display_str = peer_id.to_string();
        assert_eq!(display_str, peer_id_str);
    }

    #[test]
    fn test_peer_id_invalid() {
        // Invalid base58
        assert!(PeerId::from_str("invalid!@#$").is_err());

        // Empty string
        assert!(PeerId::from_str("").is_err());
    }
}
