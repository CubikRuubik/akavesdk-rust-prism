use std::str::FromStr;

use cid::Cid;
use prost_types::Timestamp;
use thiserror::Error;
use tokio::task::JoinError;

use crate::{types::BucketId, utils::timestamp::timestamp_serde_direct};

#[derive(Error, Debug)]
pub enum AkaveError {
    #[error("blockchain error")]
    BlockchainError(#[source] web3::Error),

    #[error("block error: {0}")]
    BlockError(String),

    #[error("chunk error: {0}")]
    ChunkError(String),

    #[error("grpc error")]
    GrpcError(#[source] Box<dyn std::error::Error + Send + Sync>),

    #[error("file error: {0}")]
    FileError(String),

    #[error("file operation error during {operation} for file '{file_name}': {message}")]
    FileOperationError {
        operation: String,
        file_name: String,
        message: String,
    },

    #[error("encryption error")]
    EncryptionError(#[from] crate::utils::encryption::EncryptionError),

    #[error("erasure coding error")]
    ErasureCodeError(#[from] crate::utils::erasure::ErasureCodeError),

    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("configuration error: {0}")]
    ConfigurationError(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("already exists: {0}")]
    AlreadyExists(String),

    #[error("permission denied: {0}")]
    PermissionDenied(String),

    #[error("internal error: {0}")]
    InternalError(String),

    #[error("provider error")]
    ProviderError(#[from] crate::blockchain::provider::ProviderError),

    #[error("bucket error: {0}")]
    BucketError(String),

    #[error("channel error: {0}")]
    ChannelError(String),

    #[error("account error: {0}")]
    AccountError(String),

    #[error("io error")]
    IoError(#[from] std::io::Error),

    #[error("serialization error")]
    SerializationError(#[source] Box<dyn std::error::Error + Send + Sync>),

    #[error("thread join error")]
    ThreadJoinError(#[from] JoinError),

    /// Returned when a byte offset or chunk index falls outside the file bounds.
    #[error("offset out of bounds: {0}")]
    OffsetOutOfBounds(String),

    /// Returned for well-known node-side error codes that do not map to a more
    /// specific variant.
    #[error("node error [{code}]: {message}")]
    NodeError { code: String, message: String },
}

/// Maps a gRPC status message to a well-typed [`AkaveError`].
///
/// The Akave node encodes machine-readable error codes in the gRPC status
/// message field.  This function recognises the codes introduced in v0.3.0
/// and returns the most specific variant available.  Unknown codes fall back
/// to a [`AkaveError::NodeError`] that preserves the original message.
pub fn map_grpc_error_message(raw_message: &str) -> AkaveError {
    let code = raw_message.trim();
    match code {
        "OffsetOutOfBounds" => AkaveError::OffsetOutOfBounds("offset is out of bounds".to_string()),
        "NonceAlreadyUsed" => AkaveError::NodeError {
            code: code.to_string(),
            message: "nonce has already been used".to_string(),
        },
        "NotSignedByBucketOwner" => AkaveError::NodeError {
            code: code.to_string(),
            message: "not signed by bucket owner".to_string(),
        },
        "InvalidBlocksAmount" => AkaveError::NodeError {
            code: code.to_string(),
            message: "invalid number of blocks".to_string(),
        },
        "InvalidBlockIndex" => AkaveError::NodeError {
            code: code.to_string(),
            message: "invalid block index".to_string(),
        },
        "LastChunkDuplicate" => AkaveError::NodeError {
            code: code.to_string(),
            message: "last chunk is a duplicate".to_string(),
        },
        "FileNotExists" => AkaveError::NotFound("file does not exist".to_string()),
        "ECDSAInvalidSignature" => AkaveError::NodeError {
            code: code.to_string(),
            message: "ECDSA signature is invalid".to_string(),
        },
        "ECDSAInvalidSignatureS" => AkaveError::NodeError {
            code: code.to_string(),
            message: "ECDSA signature has invalid S component".to_string(),
        },
        "ECDSAInvalidSignatureR" => AkaveError::NodeError {
            code: code.to_string(),
            message: "ECDSA signature has invalid R component".to_string(),
        },
        "ECDSAInvalidSignatureV" => AkaveError::NodeError {
            code: code.to_string(),
            message: "ECDSA signature has invalid V component".to_string(),
        },
        _ => AkaveError::NodeError {
            code: code.to_string(),
            message: code.to_string(),
        },
    }
}

/// Suppresses [`AkaveError::OffsetOutOfBounds`] errors, converting them to
/// `Ok(None)`.  All other outcomes are forwarded unchanged.
///
/// This simplifies pagination code that reads until the file is exhausted:
///
/// ```rust,ignore
/// while let Some(chunk) = ignore_offset_error(sdk.next_chunk(...))? {
///     process(chunk);
/// }
/// ```
pub fn ignore_offset_error<T>(result: Result<T, AkaveError>) -> Result<Option<T>, AkaveError> {
    match result {
        Ok(v) => Ok(Some(v)),
        Err(AkaveError::OffsetOutOfBounds(_)) => Ok(None),
        Err(e) => Err(e),
    }
}

impl From<web3::Error> for AkaveError {
    fn from(err: web3::Error) -> Self {
        // Check if this is a configuration error
        match &err {
            web3::Error::Decoder(msg) if msg.contains("AKAVE_PRIVATE_KEY") => {
                return AkaveError::ConfigurationError(msg.clone());
            }
            _ => {}
        }
        AkaveError::BlockchainError(err)
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(tsify_next::Tsify))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(target_arch = "wasm32", tsify(into_wasm_abi, from_wasm_abi))]
pub struct IpcFileListItem {
    pub root_cid: String,
    pub name: String,
    pub encoded_size: i64,
    #[serde(with = "timestamp_serde_direct")]
    pub created_at: Timestamp,
}

/// Rich block information returned by [`crate::AkaveSdk::latest_block_number`].
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(tsify_next::Tsify))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(target_arch = "wasm32", tsify(into_wasm_abi, from_wasm_abi))]
pub struct BlockInfo {
    pub number: u64,
    /// Unix timestamp in seconds.
    pub time: i64,
    /// Hex-encoded block hash (with 0x prefix).
    pub hash: String,
}

// Create a wrapper type for the Vec
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(tsify_next::Tsify))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(target_arch = "wasm32", tsify(into_wasm_abi, from_wasm_abi))]
pub struct IpcFileList {
    pub files: Vec<IpcFileListItem>,
}

// Define the serialization function
fn serialize_cid<S>(cid: &Cid, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&cid.to_string())
}

// Define the deserialization function
fn deserialize_cid<'de, D>(deserializer: D) -> Result<Cid, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let cid_str = <String as serde::Deserialize>::deserialize(deserializer)?;
    Cid::from_str(&cid_str).map_err(serde::de::Error::custom)
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
#[cfg_attr(target_arch = "wasm32", derive(tsify_next::Tsify))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(target_arch = "wasm32", tsify(into_wasm_abi, from_wasm_abi))]
pub struct FileBlockUpload {
    #[serde(serialize_with = "serialize_cid", deserialize_with = "deserialize_cid")]
    pub cid: Cid,
    pub data: Vec<u8>,
    pub permit: String,
    pub node_address: String,
    pub node_id: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(tsify_next::Tsify))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(target_arch = "wasm32", tsify(into_wasm_abi, from_wasm_abi))]
pub struct IpcFileChunkUpload {
    pub index: usize,
    #[serde(serialize_with = "serialize_cid", deserialize_with = "deserialize_cid")]
    pub chunk_cid: Cid,
    pub actual_size: usize,
    pub raw_data_size: usize,
    pub proto_node_size: usize,
    pub blocks: Vec<FileBlockUpload>,
    pub bucket_id: BucketId,
    pub file_name: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(tsify_next::Tsify))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(target_arch = "wasm32", tsify(into_wasm_abi, from_wasm_abi))]
pub struct AkaveBlockData {
    pub permit: String,
    pub node_address: String,
    pub node_id: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(tsify_next::Tsify))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(target_arch = "wasm32", tsify(into_wasm_abi, from_wasm_abi))]
pub struct FileBlockDownload {
    pub cid: String,
    pub data: Vec<u8>,
    pub akave: AkaveBlockData,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(tsify_next::Tsify))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(target_arch = "wasm32", tsify(into_wasm_abi, from_wasm_abi))]
pub struct FileChunkDownload {
    pub cid: String,
    pub index: i64,
    pub encoded_size: i64,
    pub size: i64,
    pub blocks: Vec<FileBlockDownload>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(tsify_next::Tsify))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(target_arch = "wasm32", tsify(into_wasm_abi, from_wasm_abi))]
pub struct BucketListResponse {
    pub buckets: Vec<BucketListItem>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(tsify_next::Tsify))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(target_arch = "wasm32", tsify(into_wasm_abi, from_wasm_abi))]
pub struct BucketListItem {
    pub id: String,
    pub name: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(tsify_next::Tsify))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(target_arch = "wasm32", tsify(into_wasm_abi, from_wasm_abi))]
pub struct BucketViewResponse {
    pub id: String,
    pub name: String,
    pub created_at: i64,
    pub file_count: i64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(tsify_next::Tsify))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(target_arch = "wasm32", tsify(into_wasm_abi, from_wasm_abi))]
pub struct FileListResponse {
    pub files: Vec<FileListItem>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(tsify_next::Tsify))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(target_arch = "wasm32", tsify(into_wasm_abi, from_wasm_abi))]
pub struct FileListItem {
    pub root_cid: String,
    pub created_at: i64,
    pub encoded_size: i64,
    pub name: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(tsify_next::Tsify))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(target_arch = "wasm32", tsify(into_wasm_abi, from_wasm_abi))]
pub struct FileViewResponse {
    pub root_cid: String,
    pub created_at: i64,
    pub encoded_size: i64,
    pub name: String,
    pub bucket_name: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(tsify_next::Tsify))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(target_arch = "wasm32", tsify(into_wasm_abi, from_wasm_abi))]
pub struct FileDownloadResponse {
    pub chunks: Vec<FileChunk>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(tsify_next::Tsify))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(target_arch = "wasm32", tsify(into_wasm_abi, from_wasm_abi))]
pub struct FileChunk {
    pub cid: String,
    pub size: i64,
    pub encoded_size: i64,
    pub blocks: Vec<FileBlock>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(tsify_next::Tsify))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(target_arch = "wasm32", tsify(into_wasm_abi, from_wasm_abi))]
pub struct FileBlock {
    pub cid: String,
    pub size: i64,
    pub node_id: String,
    pub node_address: String,
    pub permit: String,
}
