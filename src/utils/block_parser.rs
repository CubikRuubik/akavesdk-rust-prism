use quick_protobuf::{BytesReader, MessageRead};
use thiserror::Error;

use crate::utils::{dag::DAG_PROTOBUF, pb_data::PbData};

/// Errors produced by the block parser.
#[derive(Error, Debug)]
pub enum BlockParseError {
    #[error("empty block data")]
    Empty,

    #[error("unsupported codec: 0x{0:x}")]
    UnsupportedCodec(u64),

    #[error("protobuf decoding error: {0}")]
    ProtobufError(String),

    #[error("block data field missing")]
    MissingData,
}

/// The parsed payload of a single block received from an IPC node.
#[derive(Debug, Clone)]
pub struct ParsedBlock {
    /// Raw content bytes extracted from the block (codec framing removed).
    pub data: Vec<u8>,
    /// Codec identifier of the source block.
    pub codec: u64,
}

/// Parse raw block bytes received from an IPC node download stream.
///
/// Strips codec-specific framing and returns the inner payload bytes.
/// Supports the `DAG_PROTOBUF` (0x70) and raw (0x55) codecs.
pub fn parse_block(raw: Vec<u8>, codec: u64) -> Result<ParsedBlock, BlockParseError> {
    if raw.is_empty() {
        return Err(BlockParseError::Empty);
    }

    match codec {
        0x55 => Ok(ParsedBlock { data: raw, codec }),
        DAG_PROTOBUF => parse_dag_pb(raw),
        other => Err(BlockParseError::UnsupportedCodec(other)),
    }
}

fn parse_dag_pb(raw: Vec<u8>) -> Result<ParsedBlock, BlockParseError> {
    // DAG-PB blocks are protobuf-encoded PBNode messages.
    // Use the existing PbData MessageRead impl which correctly handles all fields.
    let mut reader = BytesReader::from_bytes(&raw);
    let msg = PbData::from_reader(&mut reader, &raw)
        .map_err(|e| BlockParseError::ProtobufError(e.to_string()))?;

    let data = msg
        .data
        .ok_or(BlockParseError::MissingData)?
        .into_owned()
        .to_vec();
    Ok(ParsedBlock {
        data,
        codec: DAG_PROTOBUF,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_raw_codec_passthrough() {
        let input = vec![1u8, 2, 3, 4];
        let parsed = parse_block(input.clone(), 0x55).unwrap();
        assert_eq!(parsed.data, input);
        assert_eq!(parsed.codec, 0x55);
    }

    #[test]
    fn test_empty_block_error() {
        assert!(parse_block(vec![], 0x55).is_err());
        assert!(parse_block(vec![], DAG_PROTOBUF).is_err());
    }

    #[test]
    fn test_unsupported_codec_error() {
        let result = parse_block(vec![0u8; 10], 0x99);
        assert!(matches!(
            result,
            Err(BlockParseError::UnsupportedCodec(0x99))
        ));
    }
}
