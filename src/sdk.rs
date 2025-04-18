// Proto module definition
pub(crate) mod ipcnodeapi {
    tonic::include_proto!("ipcnodeapi");
}

// Standard library imports
use std::{borrow::Cow, io::Write, str::FromStr};
use futures::Stream;
use libp2p::PeerId;

// External crate imports (general)
use crate::{blockchain::eip712_utils::create_block_eip712_data, utils::splitter::Splitter};
use alloy::hex;
use bytesize::{ByteSize, MB};
use cid::{
    multihash::{Code, MultihashDigest},
    Cid,
};
use quick_protobuf::BytesReader;
use web3::types::{TransactionReceipt, U256};

// Proto-related imports
use ipcnodeapi::{
    ipc_chunk::Block,
    ipc_node_api_client::IpcNodeApiClient, ConnectionParamsRequest, IpcBucketListRequest,
    IpcBucketViewRequest, IpcChunk, IpcFileBlockData,
    IpcFileDownloadBlockRequest, IpcFileDownloadChunkCreateRequest, IpcFileDownloadCreateRequest,
    IpcFileListRequest, IpcFileUploadChunkCreateRequest,
    IpcFileViewRequest,
};

// Internal crate imports
use crate::utils::dag::{ChunkDag, DAG_PROTOBUF}; // Removed unused RAW import
use crate::utils::encryption::Encryption;
use crate::utils::pb_data::PbData;
use crate::{
    blockchain::ipc_types::BucketResponse,
    sdk_types::{
        AkaveBlockData, FileBlockDownload, FileChunkDownload, IpcFileChunkUpload
    },
};
use crate::{blockchain::provider::BlockchainProvider, utils};
use crate::{log_debug, log_error, log_info};
use crate::sdk_types::{
    AkaveError, BucketListResponse, BucketViewResponse, FileListResponse, FileViewResponse,
    FileDownloadResponse, BucketListItem, FileListItem, FileChunk,
};

use crate::blockchain::eip712_types::{Domain, TypedData};
use web3::types::{Address};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

// Target-specific imports and types
#[cfg(target_arch = "wasm32")]
mod wasm_imports {
    pub use tonic_web_wasm_client::Client as GrpcWebClient;
    pub use wasm_bindgen_file_reader::WebSysFile as File;
}

#[cfg(target_arch = "wasm32")]
use wasm_imports::*;

#[cfg(not(target_arch = "wasm32"))]
mod native_imports {
    pub use std::fs::File;
    pub use tonic::transport::{Channel, ClientTlsConfig};
}

#[cfg(not(target_arch = "wasm32"))]
use native_imports::*;

// Target-specific type definitions
#[cfg(target_arch = "wasm32")]
type ClientTransport = GrpcWebClient;

#[cfg(not(target_arch = "wasm32"))]
type ClientTransport = Channel;

// Constants
const ENCRYPTION_OVERHEAD: usize = 32;
const BLOCK_SIZE: usize = MB as usize;
const MIN_BUCKET_NAME_LENGTH: usize = 3;
const MIN_FILE_SIZE: usize = 127;
const MAX_BLOCKS_IN_CHUNK: usize = 32;
const BLOCK_PART_SIZE: usize = ByteSize::kb(128).as_u64() as usize;

/// Represents the Akave SDK client
/// Akave SDK should support both WASM (gRPC-Web) and native gRPC
pub struct AkaveSDK {
    client: IpcNodeApiClient<ClientTransport>,
    storage: BlockchainProvider,
    erasure_code: Option<utils::erasure::ErasureCode>,
    default_encryption_key: Option<String>,
    block_size: usize,
    min_bucket_name_length: usize,
    max_blocks_in_chunk: usize,
    block_part_size: usize,
}

/// Builder for AkaveSDK
pub struct AkaveSDKBuilder {
    server_address: String,
    data_blocks: Option<usize>,
    parity_blocks: Option<usize>,
    default_encryption_key: Option<String>,
    block_size: usize,
    min_bucket_name_length: usize,
    max_blocks_in_chunk: usize,
    block_part_size: usize,
}

impl AkaveSDKBuilder {
    /// Create a new AkaveSDKBuilder with the given server address
    pub fn new(server_address: &str) -> Self {
        Self {
            server_address: server_address.to_string(),
            data_blocks: None,
            parity_blocks: None,
            default_encryption_key: None,
            block_size: BLOCK_SIZE,
            min_bucket_name_length: MIN_BUCKET_NAME_LENGTH,
            max_blocks_in_chunk: MAX_BLOCKS_IN_CHUNK,
            block_part_size: BLOCK_PART_SIZE,
        }
    }

    /// Set erasure coding parameters
    pub fn with_erasure_coding(mut self, data_blocks: usize, parity_blocks: usize) -> Self {
        self.data_blocks = Some(data_blocks);
        self.parity_blocks = Some(parity_blocks);
        self
    }

    /// Set default encryption key
    pub fn with_default_encryption(mut self, encryption_key: &str) -> Self {
        self.default_encryption_key = Some(encryption_key.to_string());
        self
    }

    /// Set block size
    pub fn with_block_size(mut self, block_size: usize) -> Self {
        self.block_size = block_size;
        self
    }

    /// Set minimum bucket name length
    pub fn with_min_bucket_length(mut self, min_bucket_name_length: usize) -> Self {
        self.min_bucket_name_length = min_bucket_name_length;
        self
    }

    /// Set maximum blocks in chunk
    pub fn with_max_blocks_in_chunk(mut self, max_blocks_in_chunk: usize) -> Self {
        self.max_blocks_in_chunk = max_blocks_in_chunk;
        self
    }

    /// Set block part size
    pub fn with_block_part_size(mut self, block_part_size: usize) -> Self {
        self.block_part_size = block_part_size;
        self
    }

    /// Build the AkaveSDK instance
    pub async fn build(self) -> Result<AkaveSDK, Box<dyn std::error::Error>> {
        let erasure_code = match (self.data_blocks, self.parity_blocks) {
            (Some(data), Some(parity)) => Some(utils::erasure::ErasureCode::new(data, parity)?),
            _ => None,
        };

        AkaveSDK::new_with_params(
            &self.server_address,
            erasure_code,
            self.default_encryption_key,
            self.block_size,
            self.min_bucket_name_length,
            self.max_blocks_in_chunk,
            self.block_part_size,
        )
        .await
    }
}

impl AkaveSDK {
    /// Creates a new AkaveSDK instance with default parameters
    pub async fn new(server_address: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Self::new_with_params(
            server_address,
            None,
            None,
            BLOCK_SIZE,
            MIN_BUCKET_NAME_LENGTH,
            MAX_BLOCKS_IN_CHUNK,
            BLOCK_PART_SIZE,
        )
        .await
    }

    /// Creates a new AkaveSDK instance with custom parameters
    async fn new_with_params(
        server_address: &str,
        erasure_code: Option<utils::erasure::ErasureCode>,
        default_encryption_key: Option<String>,
        block_size: usize,
        min_bucket_name_length: usize,
        max_blocks_in_chunk: usize,
        block_part_size: usize,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        log_info!(
            "Initializing AkaveSDK with server address: {}",
            server_address
        );

        #[cfg(target_arch = "wasm32")]
        {
            let grpc_web_client = ClientTransport::new(server_address.into());
            let mut client = IpcNodeApiClient::new(grpc_web_client);
            log_debug!("Requesting connection parameters...");
            let connection_params = client
                .connection_params(ConnectionParamsRequest {})
                .await?
                .into_inner();
            log_debug!("Creating blockchain provider...");
            let storage = BlockchainProvider::new(
                &connection_params.dial_uri,
                &connection_params.storage_address,
                None,
            );
            log_info!("AkaveSDK initialized successfully");
            Ok(Self {
                client,
                storage: storage.unwrap(),
                erasure_code,
                default_encryption_key,
                block_size,
                min_bucket_name_length,
                max_blocks_in_chunk,
                block_part_size,
            })
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let tls_config = ClientTlsConfig::new().with_native_roots();
            let channel = Channel::from_shared(server_address.to_string())?
                .tls_config(tls_config)?
                .connect()
                .await?;

            let mut client = IpcNodeApiClient::new(channel)
                .max_decoding_message_size(usize::MAX)
                .max_encoding_message_size(usize::MAX);
            log_debug!("Requesting connection parameters...");
            let connection_params = client
                .connection_params(ConnectionParamsRequest {})
                .await?
                .into_inner();

            log_debug!("Creating blockchain provider...");
            let storage = BlockchainProvider::new(
                &connection_params.dial_uri,
                &connection_params.storage_address,
                None,
            );

            log_info!("AkaveSDK initialized successfully");
            Ok(Self {
                client,
                storage: storage.unwrap(),
                erasure_code,
                default_encryption_key,
                block_size,
                min_bucket_name_length,
                max_blocks_in_chunk,
                block_part_size,
            })
        }
    }

    /// List all buckets
    pub async fn list_buckets(
        &mut self,
        address: &str,
    ) -> Result<BucketListResponse, AkaveError> {
        log_debug!("Listing buckets for address: {}", address);
        let request = IpcBucketListRequest {
            address: address.to_string(),
        };
        let response = self.client.bucket_list(request).await.map_err(|e| AkaveError::GrpcError(e.to_string()))?.into_inner();
        
        let buckets: Vec<BucketListItem> = response.buckets.into_iter()
            .map(|bucket| BucketListItem {
                id: bucket.name.clone(), // Using name as ID since that's what's available
                name: bucket.name,
                created_at: bucket.created_at.map(|ts| ts.seconds).unwrap_or(0),
            })
            .collect();

        log_info!("Found {} buckets", buckets.len());
        Ok(BucketListResponse { buckets })
    }

    /// View a bucket
    pub async fn view_bucket(
        &mut self,
        address: &str,
        bucket_name: &str,
    ) -> Result<BucketViewResponse, AkaveError> {
        log_debug!("Viewing bucket: {} for address: {}", bucket_name, address);
        let request = IpcBucketViewRequest {
            name: bucket_name.to_string(),
            address: address.to_string(),
        };
        let response = self.client.bucket_view(request).await.map_err(|e| AkaveError::GrpcError(e.to_string()))?.into_inner();
        
        let bucket = BucketViewResponse {
            id: response.id,
            name: response.name,
            created_at: response.created_at.map(|ts| ts.seconds).unwrap_or(0),
            file_count: 0, // This field is not available in the gRPC response
        };

        log_info!("Retrieved bucket details for: {}", bucket_name);
        Ok(bucket)
    }

    /// List files in a bucket
    pub async fn list_files(
        &mut self,
        address: &str,
        bucket_name: &str,
    ) -> Result<FileListResponse, AkaveError> {
        log_debug!(
            "Listing files in bucket: {} for address: {}",
            bucket_name,
            address
        );
        let request = IpcFileListRequest {
            bucket_name: bucket_name.to_string(),
            address: address.to_string(),
        };
        let response = self.client.file_list(request).await.map_err(|e| AkaveError::GrpcError(e.to_string()))?.into_inner();
        
        let files: Vec<FileListItem> = response.list.into_iter()
            .map(|file| FileListItem {
                root_cid: file.root_cid,
                created_at: file.created_at.map(|ts| ts.seconds).unwrap_or(0),
                encoded_size: file.encoded_size,
                name: file.name,
            })
            .collect();

        log_info!(
            "Found {} files in bucket: {}",
            files.len(),
            bucket_name
        );
        Ok(FileListResponse { files })
    }

    /// View file information
    pub async fn view_file_info(
        &mut self,
        address: &str,
        bucket_name: &str,
        file_name: &str,
    ) -> Result<FileViewResponse, AkaveError> {
        log_debug!(
            "Viewing file info: {} in bucket: {} for address: {}",
            file_name,
            bucket_name,
            address
        );
        let request = IpcFileViewRequest {
            bucket_name: bucket_name.to_string(),
            file_name: file_name.to_string(),
            address: address.to_string(),
        };
        let response = self.client.file_view(request).await.map_err(|e| AkaveError::GrpcError(e.to_string()))?.into_inner();
        
        let file = FileViewResponse {
            root_cid: response.root_cid,
            created_at: response.created_at.map(|ts| ts.seconds).unwrap_or(0),
            encoded_size: response.encoded_size,
            name: file_name.to_string(),
            bucket_name: bucket_name.to_string(),
        };

        log_info!(
            "Retrieved file details for: {} in bucket: {}",
            file_name,
            bucket_name
        );
        Ok(file)
    }

    // Create a new bucket
    pub async fn create_bucket(
        &mut self,
        bucket_name: &str,
    ) -> Result<BucketResponse, Box<dyn std::error::Error>> {
        log_debug!("Creating bucket: {}", bucket_name);
        if bucket_name.len() < self.min_bucket_name_length {
            let error_msg = format!(
                "Bucket name must have at least {} characters",
                self.min_bucket_name_length
            );
            log_error!("{}", error_msg);
            return Err(error_msg)?;
        }
        log_info!("Create bucket request to storage: {}", bucket_name);
        self.storage.create_bucket(bucket_name.into()).await?;
        log_info!("Bucket created successfully: {}", bucket_name);
        self.storage.get_bucket_by_name(bucket_name.into()).await
    }

    // Delete an existing bucket
    pub async fn delete_bucket(
        &mut self,
        address: &str,
        bucket_name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        log_debug!("Deleting bucket: {} for address: {}", bucket_name, address);
        let bucket = self.view_bucket(address, bucket_name).await?;
        let bucket_id = hex::decode(bucket.id.clone())?;
        let bucket_idx = self
            .storage
            .get_bucket_index_by_name(bucket_name.to_string())
            .await?;

        self.storage
            .delete_bucket(bucket_id, bucket_name.into(), bucket_idx)
            .await?;
        log_info!("Bucket deleted successfully: {}", bucket_name);
        Ok(())
    }

    // Delete an existing file
    pub async fn delete_file(
        &mut self,
        address: &str,
        bucket_name: &str,
        file_name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        log_debug!(
            "Deleting file: {} from bucket: {} for address: {}",
            file_name,
            bucket_name,
            address
        );
        let bucket = self.view_bucket(address, bucket_name).await?;
        let bucket_id = hex::decode(bucket.id.clone())?;
        self.storage
            .delete_file(file_name.to_string(), bucket_id)
            .await?;
        log_info!(
            "File deleted successfully: {} from bucket: {}",
            file_name,
            bucket_name
        );
        Ok(())
    }

    async fn create_file_upload(
        &mut self,
        bucket_id: Vec<u8>,
        file_name: &str,
    ) -> Result<TransactionReceipt, AkaveError> {
        self.storage
            .create_file(bucket_id, file_name.to_string())
            .await
            .map_err(|e| AkaveError::BlockchainError(e.to_string()))
    }

    pub async fn upload_file(
        &mut self,
        bucket_name: &str,
        file_name: &str,
        file: File,
        passwd: Option<&str>,
    ) -> Result<TransactionReceipt, AkaveError> {
        log_debug!(
            "Starting file upload: {} to bucket: {}",
            file_name,
            bucket_name
        );
        if bucket_name.is_empty() {
            return Err(AkaveError::InvalidInput("Empty bucket name".to_string()));
        }

        let bucket = self
            .storage
            .get_bucket_by_name(bucket_name.to_string())
            .await
            .map_err(|e| AkaveError::BlockchainError(e.to_string()))?;

        let resp = self.create_file_upload(bucket.id.to_vec(), file_name).await
            .map_err(|e| AkaveError::BlockchainError(e.to_string()))?;

        log_info!("File created successfully: {}", file_name);

        let info = format!("{}/{}", bucket_name, file_name);

        // Use default encryption if provided and no password was specified
        let password = match (passwd, &self.default_encryption_key) {
            (Some(p), _) => Some(p),
            (None, Some(default_key)) => Some(default_key.as_str()),
            _ => None,
        };

        let encryption = match password {
            Some(key) => {
                log_debug!("Setting up encryption");
                Some(Encryption::new(key.as_bytes(), info.as_bytes())
                    .map_err(|e| AkaveError::EncryptionError(e.to_string()))?)
            }
            None => {
                log_debug!("No encryption key provided");
                None
            }
        };

        // Calculate buffer size based on erasure coding settings
        let mut buffer_size = self.block_size * self.max_blocks_in_chunk;
        if let Some(erasure_code) = &self.erasure_code {
            buffer_size = erasure_code.data_blocks * self.block_size;
        }
        log_debug!("Buffer size: {}", buffer_size);

        // Subtract encryption overhead if encryption is enabled
        let encryption_overhead = if encryption.is_some() {
            ENCRYPTION_OVERHEAD
        } else {
            0
        };
        buffer_size -= encryption_overhead;

        let chunk_size = buffer_size as u64;

        let mut file_size: usize = 0;
        let root_hasher = Code::Sha2_256;
        let mut root_hash = None;
        let mut idx = 0;
        let mut is_empty_file = true;

        // Initialize the splitter with file and chunk_size
        let mut splitter = Splitter::new(file, chunk_size);

        while let Some(chunk_result) = splitter.next() {
            let buffer = match chunk_result {
                Ok(data) => data,
                Err(e) => return Err(AkaveError::FileError(e.to_string())),
            };

            if buffer.is_empty() && is_empty_file {
                return Err(AkaveError::InvalidInput("Empty file".to_string()));
            }

            is_empty_file = false;

            log_debug!("Processing chunk {} for file: {}", idx, file_name);

            // First encrypt if encryption is enabled (matches Go implementation)
            let encrypted_data = match &encryption {
                Some(encryption) => {
                    encryption.encrypt(&buffer[..], format!("block_{}", idx).as_bytes())
                        .map_err(|e| AkaveError::EncryptionError(e.to_string()))?
                }
                None => buffer[..].to_vec().into(),
            };

            // Then apply erasure coding if enabled (matches Go implementation)
            let processed_data = if let Some(erasure_code) = &self.erasure_code {
                erasure_code.encode(&encrypted_data)
                    .map_err(|e| AkaveError::ErasureCodeError(e.to_string()))?
            } else {
                encrypted_data.to_vec()
            };

            let (chunk, _, ipc_chunk) = self
                .create_chunk_upload(idx, processed_data, bucket.id, file_name)
                .await
                .map_err(|e| AkaveError::BlockchainError(e.to_string()))?;
            file_size += chunk.actual_size;
            root_hash = Some(root_hasher.digest(&chunk.chunk_cid.to_bytes()));

            let mut chunks_iter = chunk.blocks.iter().enumerate();
            while let Some((index, block_1mb)) = chunks_iter.next() {
                
                // Generate a nonce based on current time
                let nonce = {
                    let timestamp = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .expect("Time went backwards")
                        .as_micros();
                    U256::from(timestamp)
                };
                let chunk_cid = cid::Cid::from_str(&ipc_chunk.cid).map_err(|e| AkaveError::Internal(e.to_string()))?;
                let node_id = PeerId::from_str(&block_1mb.node_id).map_err(|e| AkaveError::Internal(e.to_string()))?;
                let (data_message, domain, data_types) = create_block_eip712_data(&block_1mb.cid, &chunk_cid, &node_id, self.storage.akave_storage.address(),  ipc_chunk.index,index as i64, nonce).map_err(|e| AkaveError::Internal(e.to_string()))?;
                // Sign the message

                log_debug!("Signing data for chunk {}, block {}", ipc_chunk.index, index);
                let signature = match self.storage.eip712_sign(domain.clone(), data_message.clone(), data_types.clone()).await {
                    Ok(sig) => {
                        // Recover signer from the signature
                        // let recovered_address = crate::blockchain::eip712::recover_signer_address(&sig, &domain, &data_message, &data_types).unwrap();
                        // println!("Recovered address: {}, signature: {}", recovered_address, hex::encode(sig));
                        hex::encode(sig)
                    }
                    Err(e) => {
                        log_error!("Failed to sign data: {}", e);
                        return Err(AkaveError::BlockchainError(format!("Failed to sign data: {}", e)));
                    }
                };
                let mut bytes = [0u8; 32];
                nonce.to_big_endian(&mut bytes);
                
                self.upload_block_segments(
                    block_1mb.data.clone(),
                    bucket.id.to_vec(),
                    file_name.to_string(),
                    block_1mb.cid.to_string(),
                    index as i64,
                    signature,
                    node_id.to_bytes(),
                    block_1mb.node_address.as_str(),
                    bytes.to_vec(),
                    Some(ipc_chunk.clone()),
                ).await
                .map_err(|e| AkaveError::BlockchainError(e.to_string()))?;
            }

            idx += 1;
        }

        let root_cid = Cid::new_v1(DAG_PROTOBUF, root_hash.unwrap());
        let receipt = self
            .storage
            .commit_file(
                bucket.id,
                file_name.to_string(),
                U256::from(file_size),
                root_cid.to_bytes(),
            )
            .await
            .map_err(|e| AkaveError::BlockchainError(e.to_string()))?;

        log_info!(
            "File uploaded successfully: {} to bucket: {}",
            file_name,
            bucket_name
        );
        Ok(receipt)
    }

    async fn create_chunk_upload(
        &mut self,
        index: usize,
        data: Vec<u8>,
        bucket_id: [u8; 32],
        file_name: &str,
    ) -> Result<(IpcFileChunkUpload, TransactionReceipt, IpcChunk), AkaveError> {
        log_debug!(
            "Creating chunk upload for file: {}, chunk index: {}",
            file_name,
            index
        );
        let size = data.len();

        // Calculate block size based on erasure coding settings
        let block_size = if let Some(erasure_code) = &self.erasure_code {
            size / (erasure_code.data_blocks + erasure_code.parity_blocks)
        } else {
            self.block_size
        };

        let chunk_dag = ChunkDag::new(block_size, data);
        let mut dag = chunk_dag.blocks.iter();

        let mut cids: Vec<[u8; 32]> = vec![];
        let mut sizes = vec![];
        let mut chunk_blocks = vec![];

        while let Some(block) = dag.next() {
            let block_cid = block.cid.to_bytes()[4..36]
                .to_vec()
                .try_into()
                .map_err(|e| AkaveError::InvalidInput(format!("Error formatting cid: {:?}", e)))?;
            chunk_blocks.push(Block {
                cid: block.cid.to_string(),
                size: block.data.len() as i64,
            });
            cids.push(block_cid);
            sizes.push(U256::from(size));
        }

        let chunk_cid = chunk_dag.cid;

        let ipc_chunk = IpcChunk {
            cid: chunk_cid.to_string(),
            index: index as i64,
            size: size as i64,
            blocks: chunk_blocks,
        };

        let chunk_create_request = IpcFileUploadChunkCreateRequest {
            chunk: Some(ipc_chunk.clone()),
            bucket_id: bucket_id.to_vec(),
            file_name: file_name.to_string(),
        };

        log_debug!("Requesting chunk upload creation");
        let chunk_create_response = self
            .client
            .file_upload_chunk_create(chunk_create_request)
            .await
            .map_err(|e| AkaveError::GrpcError(e.to_string()))?
            .into_inner();

        let mut blocks = chunk_dag.blocks;
        chunk_create_response
            .blocks
            .iter()
            .enumerate()
            .for_each(|(idx, block)| {
                blocks[idx].node_address = block.node_address.clone();
                blocks[idx].node_id = block.node_id.clone();
                blocks[idx].permit = block.permit.clone();
            });

        log_debug!("Adding file chunk to contract");
        let receipt = self
            .storage
            .add_file_chunk(
                chunk_cid.to_bytes(),
                bucket_id,
                file_name.to_string(),
                size.into(),
                cids,
                sizes,
                index.into(),
            )
            .await
            .map_err(|e| AkaveError::BlockchainError(e.to_string()))?;

        log_debug!(
            "Chunk upload created successfully for file: {}, chunk index: {}",
            file_name,
            index
        );
        Ok((
            IpcFileChunkUpload {
                index,
                chunk_cid,
                actual_size: size,
                raw_data_size: size,
                proto_node_size: size,
                blocks,
                bucket_id,
                file_name: file_name.to_string(),
            },
            receipt,
            ipc_chunk,
        ))
    }

    /// Upload a block in segments, similar to uploadIpcBlockSegments in the Go implementation
    /// 
    /// The function splits the data into smaller segments based on block_part_size
    /// and only includes metadata with the first segment.
    /// 
    /// For WASM environments, it sends requests sequentially.
    /// For native environments, it processes blocks concurrently.
    async fn upload_block_segments(
        &mut self,
        data: Vec<u8>,
        bucket_id: Vec<u8>,
        file_name: String,
        block_cid: String,
        block_index: i64,
        signature: String,
        node_id: Vec<u8>,
        node_address: &str,
        nonce: Vec<u8>,
        chunk: Option<IpcChunk>,
    ) -> Result<(), AkaveError> {
        let data_len = data.len();
        if data_len == 0 {
            return Ok(());
        }

        log_debug!(
            "Uploading block segments. CID: {}, length: {}, block index: {}",
            block_cid,
            data_len,
            block_index
        );

        #[cfg(target_arch = "wasm32")]
        {
            // WASM implementation: sequential upload of segments
            let mut i = 0;
            while i < data_len {
                let mut end = i + self.block_part_size;
                if end > data_len {
                    end = data_len;
                }

                let segment_data = data[i..end].to_vec();

                let block_data = IpcFileBlockData {
                    bucket_id: bucket_id.clone(),
                    data: segment_data,
                    cid: block_cid.clone(),
                    chunk: chunk.clone(),
                    file_name: file_name.clone(),
                    index: block_index,
                    signature: signature.clone(),
                    node_id: node_id.clone(),
                    nonce: nonce.clone(),
                };

                log_debug!("Uploading segment {} for block {}", i/self.block_part_size, block_index);
                log_debug!("Block data: {:#?}", block_data);
                let mut node_client = AkaveSDK::get_client_for_node_address(node_address).await.map_err(|e| AkaveError::GrpcError(e.to_string()))?;
                node_client
                    .file_upload_block_unary(block_data)
                    .await
                    .map_err(|e| AkaveError::GrpcError(e.to_string()))?
                    .into_inner();

                i += self.block_part_size;
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            use futures::stream::{self, StreamExt};

            // Simplified implementation: streaming upload of segments directly
            let block_part_size = self.block_part_size;
            
            // Create a stream that generates block data on demand
            let stream = stream::iter(0..((data_len + block_part_size - 1) / block_part_size))
                .map(move |segment_index| {
                    let start = segment_index * block_part_size;
                    let end = std::cmp::min(start + block_part_size, data_len);
                    let segment_data = data[start..end].to_vec();
                    
                    IpcFileBlockData {
                        bucket_id: bucket_id.clone(),
                        data: segment_data,
                        cid: block_cid.clone(),
                        chunk: chunk.clone(),
                        file_name: file_name.clone(),
                        index: block_index,
                        signature: signature.clone(),
                        node_id: node_id.clone(),
                        nonce: nonce.clone(),
                    }
                });
            
            // Send the stream to the server
            log_debug!("Streaming block segments for block {} to node {}", block_index, node_address);
            let mut node_client = AkaveSDK::get_client_for_node_address(node_address).await.map_err(|e| AkaveError::GrpcError(e.to_string()))?;
            match node_client.file_upload_block(stream).await {
                Ok(response) => {
                    log_debug!("Block upload completed successfully");
                    response.into_inner();
                }
                Err(e) => {
                    log_error!("Error uploading block: {}", e);
                    return Err(AkaveError::GrpcError(e.to_string()));
                }
            }
        }
        
        log_debug!("Block segments uploaded successfully");
        Ok(())
    }

    async fn create_file_download(
        &mut self,
        address: &str,
        bucket_name: &str,
        file_name: &str,
    ) -> Result<FileDownloadResponse, AkaveError> {
        log_debug!(
            "Creating file download for: {} in bucket: {} for address: {}",
            file_name,
            bucket_name,
            address
        );
        let request = IpcFileDownloadCreateRequest {
            address: address.to_string(),
            bucket_name: bucket_name.to_string(),
            file_name: file_name.to_string(),
        };
        let response = self
            .client
            .file_download_create(request)
            .await
            .map_err(|e| AkaveError::GrpcError(e.to_string()))?
            .into_inner();

        let chunks = response.chunks.into_iter()
            .map(|chunk| FileChunk {
                cid: chunk.cid,
                size: chunk.size,
                encoded_size: chunk.encoded_size,
                blocks: vec![], // Initialize with empty blocks since they're not available in the response
            })
            .collect();

        log_info!("File download created successfully for: {}", file_name);
        Ok(FileDownloadResponse { chunks })
    }

    pub async fn download_file(
        &mut self,
        address: &str,
        bucket_name: &str,
        file_name: &str,
        passwd: Option<&str>,
        destination_path: &str,
    ) -> Result<(), AkaveError> {
        log_debug!(
            "Starting file download: {} from bucket: {} for address: {}",
            file_name,
            bucket_name,
            address
        );
        let info = vec![bucket_name, file_name].join("/");

        // Use default encryption if provided and no password was specified
        let password = match (passwd, &self.default_encryption_key) {
            (Some(p), _) => Some(p),
            (None, Some(default_key)) => Some(default_key.as_str()),
            _ => None,
        };

        let option_encryption = match password {
            Some(key) => {
                log_debug!("Setting up decryption key");
                Some(Encryption::new(key.as_bytes(), info.as_bytes())
                    .map_err(|e| AkaveError::EncryptionError(e.to_string()))?)
            }
            None => {
                log_debug!("No decryption key provided");
                None
            }
        };

        let file_download = self
            .create_file_download(address, bucket_name, file_name)
            .await
            .map_err(|e| AkaveError::GrpcError(e.to_string()))?;

        let mut destination = utils::destination::Destination::new(destination_path, file_name)
            .map_err(|e| AkaveError::FileError(e.to_string()))?;

        let codec = Cid::try_from(file_download.chunks[0].cid.clone())
            .map_err(|e| AkaveError::InvalidInput(e.to_string()))?
            .codec();

        let mut chunk_index = 0;
        for chunk in file_download.chunks {
            log_debug!("Processing chunk {} for file: {}", chunk_index, file_name);
            let chunk_cid = chunk.cid.clone();
            let chunk_size = chunk.size;
            let chunk_download = self
                .create_chunk_download(bucket_name, file_name, address, chunk, chunk_index)
                .await
                .map_err(|e| AkaveError::GrpcError(e.to_string()))?;

            let mut block_index = 0;
            let mut blocks_data = vec![];

            for block in chunk_download.blocks {
                let mut chunk_data = vec![];
                let req = IpcFileDownloadBlockRequest {
                    address: address.to_string(),
                    chunk_cid: chunk_cid.clone(),
                    chunk_index,
                    block_cid: block.cid.clone(),
                    block_index,
                    bucket_name: bucket_name.to_string(),
                    file_name: file_name.to_string(),
                };
                log_debug!("Downloading block {} for chunk {}", block_index, chunk_index);
                let mut node_client = AkaveSDK::get_client_for_node_address(&block.akave.node_address).await.map_err(|e| AkaveError::GrpcError(e.to_string()))?;
                let mut stream = node_client.file_download_block(req).await
                    .map_err(|e| AkaveError::GrpcError(e.to_string()))?
                    .into_inner();

                while let Some(mut message) = stream.message().await
                    .map_err(|e| AkaveError::GrpcError(e.to_string()))? {
                    chunk_data.append(message.data.as_mut());
                }

                let final_data = match codec {
                    0x55 => chunk_data,
                    DAG_PROTOBUF => {
                        let mut reader = BytesReader::from_bytes(&chunk_data);

                        let mut msg = PbData::default();
                        while !reader.is_eof() {
                            match reader.next_tag(&chunk_data) {
                                Ok(18) => {
                                    msg.data = Some(reader.read_bytes(&chunk_data)
                                        .map_err(|e| AkaveError::InvalidInput(e.to_string()))
                                        .map(Cow::Borrowed)?)
                                }
                                Ok(_) => {}
                                Err(_) => return Err(AkaveError::InvalidInput("error decoding message".to_string())),
                            }
                        }

                        msg.data.unwrap().into_owned()
                    }
                    _default => return Err(AkaveError::InvalidInput("Unknown codec for decoding message".to_string())),
                };

                blocks_data.push(final_data);
                block_index += 1;
            }

            // Process the blocks with erasure coding if enabled
            let processed_data = if let Some(erasure_code) = &self.erasure_code {
                // Extract data from blocks (including parity blocks)
                let data = erasure_code.extract_data(blocks_data.clone(), chunk_size as usize)
                    .map_err(|e| AkaveError::ErasureCodeError(e.to_string()))?;
                // Clear blocks_data to remove all blocks including parity blocks
                blocks_data.clear();
                data
            } else {
                // Just concatenate all blocks if no erasure coding
                blocks_data.concat()
            };

            // Decrypt if encryption is enabled
            let decrypted_data = match option_encryption {
                Some(ref encryption) => {
                    log_info!("Decrypting chunk: {}", chunk_index);
                    encryption
                        .decrypt(&processed_data, format!("block_{}", chunk_index).as_bytes())
                        .map_err(|e| AkaveError::EncryptionError(e.to_string()))?
                }
                None => processed_data,
            };

            destination.write(&decrypted_data)
                .map_err(|e| AkaveError::FileError(e.to_string()))?;
            destination.flush()
                .map_err(|e| AkaveError::FileError(e.to_string()))?;
            chunk_index += 1;
        }
        destination.finalize()
            .map_err(|e| AkaveError::FileError(e.to_string()))?;
        log_info!(
            "File downloaded successfully: {} from bucket: {}",
            file_name,
            bucket_name
        );
        Ok(())
    }

    async fn create_chunk_download(
        &mut self,
        bucket_name: &str,
        file_name: &str,
        address: &str,
        chunk: FileChunk,
        index: i64,
    ) -> Result<FileChunkDownload, AkaveError> {
        log_debug!(
            "Creating chunk download for file: {}, chunk index: {}",
            file_name,
            index
        );
        let request = IpcFileDownloadChunkCreateRequest {
            bucket_name: bucket_name.to_string(),
            file_name: file_name.to_string(),
            chunk_cid: chunk.cid.clone(),
            address: address.to_string(),
        };

        let resp = self
            .client
            .file_download_chunk_create(request)
            .await
            .map_err(|e| AkaveError::GrpcError(e.to_string()))?
            .into_inner();
        let mut blocks = vec![];
        for block in resp.blocks {
            blocks.push(FileBlockDownload {
                cid: block.cid,
                data: Vec::new(),
                akave: AkaveBlockData {
                    node_id: block.node_id,
                    permit: block.permit,
                    node_address: block.node_address,
                },
            });
        }

        log_debug!(
            "Chunk download created successfully for file: {}, chunk index: {}",
            file_name,
            index
        );
        Ok(FileChunkDownload {
            cid: chunk.cid,
            index,
            encoded_size: chunk.encoded_size,
            size: chunk.size,
            blocks,
        })
    }

    async fn get_client_for_node_address(node_address: &str) -> Result<IpcNodeApiClient<ClientTransport>, Box<dyn std::error::Error>> {
        #[cfg(target_arch = "wasm32")]
        {
            let grpc_web_client = ClientTransport::new(node_address.into());
            let mut client = IpcNodeApiClient::new(grpc_web_client);
            Ok(client)
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let tls_config = ClientTlsConfig::new().with_native_roots();
            let address = if !node_address.starts_with("http://") {
                format!("http://{}", node_address)
            } else {
                node_address.to_string()
            };
            
            let channel = Channel::from_shared(address.clone())?
                .tls_config(tls_config)?
                .connect()
                .await.map_err(|e| AkaveError::GrpcError(format!("Failed to connect to node {}: {}", address, e)))?;

            let mut client = IpcNodeApiClient::new(channel)
                .max_decoding_message_size(usize::MAX)
                .max_encoding_message_size(usize::MAX);
            Ok(client)
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use crate::sdk::{AkaveSDK, AkaveSDKBuilder};
    use crate::utils::file_size::FileSize;
    use ctor::ctor;
    use env_logger::Builder;
    use log::LevelFilter;
    use pretty_assertions::{assert_eq, assert_ne};
    use std::fs::{self, File};
    use std::path::Path;
    use uuid::Uuid;

    const ADDRESS: &str = "0x7975eD6b732D1A4748516F66216EE703f4856759";
    const FILE_NAME_TO_TEST: &str = "1MB.txt";
    const DOWNLOAD_DESTINATION: &str = "/tmp/akave-tests/";
    const TEST_PASSWORD: &str = "testkey123";
    const TEST_KEY: &str = include_str!("blockchain/user.akvf.key");

    // This runs before any tests are executed
    #[ctor]
    fn init_test_logger() {
        Builder::new()
            .filter_level(LevelFilter::Debug)
            .is_test(true)
            .try_init()
            .ok(); // Ignore errors if logger is already initialized
    }

    #[ctor]
    fn init_test_env() {
        // setting private key for testing
        std::env::set_var("AKAVE_PRIVATE_KEY", TEST_KEY);
    }

    // Get basic SDK with no erasure coding or encryption
    async fn get_sdk() -> Result<AkaveSDK, Box<(dyn std::error::Error + 'static)>> {
        AkaveSDKBuilder::new("http://connect.akave.ai:5500")
            .build()
            .await
    }

    // Get SDK with erasure coding only
    async fn get_sdk_with_erasure() -> Result<AkaveSDK, Box<(dyn std::error::Error + 'static)>> {
        AkaveSDKBuilder::new("http://connect.akave.ai:5500")
            .with_erasure_coding(3, 2)
            .build()
            .await
    }

    // Get SDK with default encryption only
    async fn get_sdk_with_encryption() -> Result<AkaveSDK, Box<(dyn std::error::Error + 'static)>> {
        AkaveSDKBuilder::new("http://connect.akave.ai:5500")
            .with_default_encryption(TEST_PASSWORD)
            .build()
            .await
    }

    // Get SDK with both erasure coding and encryption
    async fn get_sdk_with_erasure_and_encryption(
    ) -> Result<AkaveSDK, Box<(dyn std::error::Error + 'static)>> {
        AkaveSDKBuilder::new("http://connect.akave.ai:5500")
            .with_erasure_coding(3, 2)
            .with_default_encryption(TEST_PASSWORD)
            .build()
            .await
    }

    // Helper to create a unique bucket name for each test
    fn generate_test_bucket_name() -> String {
        format!(
            "TEST_BUCKET_{}",
            Uuid::new_v4().to_string().split('-').next().unwrap()
        )
    }

    // Helper to clean up downloaded files
    fn cleanup_download(file_path: &str) {
        if Path::new(file_path).exists() {
            let _ = fs::remove_file(file_path);
        }
    }

    // Helper to ensure download directory exists
    fn ensure_download_dir() {
        let _ = fs::create_dir_all(DOWNLOAD_DESTINATION);
    }

    #[tokio::test]
    async fn test_create_bucket() {
        let bucket_name = generate_test_bucket_name();
        println!("Testing create bucket: {}", bucket_name);

        // Test
        let mut sdk = get_sdk().await.unwrap();
        let bucket_resp = sdk.create_bucket(&bucket_name).await.unwrap();
        assert_eq!(bucket_resp.name, bucket_name);

        // Cleanup
        let _ = sdk.delete_bucket(ADDRESS, &bucket_name).await;
    }

    #[tokio::test]
    async fn test_list_buckets() {
        let bucket_name = generate_test_bucket_name();
        println!("Testing list buckets");

        // Setup
        let mut sdk = get_sdk().await.unwrap();
        let _ = sdk.create_bucket(&bucket_name).await.unwrap();

        // Test
        let buckets = sdk.list_buckets(ADDRESS).await.unwrap();
        let len = buckets.buckets.len();
        println!("Found {} buckets", len);
        assert_ne!(len, 0, "there should be buckets in this account");

        // Cleanup
        let _ = sdk.delete_bucket(ADDRESS, &bucket_name).await;
    }

    #[tokio::test]
    async fn test_view_bucket() {
        let bucket_name = generate_test_bucket_name();
        println!("Testing view bucket: {}", bucket_name);

        // Setup
        let mut sdk = get_sdk().await.unwrap();
        let _ = sdk.create_bucket(&bucket_name).await.unwrap();

        // Test
        let bucket = sdk.view_bucket(ADDRESS, &bucket_name).await.unwrap();
        assert_eq!(bucket.name, bucket_name);

        // Cleanup
        let _ = sdk.delete_bucket(ADDRESS, &bucket_name).await;
    }

    #[tokio::test]
    async fn test_delete_bucket() {
        let bucket_name = generate_test_bucket_name();
        println!("Testing delete bucket: {}", bucket_name);

        // Setup
        let mut sdk = get_sdk().await.unwrap();
        let _ = sdk.create_bucket(&bucket_name).await.unwrap();

        // Test delete
        let result = sdk.delete_bucket(ADDRESS, &bucket_name).await;
        assert!(
            result.is_ok(),
            "Failed to delete bucket: {:?}",
            result.err()
        );

        // Verify deletion - this might need adjustment based on expected behavior
        // If view_bucket is expected to return an error for non-existent buckets:
        let view_result = sdk.view_bucket(ADDRESS, &bucket_name).await;
        assert!(
            view_result.is_err() || view_result.unwrap().name != bucket_name,
            "Bucket should not exist after deletion"
        );
    }

    #[tokio::test]
    async fn test_upload_and_list_files() {
        let bucket_name = generate_test_bucket_name();
        println!("Testing upload file to bucket: {}", bucket_name);

        // Setup
        let mut sdk = get_sdk().await.unwrap();
        let _ = sdk.create_bucket(&bucket_name).await.unwrap();

        // Test upload
        let file = File::open(format!("test_files/{}", FILE_NAME_TO_TEST)).unwrap();
        let upload_result = sdk
            .upload_file(&bucket_name, FILE_NAME_TO_TEST, file, None)
            .await;
        assert!(
            upload_result.is_ok(),
            "Failed to upload file: {:?}",
            upload_result.err()
        );

        // Test list files
        let file_list = sdk.list_files(ADDRESS, &bucket_name).await.unwrap();
        assert_ne!(
            file_list.files.len(),
            0,
            "there should be files in this bucket"
        );
        let has_test_file = file_list
            .files
            .iter()
            .any(|file| file.name == FILE_NAME_TO_TEST);
        assert!(has_test_file, "Uploaded file not found in bucket");

        // Test delete files and list
        for file in file_list.files {
            let _ = sdk
                .delete_file(ADDRESS, &bucket_name, &file.name)
                .await
                .expect("failed to delete file");
        }
        let file_list = sdk.list_files(ADDRESS, &bucket_name).await.unwrap();
        assert_eq!(
            file_list.files.len(),
            0,
            "there should be no files in this bucket"
        );

        // Cleanup
        let _ = sdk.delete_bucket(ADDRESS, &bucket_name).await;
    }

    #[tokio::test]
    async fn test_download_file() {
        let bucket_name = generate_test_bucket_name();
        println!("Testing download file from bucket: {}", bucket_name);

        // Setup
        ensure_download_dir();
        let download_path = format!("{}{}", DOWNLOAD_DESTINATION, FILE_NAME_TO_TEST);
        let mut sdk = get_sdk().await.unwrap();
        let _ = sdk.create_bucket(&bucket_name).await.unwrap();

        let file = File::open(format!("test_files/{}", FILE_NAME_TO_TEST)).unwrap();
        let _ = sdk
            .upload_file(&bucket_name, FILE_NAME_TO_TEST, file, None)
            .await
            .unwrap();

        // Clean up any previously downloaded file
        cleanup_download(&download_path);

        // Test download
        let download_result = sdk
            .download_file(
                ADDRESS,
                &bucket_name,
                FILE_NAME_TO_TEST,
                None,
                DOWNLOAD_DESTINATION,
            )
            .await;

        assert!(
            download_result.is_ok(),
            "Failed to download file: {:?}",
            download_result.err()
        );
        assert!(
            Path::new(&download_path).exists(),
            "Downloaded file not found"
        );

        // Cleanup
        cleanup_download(&download_path);
        let _ = sdk.delete_bucket(ADDRESS, &bucket_name).await;
    }

    #[tokio::test]
    async fn test_download_file_with_erasure() {
        let bucket_name = generate_test_bucket_name();
        println!(
            "Testing download file with erasure coding from bucket: {}",
            bucket_name
        );

        // Setup
        ensure_download_dir();
        let download_path = format!("{}{}", DOWNLOAD_DESTINATION, FILE_NAME_TO_TEST);
        let mut sdk = get_sdk_with_erasure().await.unwrap();
        let _ = sdk.create_bucket(&bucket_name).await.unwrap();

        let file = File::open(format!("test_files/{}", FILE_NAME_TO_TEST)).unwrap();
        let _ = sdk
            .upload_file(&bucket_name, FILE_NAME_TO_TEST, file, None)
            .await
            .unwrap();

        // Clean up any previously downloaded file
        cleanup_download(&download_path);

        // Test download
        let download_result = sdk
            .download_file(
                ADDRESS,
                &bucket_name,
                FILE_NAME_TO_TEST,
                None,
                DOWNLOAD_DESTINATION,
            )
            .await;

        assert!(
            download_result.is_ok(),
            "Failed to download file with erasure coding: {:?}",
            download_result.err()
        );
        assert!(
            Path::new(&download_path).exists(),
            "Downloaded file not found"
        );

        // Cleanup
        cleanup_download(&download_path);
        let _ = sdk.delete_bucket(ADDRESS, &bucket_name).await;
    }

    #[tokio::test]
    async fn test_download_file_with_encryption() {
        let bucket_name = generate_test_bucket_name();
        println!(
            "Testing download file with encryption from bucket: {}",
            bucket_name
        );

        // Setup
        ensure_download_dir();
        let download_path = format!("{}{}", DOWNLOAD_DESTINATION, FILE_NAME_TO_TEST);
        let mut sdk = get_sdk_with_encryption().await.unwrap();
        let _ = sdk.create_bucket(&bucket_name).await.unwrap();

        let file = File::open(format!("test_files/{}", FILE_NAME_TO_TEST)).unwrap();
        let _ = sdk
            .upload_file(&bucket_name, FILE_NAME_TO_TEST, file, None)
            .await
            .unwrap();

        // Clean up any previously downloaded file
        cleanup_download(&download_path);

        // Test download
        let download_result = sdk
            .download_file(
                ADDRESS,
                &bucket_name,
                FILE_NAME_TO_TEST,
                None,
                DOWNLOAD_DESTINATION,
            )
            .await;

        assert!(
            download_result.is_ok(),
            "Failed to download file with encryption: {:?}",
            download_result.err()
        );
        assert!(
            Path::new(&download_path).exists(),
            "Downloaded file not found"
        );

        // Cleanup
        cleanup_download(&download_path);
        let _ = sdk.delete_bucket(ADDRESS, &bucket_name).await;
    }

    #[tokio::test]
    async fn test_full_lifecycle() {
        let bucket_name = generate_test_bucket_name();
        println!(
            "Testing full lifecycle with erasure coding and encryption with bucket: {}",
            bucket_name
        );

        // Setup
        ensure_download_dir();
        let download_path = format!("{}{}", DOWNLOAD_DESTINATION, FILE_NAME_TO_TEST);
        let mut sdk = get_sdk_with_erasure_and_encryption().await.unwrap();

        // Create bucket
        println!("Creating bucket: {}", bucket_name);
        let bucket_resp = sdk.create_bucket(&bucket_name).await.unwrap();
        assert_eq!(bucket_resp.name, bucket_name);

        // Upload file (using default encryption from SDK)
        println!("Uploading file: {}", FILE_NAME_TO_TEST);
        let file = File::open(format!("test_files/{}", FILE_NAME_TO_TEST)).unwrap();
        let _ = sdk
            .upload_file(&bucket_name, FILE_NAME_TO_TEST, file, None)
            .await
            .unwrap();

        // List files
        println!("listing files in bucket {}", bucket_name);
        let file_list = sdk.list_files(ADDRESS, &bucket_name).await.unwrap();
        let has_test_file = file_list
            .files
            .iter()
            .any(|file| file.name == FILE_NAME_TO_TEST);
        assert!(has_test_file, "Uploaded file not found in bucket");

        // Download file (using default encryption from SDK)
        cleanup_download(&download_path); // Clean up any previously downloaded file
        let download_result = sdk
            .download_file(
                ADDRESS,
                &bucket_name,
                FILE_NAME_TO_TEST,
                None,
                DOWNLOAD_DESTINATION,
            )
            .await;
        assert!(download_result.is_ok());
        assert!(Path::new(&download_path).exists());
        let file = File::open(&download_path).unwrap();
        let fsize = file.size();
        assert_eq!(fsize, 920840);

        // Cleanup
        cleanup_download(&download_path);

        // Delete file
        println!("deleting file {}", FILE_NAME_TO_TEST);
        let _ = sdk.delete_file(ADDRESS, &bucket_name, FILE_NAME_TO_TEST);

        // Delete bucket
        println!("deleting bucket {}", bucket_name);
        let _ = sdk.delete_bucket(ADDRESS, &bucket_name).await;
    }

    // helper cleanup function
    async fn cleanup_all_test_resources() {
        println!("Starting cleanup of all test resources...");

        let mut sdk = get_sdk().await.unwrap();

        // 1. List all buckets
        println!("Listing all buckets to find test buckets...");
        let buckets = match sdk.list_buckets(ADDRESS).await {
            Ok(bucket_list) => bucket_list.buckets,
            Err(e) => {
                println!("Error listing buckets: {:?}", e);
                return;
            }
        };

        let test_buckets: Vec<_> = buckets
            .iter()
            .filter(|bucket| bucket.name.starts_with("TEST_BUCKET_"))
            .collect();

        println!("Found {} test buckets to clean up", test_buckets.len());

        // 2. Clean up any downloaded test files
        println!("Cleaning up downloaded test files...");
        if let Ok(entries) = fs::read_dir(DOWNLOAD_DESTINATION) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_file() {
                        if let Ok(file_name) = entry.file_name().into_string() {
                            println!("Removing downloaded file: {}", file_name);
                            let _ = fs::remove_file(entry.path());
                        }
                    }
                }
            }
        }

        // 3. Empty each test bucket then delete.
        for bucket in test_buckets {
            println!("Deleting test bucket: {}", bucket.name);
            // TODO: fixme
            // let files = sdk.list_files(ADDRESS, &bucket.name).await.expect("Failed to list files for specified (address, bucket_name)");

            // for file in files {
            //     println!("Deleting test file: {}, from bucket: {}", file.name, bucket.name);
            //     match sdk.delete_file(ADDRESS, bucket.name.as_str(), file.name.as_str()).await {
            //         Ok(_) => println!("Successfully deleted file: {}", file.name),
            //         Err(e) => println!("Error deleting bucket {}: {:?}", bucket.name, e),
            //     }
            // }
            match sdk.delete_bucket(ADDRESS, &bucket.name).await {
                Ok(_) => println!("Successfully deleted bucket: {}", bucket.name),
                Err(e) => println!("Error deleting bucket {}: {:?}", bucket.name, e),
            }
        }

        println!("Cleanup complete!");
    }

    #[tokio::test]
    #[ignore]
    async fn test_cleanup_manual() {
        // This test is ignored by default and must be run manually with:
        // cargo test --package akave-wasm-sdk --lib -- tests::test_cleanup_manual --ignored --nocapture
        cleanup_all_test_resources().await;
    }
}
