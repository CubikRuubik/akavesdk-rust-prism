use crate::utils::timestamp::timestamp_serde_direct;
use cid::Cid;
use prost_types::Timestamp;
use std::str::FromStr;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(tsify_next::Tsify))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(target_arch = "wasm32", tsify(into_wasm_abi, from_wasm_abi))]
pub(crate) struct IpcFileListItem {
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
pub(crate) struct IpcFileList {
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

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(tsify_next::Tsify))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(target_arch = "wasm32", tsify(into_wasm_abi, from_wasm_abi))]
pub(crate) struct FileBlockUpload {
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
pub(crate) struct IpcFileChunkUpload {
    pub index: usize,
    #[serde(serialize_with = "serialize_cid", deserialize_with = "deserialize_cid")]
    pub chunk_cid: Cid,
    pub actual_size: usize,
    pub raw_data_size: usize,
    pub proto_node_size: usize,
    pub blocks: Vec<FileBlockUpload>,
    pub bucket_id: [u8; 32],
    pub file_name: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(tsify_next::Tsify))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(target_arch = "wasm32", tsify(into_wasm_abi, from_wasm_abi))]
pub(crate) struct AkaveBlockData {
    pub permit: String,
    pub node_address: String,
    pub node_id: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(tsify_next::Tsify))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(target_arch = "wasm32", tsify(into_wasm_abi, from_wasm_abi))]
pub(crate) struct FileBlockDownload {
    pub cid: String,
    pub data: Vec<u8>,
    pub akave: AkaveBlockData,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(tsify_next::Tsify))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(target_arch = "wasm32", tsify(into_wasm_abi, from_wasm_abi))]
pub(crate) struct FileChunkDownload {
    pub cid: String,
    pub index: i64,
    pub encoded_size: i64,
    pub size: i64,
    pub blocks: Vec<FileBlockDownload>,
}
