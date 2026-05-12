use std::str::FromStr;

use cid::Cid;
use prost_types::Timestamp;
use thiserror::Error;
use tokio::task::JoinError;

use crate::{types::BucketId, utils::timestamp::timestamp_serde_direct};

#[derive(Error, Debug)]
pub enum AkaveError {
    #[error("blockchain error: {0}")]
    BlockchainError(#[source] web3::Error),

    #[error("block error: {0}")]
    BlockError(String),

    #[error("chunk error: {0}")]
    ChunkError(String),

    #[error("grpc error: {0}")]
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

    #[error("transient error: {0}")]
    Transient(String),

    #[error("provider error: {0}")]
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
    pub encoded_size: usize,
    pub blocks: Vec<FileBlockUpload>,
    pub bucket_id: BucketId,
    pub file_name: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(tsify_next::Tsify))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(target_arch = "wasm32", tsify(into_wasm_abi, from_wasm_abi))]
pub struct FileBlockDownload {
    pub cid: String,
    pub data: Vec<u8>,
    pub permit: String,
    pub node_address: String,
    pub node_id: String,
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
    pub actual_size: i64,
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
    pub actual_size: i64,
    pub encoded_size: i64,
    pub name: String,
    pub bucket_name: String,
    pub is_public: bool,
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

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(tsify_next::Tsify))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(target_arch = "wasm32", tsify(into_wasm_abi, from_wasm_abi))]
pub struct PDPBlockData {
    pub url: String,
    pub offset: i64,
    pub size: i64,
    pub data_set_id: u64,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(tsify_next::Tsify))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(target_arch = "wasm32", tsify(into_wasm_abi, from_wasm_abi))]
pub struct ArchivalBlock {
    pub cid: String,
    pub size: i64,
    pub pdp_data: Option<PDPBlockData>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(tsify_next::Tsify))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(target_arch = "wasm32", tsify(into_wasm_abi, from_wasm_abi))]
pub struct ArchivalChunk {
    pub cid: String,
    pub encoded_size: i64,
    pub size: i64,
    pub index: i64,
    pub blocks: Vec<ArchivalBlock>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(tsify_next::Tsify))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(target_arch = "wasm32", tsify(into_wasm_abi, from_wasm_abi))]
pub struct ArchivalMetadata {
    pub bucket_name: String,
    pub name: String,
    pub chunks: Vec<ArchivalChunk>,
}
