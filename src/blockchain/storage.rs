// Standard library imports

use std::sync::Arc;

// External crate imports (general)
use web3::{
    contract::{Contract, Options},
    types::{TransactionReceipt, H160, U256},
    Error,
};

// Internal imports
use super::ipc_types::{
    BlockPeersResult, BucketIndexResult, BucketListResult, BucketResponse, FileIndexResult,
    FileResponse, FillChunkBlockArgs,
};
use crate::{
    blockchain::provider::{BlockchainProvider, ProviderError},
    log_debug, log_error, log_info,
    types::BucketId,
};

// Target-specific imports
#[cfg(target_arch = "wasm32")]
mod wasm_imports {
    pub use web3::transports::eip_1193::Eip1193;
}

#[cfg(target_arch = "wasm32")]
use wasm_imports::*;

#[cfg(not(target_arch = "wasm32"))]
mod native_imports {

    pub use web3::{signing::Key, transports::http::Http};
}

#[cfg(not(target_arch = "wasm32"))]
use native_imports::*;

// Target-specific type definitions
#[cfg(target_arch = "wasm32")]
type ProviderType = Eip1193;

#[cfg(not(target_arch = "wasm32"))]
type ProviderType = Http;

// Constants
const CREATE_BUCKET: &str = "createBucket";
const DELETE_BUCKET: &str = "deleteBucket";
const GET_BUCKET_BY_NAME: &str = "getBucketByName";
const GET_BUCKET_INDEX_BY_NAME: &str = "getBucketIndexByName";
const ADD_FILE_CHUNK: &str = "addFileChunk";
const ADD_FILE_CHUNKS: &str = "addFileChunks";
const COMMIT_FILE: &str = "commitFile";
const CREATE_FILE: &str = "createFile";
const DELETE_FILE: &str = "deleteFile";
const GET_FILE_INDEX_BY_NAME: &str = "getFileIndexById";
const GET_FILE_BY_NAME: &str = "getFileByName";
const IS_FILE_FILLED: &str = "isFileFilled";
const FILL_CHUNK_BLOCKS: &str = "fillChunkBlocks";
const GET_BLOCK_PEERS_OF_CHUNK: &str = "getBlockPeersOfChunk";
const GET_BUCKETS_BY_IDS_WITH_FILES: &str = "getBucketsByIdsWithFiles";

#[derive(Clone)]
pub struct FileStorageContract {
    pub client: Arc<BlockchainProvider>,
    pub contract: Contract<ProviderType>,
}

impl FileStorageContract {
    pub fn new(
        client: Arc<BlockchainProvider>,
        access_address: &str,
    ) -> Result<FileStorageContract, Error> {
        log_debug!(
            "Initializing BlockchainProvider with access address: {}",
            access_address
        );

        let storage_address = access_address
            .parse::<H160>()
            .map_err(|e| Error::Decoder(format!("Invalid contract address: {}", e)))?;

        let akave_storage = Contract::from_json(
            client.web3_provider.eth(),
            storage_address,
            include_bytes!("storage.json"),
        )
        .map_err(|e| Error::Decoder(format!("Failed to create contract instance: {}", e)))?;

        log_info!("Akave contract address: 0x{:x}", akave_storage.address());

        Ok(Self {
            client,
            contract: akave_storage,
        })
    }

    pub async fn create_file(
        &self,
        bucket_id: BucketId,
        file_name: String,
    ) -> Result<TransactionReceipt, ProviderError> {
        let file_name_clone = file_name.clone();
        log_debug!(
            "Creating file: {} in bucket: {}",
            file_name_clone,
            bucket_id
        );
        let result = self
            .client
            .call_contract_with_confirmations(
                &self.contract,
                CREATE_FILE,
                (bucket_id.to_bytes(), file_name),
                None,
            )
            .await;
        match &result {
            Ok(_) => log_info!("File created successfully: {}", file_name_clone),
            Err(e) => log_error!("Failed to create file: {}", e),
        }
        result
    }

    pub async fn is_file_filled(&self, file_id: [u8; 32]) -> Result<bool, ProviderError> {
        let result: bool = self
            .contract
            .query(IS_FILE_FILLED, file_id, None, Options::default(), None)
            .await
            .map_err(ProviderError::ContractCallError)?;
        Ok(result)
    }

    pub async fn commit_file(
        &self,
        bucket_id: BucketId,
        file_name: String,
        encode_size: U256,
        actual_size: U256,
        root_cid: Vec<u8>,
    ) -> Result<TransactionReceipt, ProviderError> {
        let file_name_clone = file_name.clone();
        log_debug!(
            "Committing file: {} in bucket: {}, encoded size: {}, actual size: {}",
            file_name_clone,
            bucket_id,
            encode_size,
            actual_size
        );
        log_info!(
            "Committing file: {} in bucket: {}, encoded size: {}, actual size: {}",
            file_name_clone,
            bucket_id,
            encode_size,
            actual_size
        );
        let result = self
            .client
            .call_contract_with_confirmations(
                &self.contract,
                COMMIT_FILE,
                (
                    bucket_id.to_bytes(),
                    file_name,
                    encode_size,
                    actual_size,
                    root_cid,
                ),
                None,
            )
            .await;
        match &result {
            Ok(_) => log_info!("File committed successfully: {}", file_name_clone),
            Err(e) => log_error!("Failed to commit file: {}", e),
        }
        result
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn add_file_chunk(
        &self,
        root_cid: Vec<u8>,
        bucket_id: BucketId,
        file_name: String,
        size: U256,
        cids: Vec<[u8; 32]>,
        sizes: Vec<U256>,
        index: U256,
    ) -> Result<TransactionReceipt, ProviderError> {
        let file_name_clone = file_name.clone();
        log_debug!(
            "Adding file chunk for file: {} in bucket: {}",
            file_name_clone,
            bucket_id
        );
        let result = self
            .client
            .call_contract_with_confirmations(
                &self.contract,
                ADD_FILE_CHUNK,
                (
                    root_cid,
                    bucket_id.to_bytes(),
                    file_name,
                    size,
                    cids,
                    sizes,
                    index,
                ),
                None,
            )
            .await;
        match &result {
            Ok(_) => log_info!(
                "File chunk added successfully for file: {}",
                file_name_clone
            ),
            Err(e) => log_error!("Failed to add file chunk: {}", e),
        }
        result
    }

    /// Adds multiple file chunks in a single batched transaction.
    #[allow(clippy::too_many_arguments)]
    pub async fn add_file_chunks(
        &self,
        cids: Vec<Vec<u8>>,
        bucket_id: BucketId,
        file_name: String,
        encoded_chunk_sizes: Vec<U256>,
        chunk_blocks_cids: Vec<Vec<[u8; 32]>>,
        chunk_block_sizes: Vec<Vec<U256>>,
        starting_chunk_index: U256,
    ) -> Result<TransactionReceipt, ProviderError> {
        let file_name_clone = file_name.clone();
        log_debug!(
            "Adding {} file chunks (batch) for file: {}",
            cids.len(),
            file_name_clone
        );
        let result = self
            .client
            .call_contract_with_confirmations(
                &self.contract,
                ADD_FILE_CHUNKS,
                (
                    cids,
                    bucket_id.to_bytes(),
                    file_name,
                    encoded_chunk_sizes,
                    chunk_blocks_cids,
                    chunk_block_sizes,
                    starting_chunk_index,
                ),
                None,
            )
            .await;
        match &result {
            Ok(_) => log_info!(
                "File chunks batch added successfully for file: {}",
                file_name_clone
            ),
            Err(e) => log_error!("Failed to add file chunks batch: {}", e),
        }
        result
    }

    pub async fn create_bucket(
        &self,
        bucket_name: String,
    ) -> Result<TransactionReceipt, ProviderError> {
        let bucket_name_clone = bucket_name.clone();
        log_debug!("Creating bucket: {}", bucket_name_clone);
        let result = self
            .client
            .call_contract_with_confirmations(&self.contract, CREATE_BUCKET, (bucket_name,), None)
            .await;
        match &result {
            Ok(_) => log_info!("Bucket created successfully: {}", bucket_name_clone),
            Err(e) => log_error!("Failed to create bucket: {}", e),
        }
        result
    }

    pub async fn delete_bucket(
        &self,
        bucket_id: BucketId,
        bucket_name: String,
        bucket_idx: U256,
    ) -> Result<TransactionReceipt, ProviderError> {
        let bucket_name_clone = bucket_name.clone();
        log_debug!(
            "Deleting bucket: {} with ID: {}",
            bucket_name_clone,
            bucket_id
        );
        let result = self
            .client
            .call_contract_with_confirmations(
                &self.contract,
                DELETE_BUCKET,
                (bucket_id.to_bytes(), bucket_name, bucket_idx),
                None,
            )
            .await;
        match &result {
            Ok(_) => log_info!("Bucket deleted successfully: {}", bucket_name_clone),
            Err(e) => log_error!("Failed to delete bucket: {}", e),
        }
        result
    }

    pub async fn get_bucket_by_name(
        &self,
        bucket_name: String,
    ) -> Result<BucketResponse, ProviderError> {
        let bucket_name_clone = bucket_name.clone();
        log_debug!("Getting bucket by name: {}", bucket_name_clone);
        let address = self.client.get_address().await?;
        let result: BucketResponse = self
            .contract
            .query(
                GET_BUCKET_BY_NAME,
                (bucket_name, address, U256::zero(), U256::zero()),
                address,
                Options::default(),
                None,
            )
            .await
            .map_err(ProviderError::ContractCallError)?;
        Ok(result)
    }

    pub async fn get_bucket_index_by_name(
        &self,
        bucket_name: String,
    ) -> Result<BucketIndexResult, ProviderError> {
        let bucket_name_clone = bucket_name.clone();
        log_debug!("Getting bucket index for name: {}", bucket_name_clone);
        let address = self.client.get_address().await?;
        let result: BucketIndexResult = self
            .contract
            .query(
                GET_BUCKET_INDEX_BY_NAME,
                (bucket_name, address),
                address,
                Options::default(),
                None,
            )
            .await
            .map_err(ProviderError::ContractCallError)?;
        Ok(result)
    }

    pub async fn delete_file(
        &self,
        file_name: String,
        bucket_name: String,
        bucket_id: BucketId,
    ) -> Result<TransactionReceipt, ProviderError> {
        let file_name_clone = file_name.clone();
        log_debug!(
            "Deleting file: {} from bucket: {}",
            file_name_clone,
            bucket_id
        );

        let owner = self.client.get_address().await?;
        let file = self
            .get_file_by_name(bucket_id, file_name.to_string())
            .await?;
        let file_idx = self
            .get_file_index_by_id(bucket_name, file.id.to_vec(), owner)
            .await?;
        if !file_idx.exists {
            return Err(ProviderError::InvalidFileId(0));
        }
        let result = self
            .client
            .call_contract_with_confirmations(
                &self.contract,
                DELETE_FILE,
                (
                    file.id.to_bytes(),
                    bucket_id.to_bytes(),
                    file_name,
                    file_idx.index,
                ),
                None,
            )
            .await;
        match &result {
            Ok(_) => log_info!("File deleted successfully: {}", file_name_clone),
            Err(e) => log_error!("Failed to delete file: {}", e),
        }
        result
    }

    pub async fn get_file_index_by_id(
        &self,
        bucket_name: String,
        file_id: Vec<u8>,
        owner: H160,
    ) -> Result<FileIndexResult, ProviderError> {
        let bucket_name_clone = bucket_name.clone();
        log_debug!("Getting file index for bucket: {}", bucket_name_clone);
        let parsed_id: [u8; 32] = file_id
            .try_into()
            .map_err(|v: Vec<u8>| ProviderError::InvalidFileId(v.len()))?;
        let result: FileIndexResult = self
            .contract
            .query(
                GET_FILE_INDEX_BY_NAME,
                (bucket_name, parsed_id, owner),
                owner,
                Options::default(),
                None,
            )
            .await
            .map_err(ProviderError::ContractCallError)?;
        Ok(result)
    }

    pub async fn get_file_by_name(
        &self,
        bucket_id: BucketId,
        file_name: String,
    ) -> Result<FileResponse, ProviderError> {
        let file_name_clone = file_name.clone();
        log_debug!(
            "Getting file by name: {} from bucket: {}",
            file_name_clone,
            bucket_id
        );
        let address = self.client.get_address().await?;
        let result = self
            .contract
            .query(
                GET_FILE_BY_NAME,
                (bucket_id.to_bytes(), file_name),
                address,
                Options::default(),
                None,
            )
            .await
            .map_err(ProviderError::ContractCallError)?;
        Ok(result)
    }

    /// Submits multiple block-fill proofs in a single batched transaction.
    ///
    /// Each element of `args` corresponds to one `IStorage.FillChunkBlockArgs` struct.
    /// `blockIndex` inside each arg is encoded as `uint8` — values must fit in one byte.
    pub async fn fill_chunk_blocks(
        &self,
        args: Vec<FillChunkBlockArgs>,
    ) -> Result<TransactionReceipt, ProviderError> {
        log_debug!("Filling {} chunk blocks (batch)", args.len());
        let result = self
            .client
            .call_contract_with_confirmations(&self.contract, FILL_CHUNK_BLOCKS, (args,), None)
            .await;
        match &result {
            Ok(_) => log_info!("Chunk blocks batch filled successfully"),
            Err(e) => log_error!("Failed to fill chunk blocks batch: {}", e),
        }
        result
    }

    /// Returns the peer node IDs for the blocks of a given chunk, identified by their CIDs.
    pub async fn get_block_peers_of_chunk(
        &self,
        block_cids: Vec<[u8; 32]>,
        file_id: [u8; 32],
        chunk_index: U256,
    ) -> Result<Vec<[u8; 32]>, ProviderError> {
        log_debug!("Getting block peers for chunk (file_id={:?})", file_id);
        let address = self.client.get_address().await?;
        let result: BlockPeersResult = self
            .contract
            .query(
                GET_BLOCK_PEERS_OF_CHUNK,
                (block_cids, file_id, chunk_index),
                address,
                Options::default(),
                None,
            )
            .await
            .map_err(ProviderError::ContractCallError)?;
        Ok(result.0)
    }

    /// Returns buckets (with their file lists) for the given bucket IDs, with pagination.
    pub async fn get_buckets_by_ids_with_files(
        &self,
        ids: Vec<[u8; 32]>,
        bucket_offset: U256,
        bucket_limit: U256,
        file_offset: U256,
        file_limit: U256,
    ) -> Result<Vec<BucketResponse>, ProviderError> {
        log_debug!("Getting buckets by IDs with files ({} IDs)", ids.len());
        let address = self.client.get_address().await?;
        let result: BucketListResult = self
            .contract
            .query(
                GET_BUCKETS_BY_IDS_WITH_FILES,
                (ids, bucket_offset, bucket_limit, file_offset, file_limit),
                address,
                Options::default(),
                None,
            )
            .await
            .map_err(ProviderError::ContractCallError)?;
        Ok(result.0)
    }
}
