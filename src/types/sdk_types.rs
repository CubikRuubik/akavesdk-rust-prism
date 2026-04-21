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

    /// Returned when a bucket with the same name already exists.
    #[error("bucket already exists: {0}")]
    BucketAlreadyExists(String),

    /// Returned when the bucket data is invalid.
    #[error("bucket invalid: {0}")]
    BucketInvalid(String),

    /// Returned when the bucket owner is invalid.
    #[error("bucket invalid owner: {0}")]
    BucketInvalidOwner(String),

    /// Returned when the bucket does not exist.
    #[error("bucket does not exist: {0}")]
    BucketNonexists(String),

    /// Returned when the bucket is not empty.
    #[error("bucket not empty: {0}")]
    BucketNonempty(String),

    /// Returned when a file with the same name already exists.
    #[error("file already exists: {0}")]
    FileAlreadyExists(String),

    /// Returned when the file data is invalid.
    #[error("file invalid: {0}")]
    FileInvalid(String),

    /// Returned when the file does not exist.
    #[error("file nonexistent: {0}")]
    FileNonexists(String),

    /// Returned when the file is not empty.
    #[error("file not empty: {0}")]
    FileNonempty(String),

    /// Returned when a file name is duplicated.
    #[error("file name duplicate: {0}")]
    FileNameDuplicate(String),

    /// Returned when the file is already fully uploaded.
    #[error("file fully uploaded: {0}")]
    FileFullyUploaded(String),

    /// Returned when a file chunk is duplicated.
    #[error("file chunk duplicate: {0}")]
    FileChunkDuplicate(String),

    /// Returned when the file is not fully filled.
    #[error("file not filled: {0}")]
    FileNotFilled(String),

    /// Returned when a chunk CID mismatch is detected.
    #[error("chunk cid mismatch: {0}")]
    ChunkCIDMismatch(String),

    /// Returned when no storage policy is set.
    #[error("no policy: {0}")]
    NoPolicy(String),
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
        "BucketAlreadyExists" => {
            AkaveError::BucketAlreadyExists("bucket already exists".to_string())
        }
        "BucketInvalid" => AkaveError::BucketInvalid("bucket data is invalid".to_string()),
        "BucketInvalidOwner" => {
            AkaveError::BucketInvalidOwner("bucket owner is invalid".to_string())
        }
        "BucketNonexists" => AkaveError::BucketNonexists("bucket does not exist".to_string()),
        "BucketNonempty" => AkaveError::BucketNonempty("bucket is not empty".to_string()),
        "BucketNotFound" => AkaveError::NotFound("bucket not found".to_string()),
        "FileAlreadyExists" => AkaveError::FileAlreadyExists("file already exists".to_string()),
        "FileInvalid" => AkaveError::FileInvalid("file data is invalid".to_string()),
        "FileNonexists" | "FileNotExists" | "FileDoesNotExist" => {
            AkaveError::NotFound("file does not exist".to_string())
        }
        "FileNonempty" => AkaveError::FileNonempty("file is not empty".to_string()),
        "FileNameDuplicate" => AkaveError::FileNameDuplicate("file name is duplicated".to_string()),
        "FileFullyUploaded" => {
            AkaveError::FileFullyUploaded("file is already fully uploaded".to_string())
        }
        "FileChunkDuplicate" => {
            AkaveError::FileChunkDuplicate("file chunk is duplicated".to_string())
        }
        "FileNotFilled" => AkaveError::FileNotFilled("file is not fully filled".to_string()),
        "ChunkCIDMismatch" => AkaveError::ChunkCIDMismatch("chunk CID mismatch".to_string()),
        "NoPolicy" => AkaveError::NoPolicy("no storage policy is set".to_string()),
        "NonceAlreadyUsed" => AkaveError::NodeError {
            code: code.to_string(),
            message: "nonce has already been used".to_string(),
        },
        "NotSignedByBucketOwner" | "NotBucketOwner" => AkaveError::NodeError {
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
        "BlockAlreadyExists" => AkaveError::NodeError {
            code: code.to_string(),
            message: "block already exists".to_string(),
        },
        "BlockInvalid" => AkaveError::NodeError {
            code: code.to_string(),
            message: "block data is invalid".to_string(),
        },
        "BlockNonexists" => AkaveError::NodeError {
            code: code.to_string(),
            message: "block does not exist".to_string(),
        },
        "BlockAlreadyFilled" => AkaveError::NodeError {
            code: code.to_string(),
            message: "block is already filled".to_string(),
        },
        "InvalidArrayLength" => AkaveError::NodeError {
            code: code.to_string(),
            message: "array length is invalid".to_string(),
        },
        "InvalidFileBlocksCount" => AkaveError::NodeError {
            code: code.to_string(),
            message: "file blocks count is invalid".to_string(),
        },
        "InvalidLastBlockSize" => AkaveError::NodeError {
            code: code.to_string(),
            message: "last block size is invalid".to_string(),
        },
        "InvalidEncodedSize" => AkaveError::NodeError {
            code: code.to_string(),
            message: "encoded size is invalid".to_string(),
        },
        "InvalidFileCID" => AkaveError::NodeError {
            code: code.to_string(),
            message: "file CID is invalid".to_string(),
        },
        "IndexMismatch" => AkaveError::NodeError {
            code: code.to_string(),
            message: "index mismatch".to_string(),
        },
        "ECDSAInvalidSignature" => AkaveError::NodeError {
            code: code.to_string(),
            message: "ECDSA signature is invalid".to_string(),
        },
        "ECDSAInvalidSignatureS" => AkaveError::NodeError {
            code: code.to_string(),
            message: "ECDSA signature has invalid S component".to_string(),
        },
        "ECDSAInvalidSignatureLength" | "ECDSAInvalidSignatureR" => AkaveError::NodeError {
            code: code.to_string(),
            message: "ECDSA signature has invalid length".to_string(),
        },
        "ECDSAInvalidSignatureV" => AkaveError::NodeError {
            code: code.to_string(),
            message: "ECDSA signature has invalid V component".to_string(),
        },
        "AlreadyWhitelisted" => AkaveError::NodeError {
            code: code.to_string(),
            message: "address is already whitelisted".to_string(),
        },
        "InvalidAddress" => AkaveError::NodeError {
            code: code.to_string(),
            message: "invalid address".to_string(),
        },
        "NotWhitelisted" => AkaveError::NodeError {
            code: code.to_string(),
            message: "address is not whitelisted".to_string(),
        },
        "MathOverflowedMulDiv" => AkaveError::NodeError {
            code: code.to_string(),
            message: "math overflow in multiplication or division".to_string(),
        },
        "NotThePolicyOwner" => AkaveError::NodeError {
            code: code.to_string(),
            message: "caller is not the policy owner".to_string(),
        },
        "CloneArgumentsTooLong" => AkaveError::NodeError {
            code: code.to_string(),
            message: "clone arguments are too long".to_string(),
        },
        "Create2EmptyBytecode" => AkaveError::NodeError {
            code: code.to_string(),
            message: "empty bytecode in create2 operation".to_string(),
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
