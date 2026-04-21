use thiserror::Error;

/// Errors produced by the transaction data parser.
#[derive(Error, Debug)]
pub enum TxDataParseError {
    #[error("empty transaction data")]
    Empty,

    #[error("invalid hex encoding: {0}")]
    HexError(String),

    #[error("data too short: expected at least {expected} bytes, got {got}")]
    TooShort { expected: usize, got: usize },

    #[error("unknown transaction type: 0x{0:02x}")]
    UnknownType(u8),
}

/// Structured data extracted from a raw IPC transaction payload.
#[derive(Debug, Clone, PartialEq)]
pub struct TxData {
    /// Raw payload bytes.
    pub raw: Vec<u8>,
    /// Transaction type byte (EIP-2718 envelope prefix), if present.
    pub tx_type: Option<u8>,
    /// The inner payload after stripping the type prefix (if any).
    pub payload: Vec<u8>,
}

/// Parse a raw transaction byte slice as received from the IPC layer.
///
/// Supports EIP-2718 typed transactions (type prefix ≤ `0x7f`) as well as
/// legacy RLP-encoded transactions (first byte ≥ `0xc0`).  The parsed
/// `TxData` always contains the original `raw` bytes for round-trip
/// fidelity, plus the type tag and stripped payload.
pub fn parse_tx_data(raw: Vec<u8>) -> Result<TxData, TxDataParseError> {
    if raw.is_empty() {
        return Err(TxDataParseError::Empty);
    }

    let first = raw[0];

    // EIP-2718: typed transaction envelopes have a type byte in [0x00, 0x7f].
    // Legacy transactions are RLP lists; their first byte is always ≥ 0xc0.
    if first <= 0x7f {
        // Typed transaction: first byte is the type, remainder is the payload.
        if raw.len() < 2 {
            return Err(TxDataParseError::TooShort {
                expected: 2,
                got: raw.len(),
            });
        }
        Ok(TxData {
            tx_type: Some(first),
            payload: raw[1..].to_vec(),
            raw,
        })
    } else if first >= 0xc0 {
        // Legacy RLP-encoded transaction: no type prefix.
        Ok(TxData {
            tx_type: None,
            payload: raw.clone(),
            raw,
        })
    } else {
        Err(TxDataParseError::UnknownType(first))
    }
}

/// Parse a hex-encoded transaction string (with or without `0x` prefix).
pub fn parse_tx_data_hex(hex_str: &str) -> Result<TxData, TxDataParseError> {
    let stripped = hex_str.trim_start_matches("0x");
    let bytes = hex::decode(stripped).map_err(|e| TxDataParseError::HexError(e.to_string()))?;
    parse_tx_data(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_returns_error() {
        assert!(parse_tx_data(vec![]).is_err());
    }

    #[test]
    fn test_typed_transaction() {
        // EIP-1559 type-2 transaction starts with 0x02
        let raw = vec![0x02u8, 0xf8, 0x6a, 0x01];
        let parsed = parse_tx_data(raw.clone()).unwrap();
        assert_eq!(parsed.tx_type, Some(0x02));
        assert_eq!(parsed.payload, &raw[1..]);
        assert_eq!(parsed.raw, raw);
    }

    #[test]
    fn test_legacy_transaction() {
        // Legacy RLP list; first byte 0xf8 >= 0xc0
        let raw = vec![0xf8u8, 0x6a, 0x01, 0x85];
        let parsed = parse_tx_data(raw.clone()).unwrap();
        assert_eq!(parsed.tx_type, None);
        assert_eq!(parsed.payload, raw);
    }

    #[test]
    fn test_hex_parsing() {
        let raw = vec![0x02u8, 0xab, 0xcd];
        let hex = format!("0x{}", hex::encode(&raw));
        let parsed = parse_tx_data_hex(&hex).unwrap();
        assert_eq!(parsed.raw, raw);
    }
}
