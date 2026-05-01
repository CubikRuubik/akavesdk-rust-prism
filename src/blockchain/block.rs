use web3::types::{Block, H256};

/// Parses a raw Ethereum JSON-RPC block response into a typed Block.
/// A JSON `null` (pending block or block-not-found) returns `Ok(None)`.
pub fn parse_block_from_json(json: &str) -> Result<Option<Block<H256>>, serde_json::Error> {
    serde_json::from_str(json)
}

#[cfg(test)]
mod tests {
    use super::parse_block_from_json;
    use web3::types::U64;

    // Minimal but fully-valid Ethereum block JSON (no transactions, no uncles).
    // All required non-optional fields are present; optional ones are null/absent.
    const VALID_BLOCK_JSON: &str = r#"{
        "hash":             "0xdc0818cf78f21a8e70579cb46a43643f78291264dda342ae31049421c82d21ae",
        "parentHash":       "0xe99e022112df268087ea7eafaf4790497fd21dbeeb6bd7a1721df161a6657a54",
        "sha3Uncles":       "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
        "miner":            "0xbb7b8287f3f0a933474a79eae42cbca977791171",
        "stateRoot":        "0xddc8b0234c2e0cad087c8b389aa7ef01f7d79b2570bccb77ce48648aa61c904d",
        "transactionsRoot": "0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421",
        "receiptsRoot":     "0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421",
        "logsBloom":        null,
        "difficulty":       "0x4ea3f27bc",
        "number":           "0x1b4",
        "gasLimit":         "0x1388",
        "gasUsed":          "0x0",
        "timestamp":        "0x55ba467c",
        "extraData":        "0x",
        "mixHash":          "0x4fffe9ae21f1c9e15207b1f472d5bbdd68c9595d461666602f2be20daf5e7843",
        "nonce":            "0x1c97a0c536168a0e",
        "size":             "0x21d",
        "totalDifficulty":  "0x78ed983323d",
        "transactions":     [],
        "uncles":           []
    }"#;

    #[test]
    fn test_parse_block_from_json_valid_block() {
        let block = parse_block_from_json(VALID_BLOCK_JSON)
            .expect("valid block JSON should parse without error")
            .expect("valid block JSON should not be null");

        assert_eq!(block.number, Some(U64::from(0x1b4_u64)));
        assert!(block.hash.is_some(), "hash should be present");
    }

    #[test]
    fn test_parse_block_from_json_null_block() {
        let result = parse_block_from_json("null");
        assert!(
            result.is_ok(),
            "null block JSON should not be a parse error"
        );
        assert!(result.unwrap().is_none(), "null JSON should yield None");
    }

    #[test]
    fn test_parse_block_from_json_invalid_json() {
        let result = parse_block_from_json("{not valid json}");
        assert!(result.is_err(), "invalid JSON should return an error");
    }

    #[test]
    fn test_parse_block_from_json_empty_transactions_and_uncles() {
        let block = parse_block_from_json(VALID_BLOCK_JSON).unwrap().unwrap();
        assert_eq!(block.transactions.len(), 0, "transactions should be empty");
        assert_eq!(block.uncles.len(), 0, "uncles should be empty");
    }
}
