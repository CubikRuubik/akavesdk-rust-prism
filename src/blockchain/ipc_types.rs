use web3::{
    contract::tokens::Detokenize,
    ethabi::Token,
    types::{Address, U256},
};

/// Arguments for the `fillChunkBlock` / `fillChunkBlocks` contract functions.
#[derive(Debug, Clone)]
pub struct FillChunkBlockArgs {
    pub block_cid: [u8; 32],
    pub node_id: [u8; 32],
    pub bucket_id: [u8; 32],
    pub chunk_index: U256,
    pub nonce: U256,
    /// Solidity `uint8` — must fit in one byte.
    pub block_index: u8,
    pub file_name: String,
    pub signature: Vec<u8>,
    pub deadline: U256,
}

impl FillChunkBlockArgs {
    /// Encode this value as the Solidity `FillChunkBlockArgs` tuple token.
    pub fn into_tuple_token(self) -> Token {
        Token::Tuple(vec![
            Token::FixedBytes(self.block_cid.to_vec()),
            Token::FixedBytes(self.node_id.to_vec()),
            Token::FixedBytes(self.bucket_id.to_vec()),
            Token::Uint(self.chunk_index),
            Token::Uint(self.nonce),
            Token::Uint(U256::from(self.block_index)),
            Token::String(self.file_name),
            Token::Bytes(self.signature),
            Token::Uint(self.deadline),
        ])
    }
}

/// Wraps `Vec<BucketResponse>` so it can be decoded from a contract query
/// that returns a Solidity `Bucket[]` (i.e. `Token::Array` of `Token::Tuple`s).
pub struct BucketList(pub Vec<BucketResponse>);

impl Detokenize for BucketList {
    fn from_tokens(tokens: Vec<Token>) -> Result<Self, web3::contract::Error> {
        match tokens.as_slice() {
            [Token::Array(items)] => {
                let buckets = items
                    .iter()
                    .map(|t| BucketResponse::from_tokens(vec![t.clone()]))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(BucketList(buckets))
            }
            _ => Err(web3::contract::Error::InterfaceUnsupported),
        }
    }
}

#[derive(Debug)]
pub struct BucketIndexResult {
    pub index: U256,
    pub exists: bool,
}

impl Detokenize for BucketIndexResult {
    fn from_tokens(tokens: Vec<Token>) -> Result<Self, web3::contract::Error> {
        if let [Token::Uint(index), Token::Bool(exists)] = tokens.as_slice() {
            Ok(Self {
                index: *index,
                exists: *exists,
            })
        } else {
            Err(web3::contract::Error::InterfaceUnsupported)
        }
    }
}

#[derive(Debug)]
pub struct FileIndexResult {
    pub index: U256,
    pub exists: bool,
}

impl Detokenize for FileIndexResult {
    fn from_tokens(tokens: Vec<Token>) -> Result<Self, web3::contract::Error> {
        if let [Token::Uint(index), Token::Bool(exists)] = tokens.as_slice() {
            Ok(Self {
                index: *index,
                exists: *exists,
            })
        } else {
            Err(web3::contract::Error::InterfaceUnsupported)
        }
    }
}

use crate::types::{BucketId, FileId};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(tsify_next::Tsify))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(target_arch = "wasm32", tsify(into_wasm_abi, from_wasm_abi))]
pub(crate) struct DeleteBucketResponse {}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(tsify_next::Tsify))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(target_arch = "wasm32", tsify(into_wasm_abi, from_wasm_abi))]
pub struct BucketResponse {
    pub id: BucketId,
    pub name: String,
    pub created_at: U256,
    pub owner: Address,
    pub files: Vec<FileId>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(tsify_next::Tsify))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(target_arch = "wasm32", tsify(into_wasm_abi, from_wasm_abi))]
pub(crate) struct IStorageChunk {
    chunk_cids: Vec<Vec<u8>>,
    chunk_size: Vec<U256>,
    fulfilled_blocks: Vec<u32>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(tsify_next::Tsify))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(target_arch = "wasm32", tsify(into_wasm_abi, from_wasm_abi))]
pub struct FileResponse {
    pub id: FileId,
    pub file_cid: Vec<u8>,
    pub bucket_id: BucketId,
    pub name: String,
    encoded_size: U256,
    created_at: U256,
    actual_size: U256,
    chunks: IStorageChunk,
}

impl Detokenize for BucketResponse {
    fn from_tokens(tokens: Vec<Token>) -> Result<Self, web3::contract::Error> {
        if let [Token::Tuple(tokens)] = tokens.as_slice() {
            if let [Token::FixedBytes(id), Token::String(name), Token::Uint(created_at), Token::Address(owner), Token::Array(files)] =
                tokens.as_slice()
            {
                let mut id_bytes = [0u8; 32];
                id_bytes.copy_from_slice(id);
                let files = files
                    .iter()
                    .map(|token| {
                        if let Token::FixedBytes(bytes) = token {
                            let mut file_bytes = [0u8; 32];
                            file_bytes.copy_from_slice(bytes);
                            Ok(FileId::from(file_bytes))
                        } else {
                            Err(web3::contract::Error::InterfaceUnsupported)
                        }
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(BucketResponse {
                    id: BucketId::from(id_bytes),
                    name: name.clone(),
                    created_at: *created_at,
                    owner: *owner,
                    files,
                })
            } else {
                Err(web3::contract::Error::InterfaceUnsupported)
            }
        } else {
            Err(web3::contract::Error::InterfaceUnsupported)
        }
    }
}

impl Detokenize for FileResponse {
    fn from_tokens(tokens: Vec<Token>) -> Result<Self, web3::contract::Error> {
        if let [Token::Tuple(tokens)] = tokens.as_slice() {
            if let [Token::FixedBytes(id), Token::Bytes(file_cid), Token::FixedBytes(bucket_id), Token::String(name), Token::Uint(encoded_size), Token::Uint(created_at), Token::Uint(actual_size), Token::Tuple(chunks_tokens)] =
                tokens.as_slice()
            {
                let mut id_bytes = [0u8; 32];
                id_bytes.copy_from_slice(id);

                let mut bucket_id_bytes = [0u8; 32];
                bucket_id_bytes.copy_from_slice(bucket_id);

                let chunks = IStorageChunk::from_tokens(vec![Token::Tuple(chunks_tokens.clone())])?;

                Ok(FileResponse {
                    id: FileId::from(id_bytes),
                    file_cid: file_cid.clone(),
                    bucket_id: BucketId::from(bucket_id_bytes),
                    name: name.clone(),
                    encoded_size: *encoded_size,
                    created_at: *created_at,
                    actual_size: *actual_size,
                    chunks,
                })
            } else {
                Err(web3::contract::Error::InterfaceUnsupported)
            }
        } else {
            Err(web3::contract::Error::InterfaceUnsupported)
        }
    }
}

impl Detokenize for IStorageChunk {
    fn from_tokens(tokens: Vec<Token>) -> Result<Self, web3::contract::Error> {
        if let [Token::Tuple(tokens)] = tokens.as_slice() {
            if let [Token::Array(chunk_cids), Token::Array(chunk_sizes), Token::Array(fulfilled_blocks_tokens)] =
                tokens.as_slice()
            {
                let chunk_cids = chunk_cids
                    .iter()
                    .map(|token| {
                        if let Token::Bytes(cid) = token {
                            Ok(cid.clone())
                        } else {
                            Err(web3::contract::Error::InterfaceUnsupported)
                        }
                    })
                    .collect::<Result<Vec<_>, _>>()?;

                let chunk_size = chunk_sizes
                    .iter()
                    .map(|token| {
                        if let Token::Uint(size) = token {
                            Ok(*size)
                        } else {
                            Err(web3::contract::Error::InterfaceUnsupported)
                        }
                    })
                    .collect::<Result<Vec<_>, _>>()?;

                let fulfilled_blocks = fulfilled_blocks_tokens
                    .iter()
                    .map(|token| {
                        if let Token::Uint(v) = token {
                            Ok(v.as_u32())
                        } else {
                            Err(web3::contract::Error::InterfaceUnsupported)
                        }
                    })
                    .collect::<Result<Vec<_>, _>>()?;

                Ok(IStorageChunk {
                    chunk_cids,
                    chunk_size,
                    fulfilled_blocks,
                })
            } else {
                Err(web3::contract::Error::InterfaceUnsupported)
            }
        } else {
            Err(web3::contract::Error::InterfaceUnsupported)
        }
    }
}
