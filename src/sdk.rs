// ==========================
// Proto module definition
// ==========================
pub(crate) mod ipcnodeapi {
    tonic::include_proto!("ipcnodeapi");
}

// ==========================
// Standard library imports
// ==========================
use std::{
    borrow::Cow,
    io::{Read, Write},
    str::FromStr,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

// ==========================
// External crate imports
// ==========================
use alloy::hex;
use bytesize::{ByteSize, MB};
use cid::Cid;
// ==========================
// Proto-related imports
// ==========================
#[cfg(not(target_arch = "wasm32"))]
use ipcnodeapi::{ipc_archival_api_client::IpcArchivalApiClient, IpcFileResolveBlockRequest};
use ipcnodeapi::{
    ipc_chunk::Block, ipc_node_api_client::IpcNodeApiClient, ConnectionParamsRequest,
    IpcBucketListRequest, IpcBucketViewRequest, IpcChunk, IpcFileBlockData,
    IpcFileDownloadBlockRequest, IpcFileDownloadChunkCreateRequest, IpcFileDownloadCreateRequest,
    IpcFileDownloadRangeCreateRequest, IpcFileListRequest, IpcFileUploadChunkCreateRequest,
    IpcFileViewRequest,
};
use quick_protobuf::BytesReader;
use tokio::sync::Semaphore;
use web3::types::{TransactionReceipt, U256};

use crate::{
    blockchain::{access_manager::AccessManagerContract, storage::FileStorageContract},
    utils::peer_id::PeerId,
};
// ==========================
// Internal crate imports
// ==========================
use crate::{
    blockchain::{
        eip712_utils::create_block_eip712_data, ipc_types::BucketResponse,
        provider::BlockchainProvider,
    },
    log_debug, log_error, log_info,
    types::{
        sdk_types::{
            AkaveError, ArchivalBlock, ArchivalChunk, ArchivalMetadata, BucketListItem,
            BucketListResponse, BucketViewResponse, FileBlockDownload, FileChunk,
            FileChunkDownload, FileDownloadResponse, FileListItem, FileListResponse,
            FileViewResponse, IpcFileChunkUpload, PDPBlockData,
        },
        BucketId,
    },
    utils,
    utils::dag::{ChunkDag, DagRoot, DAG_PROTOBUF},
    utils::encryption::Encryption,
    utils::erasure::ErasureCode,
    utils::pb_data::PbData,
};

// ==========================
// Target-specific imports and types
// ==========================
#[cfg(target_arch = "wasm32")]
mod wasm_support {
    pub use std::{future::Future, pin::Pin};

    pub use tonic_web_wasm_client::Client as GrpcWebClient;
    pub type ClientTransport = GrpcWebClient;
    // Add more WASM-specific imports/types here if needed
}
#[cfg(target_arch = "wasm32")]
use wasm_support::*;

#[cfg(not(target_arch = "wasm32"))]
mod native_support {
    pub use tokio_stream::{self, StreamExt};
    pub use tonic::transport::{Channel, ClientTlsConfig};
    pub type ClientTransport = Channel;
    // Add more native-specific imports/types here if needed
}
#[cfg(not(target_arch = "wasm32"))]
use native_support::*;

// Constants
const BLOCK_SIZE: usize = MB as usize;
const MIN_BUCKET_NAME_LENGTH: usize = 3;
const MIN_FILE_SIZE: usize = 127;
const MAX_BLOCKS_IN_CHUNK: usize = 32;
const BLOCK_PART_SIZE: usize = ByteSize::kb(128).as_u64() as usize;

/// Represents the Akave SDK client
/// Akave SDK should support both WASM (gRPC-Web) and native gRPC
#[derive(Clone)]
pub struct AkaveSDK {
    client: IpcNodeApiClient<ClientTransport>,
    storage: FileStorageContract,
    access_manager: AccessManagerContract,
    erasure_code: Option<utils::erasure::ErasureCode>,
    default_encryption_key: Option<String>,
    use_metadata_encryption: bool,
    block_size: usize,
    min_bucket_name_length: usize,
    max_blocks_in_chunk: usize,
    block_part_size: usize,
    min_file_size: usize,
    max_concurrent_blocks: usize,
    chunk_batch_size: usize,
    chain_id: U256,
    with_retry: crate::utils::retry::WithRetry,
    #[cfg(not(target_arch = "wasm32"))]
    connection_pool: Arc<tokio::sync::RwLock<std::collections::HashMap<String, Channel>>>,
    #[cfg(not(target_arch = "wasm32"))]
    http_client: Option<reqwest::Client>,
}

/// Builder for AkaveSDK
pub struct AkaveSDKBuilder {
    server_address: String,
    data_blocks: Option<usize>,
    parity_blocks: Option<usize>,
    default_encryption_key: Option<String>,
    use_metadata_encryption: bool,
    block_size: usize,
    min_bucket_name_length: usize,
    max_blocks_in_chunk: usize,
    block_part_size: usize,
    min_file_size: usize,
    max_concurrent_blocks: usize,
    chunk_batch_size: usize,
    with_retry: crate::utils::retry::WithRetry,
    #[cfg(not(target_arch = "wasm32"))]
    private_key: Option<String>,
    #[cfg(not(target_arch = "wasm32"))]
    http_client: Option<reqwest::Client>,
}

impl AkaveSDKBuilder {
    /// Create a new AkaveSDKBuilder with the given server address
    pub fn new(server_address: &str) -> Self {
        Self {
            server_address: server_address.to_string(),
            data_blocks: None,
            parity_blocks: None,
            default_encryption_key: None,
            use_metadata_encryption: false,
            block_size: BLOCK_SIZE,
            min_bucket_name_length: MIN_BUCKET_NAME_LENGTH,
            max_blocks_in_chunk: MAX_BLOCKS_IN_CHUNK,
            block_part_size: BLOCK_PART_SIZE,
            min_file_size: MIN_FILE_SIZE,
            max_concurrent_blocks: 5,
            chunk_batch_size: 1,
            with_retry: crate::utils::retry::WithRetry {
                max_attempts: 5,
                base_delay: std::time::Duration::from_millis(100),
            },
            #[cfg(not(target_arch = "wasm32"))]
            private_key: None,
            #[cfg(not(target_arch = "wasm32"))]
            http_client: None,
        }
    }

    /// Configure retry for blockchain write operations (default: 5 attempts, 100ms base delay).
    pub fn with_retry(mut self, max_attempts: usize, base_delay: std::time::Duration) -> Self {
        self.with_retry = crate::utils::retry::WithRetry {
            max_attempts,
            base_delay,
        };
        self
    }

    /// Disable retries for blockchain write operations (useful in tests).
    pub fn without_retry(mut self) -> Self {
        self.with_retry = crate::utils::retry::WithRetry {
            max_attempts: 0,
            base_delay: std::time::Duration::from_millis(100),
        };
        self
    }

    /// Set erasure coding parameters
    pub fn with_erasure_coding(mut self, data_blocks: usize, parity_blocks: usize) -> Self {
        self.data_blocks = Some(data_blocks);
        self.parity_blocks = Some(parity_blocks);
        self
    }

    /// Set default encryption key
    pub fn with_default_encryption(mut self, encryption_key: &str, encrypt_metadata: bool) -> Self {
        self.default_encryption_key = Some(encryption_key.to_string());
        self.use_metadata_encryption = encrypt_metadata;
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

    /// Set minimum file size
    pub fn with_min_file_size(mut self, min_file_size: usize) -> Self {
        self.min_file_size = min_file_size;
        self
    }

    /// Set maximum concurrent block downloads
    pub fn with_max_concurrent_blocks(mut self, max_concurrent: usize) -> Self {
        self.max_concurrent_blocks = max_concurrent;
        self
    }

    /// Set batch size for chunk upload transactions (min 1)
    pub fn with_chunk_batch_size(mut self, chunk_batch_size: usize) -> Self {
        self.chunk_batch_size = chunk_batch_size.max(1);
        self
    }

    /// Set private key for native (non-WASM) environments
    #[cfg(not(target_arch = "wasm32"))]
    pub fn with_private_key(mut self, private_key: &str) -> Self {
        self.private_key = Some(private_key.to_string());
        self
    }

    /// Inject a custom reqwest HTTP client (used for PDP/archival downloads).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn with_http_client(mut self, client: reqwest::Client) -> Self {
        self.http_client = Some(client);
        self
    }

    /// Build the AkaveSDK instance
    pub async fn build(self) -> Result<AkaveSDK, AkaveError> {
        let erasure_code = match (self.data_blocks, self.parity_blocks) {
            (Some(data), Some(parity)) => Some(utils::erasure::ErasureCode::new(data, parity)?),
            _ => None,
        };

        AkaveSDK::new_with_params(
            &self.server_address,
            erasure_code,
            self.default_encryption_key,
            self.use_metadata_encryption,
            self.block_size,
            self.min_bucket_name_length,
            self.max_blocks_in_chunk,
            self.block_part_size,
            self.min_file_size,
            self.max_concurrent_blocks,
            self.chunk_batch_size,
            self.with_retry,
            #[cfg(not(target_arch = "wasm32"))]
            self.private_key,
            #[cfg(not(target_arch = "wasm32"))]
            self.http_client,
        )
        .await
    }
}

/// Holds the result of the gRPC-only chunk preparation (no contract call).
struct PreparedChunk {
    chunk_upload: IpcFileChunkUpload,
    ipc_chunk: IpcChunk,
    cids: Vec<[u8; 32]>,
    sizes: Vec<U256>,
}

impl AkaveSDK {
    /// Creates a new AkaveSDK instance with default parameters
    pub async fn new(server_address: &str) -> Result<Self, AkaveError> {
        Self::new_with_params(
            server_address,
            None,
            None,
            false,
            BLOCK_SIZE,
            MIN_BUCKET_NAME_LENGTH,
            MAX_BLOCKS_IN_CHUNK,
            BLOCK_PART_SIZE,
            MIN_FILE_SIZE,
            5,
            1,
            crate::utils::retry::WithRetry {
                max_attempts: 5,
                base_delay: std::time::Duration::from_millis(100),
            },
            #[cfg(not(target_arch = "wasm32"))]
            None,
            #[cfg(not(target_arch = "wasm32"))]
            None,
        )
        .await
    }

    /// Creates a new AkaveSDK instance with custom parameters
    #[allow(clippy::too_many_arguments)]
    async fn new_with_params(
        server_address: &str,
        erasure_code: Option<utils::erasure::ErasureCode>,
        default_encryption_key: Option<String>,
        use_metadata_encryption: bool,
        block_size: usize,
        min_bucket_name_length: usize,
        max_blocks_in_chunk: usize,
        block_part_size: usize,
        min_file_size: usize,
        max_concurrent_blocks: usize,
        batch_size: usize,
        with_retry: crate::utils::retry::WithRetry,
        #[cfg(not(target_arch = "wasm32"))] private_key: Option<String>,
        #[cfg(not(target_arch = "wasm32"))] http_client: Option<reqwest::Client>,
    ) -> Result<Self, AkaveError> {
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
                .await
                .map_err(|e| AkaveError::GrpcError(Box::new(e)))?
                .into_inner();
            log_debug!("Creating blockchain provider... {:?}", connection_params);

            let blockchain_provider =
                Arc::new(BlockchainProvider::new(&connection_params.dial_uri, None)?);

            let storage = FileStorageContract::new(
                blockchain_provider.clone(),
                &connection_params.storage_address,
            )?;

            let access_manager = AccessManagerContract::new(
                blockchain_provider.clone(),
                &connection_params.access_address,
            )?;

            // Query chain ID once during initialization
            let chain_id = blockchain_provider.web3_provider.eth().chain_id().await?;
            log_debug!("Chain ID: {}", chain_id);

            log_info!("AkaveSDK initialized successfully");
            Ok(Self {
                client,
                storage,
                access_manager,
                erasure_code,
                default_encryption_key,
                use_metadata_encryption,
                block_size,
                min_bucket_name_length,
                max_blocks_in_chunk,
                block_part_size,
                min_file_size,
                max_concurrent_blocks,
                chunk_batch_size: batch_size,
                chain_id,
                with_retry,
            })
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            use crate::blockchain::storage::FileStorageContract;
            #[allow(unused_imports)]
            use std::collections::HashMap;

            let tls_config = ClientTlsConfig::new().with_native_roots();
            let channel = Channel::from_shared(server_address.to_string())
                .map_err(|e| AkaveError::ChannelError(e.to_string()))?
                .tls_config(tls_config)
                .map_err(|e| AkaveError::ChannelError(e.to_string()))?
                .connect()
                .await
                .map_err(|e| AkaveError::ChannelError(e.to_string()))?;

            let mut client = IpcNodeApiClient::new(channel)
                .max_decoding_message_size(usize::MAX)
                .max_encoding_message_size(usize::MAX);
            log_debug!("Requesting connection parameters...");
            let connection_params = client
                .connection_params(ConnectionParamsRequest {})
                .await
                .map_err(|e| AkaveError::GrpcError(Box::new(e)))?
                .into_inner();

            log_debug!("Creating blockchain provider... {:?}", connection_params);
            let blockchain_provider = Arc::new(BlockchainProvider::new(
                &connection_params.dial_uri,
                None,
                private_key.as_deref(),
            )?);

            let storage = FileStorageContract::new(
                blockchain_provider.clone(),
                &connection_params.storage_address,
            )?;

            let access_manager = AccessManagerContract::new(
                blockchain_provider.clone(),
                &connection_params.access_address,
            )?;

            // Query chain ID once during initialization
            let chain_id = blockchain_provider.web3_provider.eth().chain_id().await?;
            log_debug!("Chain ID: {}", chain_id);

            let connection_pool =
                Arc::new(tokio::sync::RwLock::new(HashMap::<String, Channel>::new()));

            log_info!("AkaveSDK initialized successfully");
            Ok(Self {
                client,
                storage,
                access_manager,
                erasure_code,
                default_encryption_key,
                use_metadata_encryption,
                block_size,
                min_bucket_name_length,
                max_blocks_in_chunk,
                block_part_size,
                min_file_size,
                max_concurrent_blocks,
                chunk_batch_size: batch_size,
                chain_id,
                with_retry,
                connection_pool,
                http_client,
            })
        }
    }

    /// Returns true for transient Ethereum RPC errors that warrant retrying.
    fn is_retryable_tx_error(err: &str) -> bool {
        err.contains("nonce too low")
            || err.contains("replacement transaction underpriced")
            || err.contains("EOF")
    }

    /// Read up to `n` bytes from `reader`, treating `UnexpectedEof` as a normal EOF.
    async fn read_up_to<R: tokio::io::AsyncRead + Unpin>(
        reader: &mut R,
        n: usize,
    ) -> std::io::Result<Vec<u8>> {
        use tokio::io::AsyncReadExt;
        let mut buf = vec![0u8; n];
        let mut pos = 0;
        loop {
            match reader.read(&mut buf[pos..]).await {
                Ok(0) => break,
                Ok(read) => pos += read,
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(e),
            }
        }
        buf.truncate(pos);
        Ok(buf)
    }

    /// Execute a blockchain transaction operation with the SDK's configured retry policy.
    async fn retry_tx<F, Fut, T>(&self, mut op: F) -> Result<T, AkaveError>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T, AkaveError>>,
    {
        let (_cancel_tx, cancel_rx) = tokio::sync::watch::channel(false);
        self.with_retry
            .do_retry(cancel_rx, || {
                let fut = op();
                async move {
                    fut.await.map_err(|e| {
                        let retryable = Self::is_retryable_tx_error(&e.to_string());
                        (retryable, e)
                    })
                }
            })
            .await
            .map_err(|e| match e {
                crate::utils::retry::RetryError::Aborted => {
                    AkaveError::InternalError("retry aborted".into())
                }
                crate::utils::retry::RetryError::Failed(e) => e,
            })
    }

    /// List all buckets
    pub async fn list_buckets(
        &self,
        offset: i64,
        limit: i64,
    ) -> Result<BucketListResponse, AkaveError> {
        let address = self.storage.client.get_hex_address().await?;
        log_debug!("Listing buckets for address: {}", address);
        let request = IpcBucketListRequest {
            address: address.to_string(),
            offset,
            limit,
        };
        let mut client = self.client.clone();
        let response = client
            .bucket_list(request)
            .await
            .map_err(|e| AkaveError::GrpcError(Box::new(e)))?
            .into_inner();

        let buckets: Vec<BucketListItem> = response
            .buckets
            .into_iter()
            .map(|bucket| {
                let name = self
                    .maybe_decrypt_metadata(&bucket.name, "bucket")
                    .unwrap_or(bucket.name);
                BucketListItem {
                    id: String::new(),
                    name,
                    created_at: bucket.created_at.map(|ts| ts.seconds).unwrap_or(0),
                }
            })
            .collect();

        log_info!("Found {} buckets", buckets.len());
        Ok(BucketListResponse { buckets })
    }

    /// View a bucket
    pub async fn view_bucket(&self, bucket_name: &str) -> Result<BucketViewResponse, AkaveError> {
        let address = self.storage.client.get_hex_address().await?;
        log_debug!("Viewing bucket: {} for address: {}", bucket_name, address);
        let bucket_name =
            self.maybe_encrypt_metadata(bucket_name.to_string(), "bucket".to_string(), None)?;
        let request = IpcBucketViewRequest {
            name: bucket_name.clone(),
            address: address.to_string(),
        };
        let response = self
            .client
            .clone()
            .bucket_view(request)
            .await
            .map_err(|e| AkaveError::GrpcError(Box::new(e)))?
            .into_inner();

        let bucket = BucketViewResponse {
            id: response.id,
            name: self
                .maybe_decrypt_metadata(&response.name, "bucket")
                .unwrap_or(response.name),
            created_at: response.created_at.map(|ts| ts.seconds).unwrap_or(0),
            file_count: 0,
        };

        log_info!("Retrieved bucket details for: {}", bucket_name);
        Ok(bucket)
    }

    /// List files in a bucket
    pub async fn list_files(
        &self,
        bucket_name: &str,
        offset: i64,
        limit: i64,
    ) -> Result<FileListResponse, AkaveError> {
        let address = self.storage.client.get_hex_address().await?;
        log_debug!(
            "Listing files in bucket: {} for address: {}",
            bucket_name,
            address
        );
        let original_bucket_name = bucket_name;
        let bucket_name =
            self.maybe_encrypt_metadata(bucket_name.to_string(), "bucket".to_string(), None)?;

        let request = IpcFileListRequest {
            bucket_name: bucket_name.clone(),
            address: address.to_string(),
            offset,
            limit,
        };
        let response = self
            .client
            .clone()
            .file_list(request)
            .await
            .map_err(|e| AkaveError::GrpcError(Box::new(e)))?
            .into_inner();

        let files: Vec<FileListItem> = response
            .list
            .into_iter()
            .map(|file| {
                let name = self
                    .maybe_decrypt_metadata(&file.name, original_bucket_name)
                    .unwrap_or(file.name);
                FileListItem {
                    root_cid: file.root_cid,
                    created_at: file.created_at.map(|ts| ts.seconds).unwrap_or(0),
                    actual_size: file.actual_size,
                    encoded_size: file.encoded_size,
                    name,
                }
            })
            .collect();

        log_info!("Found {} files in bucket: {}", files.len(), bucket_name);
        Ok(FileListResponse { files })
    }

    /// View file information
    pub async fn view_file_info(
        &self,
        bucket_name: &str,
        file_name: &str,
    ) -> Result<FileViewResponse, AkaveError> {
        let address = self.storage.client.get_hex_address().await?;
        log_debug!(
            "Viewing file info: {} in bucket: {} for address: {}",
            file_name,
            bucket_name,
            address
        );

        let original_bucket_name = bucket_name;
        let file_name =
            self.maybe_encrypt_metadata(file_name.to_string(), bucket_name.to_string(), None)?;

        let bucket_name =
            self.maybe_encrypt_metadata(bucket_name.to_string(), "bucket".to_string(), None)?;

        let request = IpcFileViewRequest {
            bucket_name: bucket_name.clone(),
            file_name: file_name.clone(),
            address: address.to_string(),
        };
        let response = self
            .client
            .clone()
            .file_view(request)
            .await
            .map_err(|e| AkaveError::GrpcError(Box::new(e)))?
            .into_inner();

        let decrypted_bucket = self
            .maybe_decrypt_metadata(&response.bucket_name, "bucket")
            .unwrap_or(response.bucket_name);
        let decrypted_file = self
            .maybe_decrypt_metadata(&response.file_name, original_bucket_name)
            .unwrap_or(response.file_name);

        let file = FileViewResponse {
            root_cid: response.root_cid,
            created_at: response.created_at.map(|ts| ts.seconds).unwrap_or(0),
            actual_size: response.actual_size,
            encoded_size: response.encoded_size,
            name: decrypted_file,
            bucket_name: decrypted_bucket,
            is_public: response.is_public,
        };

        log_info!(
            "Retrieved file details for: {} in bucket: {}",
            file_name,
            bucket_name
        );
        Ok(file)
    }

    // Create a new bucket
    pub async fn create_bucket(&self, bucket_name: &str) -> Result<BucketResponse, AkaveError> {
        log_debug!("Creating bucket: {}", bucket_name);

        let bucket_name =
            self.maybe_encrypt_metadata(bucket_name.to_string(), "bucket".to_string(), None)?;

        if bucket_name.len() < self.min_bucket_name_length {
            let error_msg = format!(
                "Bucket name must have at least {} characters",
                self.min_bucket_name_length
            );
            log_error!("{}", error_msg);
            return Err(AkaveError::BucketError(error_msg));
        }
        log_info!("Create bucket request to storage: {}", &bucket_name);
        let storage = self.storage.clone();
        let bn = bucket_name.clone();
        self.retry_tx(|| {
            let storage = storage.clone();
            let bn = bn.clone();
            async move {
                storage
                    .create_bucket(bn)
                    .await
                    .map_err(AkaveError::ProviderError)
            }
        })
        .await?;
        log_info!("Bucket created successfully: {}", bucket_name);
        let mut resp = self
            .storage
            .get_bucket_by_name(bucket_name)
            .await
            .map_err(AkaveError::ProviderError)?;
        resp.name = self
            .maybe_decrypt_metadata(&resp.name, "bucket")
            .unwrap_or(resp.name);
        Ok(resp)
    }

    // Delete an existing bucket
    pub async fn delete_bucket(&self, bucket_name: &str) -> Result<(), AkaveError> {
        let address = self.storage.client.get_hex_address().await?;
        log_debug!("Deleting bucket: {} for address: {}", bucket_name, address);
        let bucket = self.view_bucket(bucket_name).await?;
        let bucket_name =
            self.maybe_encrypt_metadata(bucket_name.to_string(), "bucket".to_string(), None)?;
        let bucket_id_bytes = hex::decode(bucket.id.clone())
            .map_err(|e| AkaveError::InvalidInput(format!("Invalid bucket ID hex: {}", e)))?;
        let bucket_id = BucketId::from_slice(&bucket_id_bytes)
            .ok_or_else(|| AkaveError::InvalidInput("Invalid bucket ID length".to_string()))?;
        let bucket_idx = self
            .storage
            .get_bucket_index_by_name(bucket_name.clone())
            .await
            .map_err(AkaveError::ProviderError)?;
        if !bucket_idx.exists {
            return Err(AkaveError::BucketError(format!(
                "bucket index not found: {}",
                bucket_name
            )));
        }

        self.storage
            .delete_bucket(bucket_id, bucket_name.clone(), bucket_idx.index)
            .await
            .map_err(AkaveError::ProviderError)?;
        log_info!("Bucket deleted successfully: {}", &bucket_name);
        Ok(())
    }

    // Delete an existing file
    pub async fn delete_file(&self, bucket_name: &str, file_name: &str) -> Result<(), AkaveError> {
        let address = self.storage.client.get_hex_address().await?;
        log_debug!(
            "Deleting file: {} from bucket: {} for address: {}",
            file_name,
            bucket_name,
            address
        );
        let bucket = self.view_bucket(bucket_name).await?;
        let file_name =
            self.maybe_encrypt_metadata(file_name.to_string(), bucket_name.to_string(), None)?;
        let bucket_name =
            self.maybe_encrypt_metadata(bucket_name.to_string(), "bucket".to_string(), None)?;
        let bucket_id_bytes = hex::decode(bucket.id.clone())
            .map_err(|e| AkaveError::InvalidInput(format!("Invalid bucket ID hex: {}", e)))?;
        let bucket_id = BucketId::from_slice(&bucket_id_bytes)
            .ok_or_else(|| AkaveError::InvalidInput("Invalid bucket ID length".to_string()))?;
        self.storage
            .delete_file(file_name.to_string(), bucket_name.clone(), bucket_id)
            .await
            .map_err(AkaveError::ProviderError)?;
        log_info!(
            "File deleted successfully: {} from bucket: {}",
            file_name,
            bucket_name
        );
        Ok(())
    }

    /// Calculate file ID the same way as the smart contract (Keccak256 of bucket_id + filename)
    fn calculate_file_id(bucket_id: &BucketId, file_name: &str) -> [u8; 32] {
        use web3::signing::keccak256;
        let mut data = Vec::new();
        data.extend_from_slice(&bucket_id.to_bytes());
        data.extend_from_slice(file_name.as_bytes());
        keccak256(&data)
    }

    /// Wait for file to be filled on the blockchain before committing.
    /// Polls indefinitely (1-second intervals) matching Go's context-bounded infinite loop.
    async fn wait_for_file_filled(
        &self,
        file_id: [u8; 32],
        file_name: &str,
    ) -> Result<(), AkaveError> {
        log_debug!("Waiting for file to be filled: {}", file_name);

        loop {
            let is_filled = self.storage.is_file_filled(file_id).await.map_err(|e| {
                AkaveError::FileOperationError {
                    operation: "check_file_filled".to_string(),
                    file_name: file_name.to_string(),
                    message: format!("Failed to check if file is filled: {}", e),
                }
            })?;

            if is_filled {
                log_debug!("File is filled");
                return Ok(());
            }

            log_debug!("File not yet filled, waiting...");
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    }

    // uploads a file to akave network.
    pub async fn upload_file<R>(
        &self,
        bucket_name: &str,
        file_name: &str,
        reader: &mut R,
        passwd: Option<&str>,
    ) -> Result<TransactionReceipt, AkaveError>
    where
        R: Read + Send,
    {
        log_debug!(
            "Starting file upload: {} to bucket: {}",
            file_name,
            bucket_name
        );

        let min_file_size = self.min_file_size;
        let block_size = self.block_size;
        let block_part_size = self.block_part_size;

        if bucket_name.is_empty() {
            return Err(AkaveError::InvalidInput("Empty bucket name".to_string()));
        }

        let file_name =
            self.maybe_encrypt_metadata(file_name.to_string(), bucket_name.to_string(), None)?;
        let bucket_name =
            self.maybe_encrypt_metadata(bucket_name.to_string(), "bucket".to_string(), None)?;

        log_info!("File name changed to: {}", &file_name);

        let bucket = self.storage.get_bucket_by_name(bucket_name.clone()).await?;

        {
            let storage = self.storage.clone();
            let fn_ = file_name.to_string();
            let bid = bucket.id;
            self.retry_tx(|| {
                let storage = storage.clone();
                let fn_ = fn_.clone();
                async move {
                    storage.create_file(bid, fn_.clone()).await.map_err(|e| {
                        AkaveError::FileOperationError {
                            operation: "create_file".to_string(),
                            file_name: fn_,
                            message: format!("Failed to create file entry on blockchain: {}", e),
                        }
                    })
                }
            })
            .await?;
        }

        log_info!("File created successfully: {}", &file_name);

        let info = format!("{}/{}", &bucket_name, &file_name);

        let password = match (passwd, &self.default_encryption_key) {
            (Some(p), _) => Some(p),
            (None, Some(default_key)) => Some(default_key.as_str()),
            _ => None,
        };

        let encryption = match password {
            Some(key) => {
                log_debug!("Setting up encryption");
                Some(Encryption::new(key.as_bytes(), &info).map_err(AkaveError::EncryptionError)?)
            }
            None => {
                log_debug!("No encryption key provided");
                None
            }
        };

        let buffer_size = self.get_effective_chunk_size(encryption.is_some());
        log_debug!("Buffer size: {}", buffer_size);

        let mut dag_root = DagRoot::new();
        let mut encode_file_size: usize = 0;
        let mut actual_file_size: usize = 0;
        let mut idx = 0;
        let mut no_data = true;
        let batch_size = self.chunk_batch_size.max(1);

        'outer: loop {
            // Collect up to batch_size prepared chunks before calling the contract.
            let mut batch: Vec<PreparedChunk> = Vec::with_capacity(batch_size);
            let mut eof = false;

            for _ in 0..batch_size {
                let mut buffer = vec![0u8; buffer_size];
                let n = reader
                    .read(&mut buffer)
                    .map_err(|e| AkaveError::FileError(e.to_string()))?;

                if n == 0 {
                    eof = true;
                    break;
                }
                buffer.truncate(n);
                actual_file_size += n;

                if no_data && buffer.len() < min_file_size {
                    return Err(AkaveError::InvalidInput(format!(
                        "File size must be at least {} bytes",
                        min_file_size
                    )));
                }
                no_data = false;

                log_debug!("Processing chunk {} for file: {}", idx, &file_name);
                let original_chunk_size = buffer.len();

                let encrypted_data = match encryption {
                    Some(ref enc) => enc
                        .encrypt(&buffer[..], &format!("{}", idx))
                        .map_err(AkaveError::EncryptionError)?,
                    None => buffer[..].to_vec().into(),
                };

                let processed_data = if let Some(ref ec) = self.erasure_code {
                    ec.encode(&encrypted_data)?
                } else {
                    encrypted_data.to_vec()
                };

                let mut client = self.client.clone();
                let prepared = AkaveSDK::prepare_chunk_grpc(
                    idx,
                    processed_data,
                    original_chunk_size,
                    bucket.id,
                    &file_name,
                    self.erasure_code.as_ref(),
                    block_size,
                    &mut client,
                )
                .await?;
                batch.push(prepared);
                idx += 1;
            }

            if batch.is_empty() {
                break 'outer;
            }

            if no_data {
                return Err(AkaveError::InvalidInput("Empty file".to_string()));
            }

            // Build batch contract call parameters.
            let starting_index = batch[0].chunk_upload.index;
            let batch_cids: Vec<Vec<u8>> = batch
                .iter()
                .map(|p| p.chunk_upload.chunk_cid.to_bytes())
                .collect();
            let batch_encoded_sizes: Vec<U256> = batch
                .iter()
                .map(|p| U256::from(p.chunk_upload.encoded_size))
                .collect();
            let batch_block_cids: Vec<Vec<[u8; 32]>> =
                batch.iter().map(|p| p.cids.clone()).collect();
            let batch_block_sizes: Vec<Vec<U256>> = batch.iter().map(|p| p.sizes.clone()).collect();

            {
                let storage = self.storage.clone();
                let fn_ = file_name.to_string();
                let bc = batch_cids.clone();
                let bid = bucket.id;
                let bes = batch_encoded_sizes.clone();
                let bbcs = batch_block_cids.clone();
                let bbss = batch_block_sizes.clone();
                let si = U256::from(starting_index);
                self.retry_tx(|| {
                    let storage = storage.clone();
                    let fn_ = fn_.clone();
                    let bc = bc.clone();
                    let bes = bes.clone();
                    let bbcs = bbcs.clone();
                    let bbss = bbss.clone();
                    async move {
                        storage
                            .add_file_chunks(bc, bid, fn_.clone(), bes, bbcs, bbss, si)
                            .await
                            .map_err(|e| AkaveError::FileOperationError {
                                operation: "add_file_chunks".to_string(),
                                file_name: fn_,
                                message: format!(
                                    "Failed to register chunk batch on blockchain: {}",
                                    e
                                ),
                            })
                    }
                })
                .await?;
            }

            // Upload blocks for every chunk in the batch.
            for prepared in &batch {
                let chunk = &prepared.chunk_upload;
                let ipc_chunk = &prepared.ipc_chunk;
                encode_file_size += chunk.encoded_size;
                dag_root.add_link(
                    chunk.chunk_cid,
                    chunk.actual_size as u64,
                    chunk.encoded_size as u64,
                );

                // Phase 1: Sign each block sequentially (local crypto, no network I/O).
                struct SignedBlock {
                    data: Vec<u8>,
                    bucket_id: Vec<u8>,
                    file_name: String,
                    block_cid: String,
                    block_index: i64,
                    signature: String,
                    node_id_bytes: Vec<u8>,
                    node_address: String,
                    nonce_bytes: Vec<u8>,
                    deadline_secs: i64,
                    chunk: Option<IpcChunk>,
                }

                let blocks = chunk.blocks.clone();
                let mut signed_blocks: Vec<SignedBlock> = Vec::with_capacity(blocks.len());

                for (index, block_1mb) in blocks.iter().enumerate() {
                    let nonce = crate::get_nonce();
                    let deadline_secs = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .map_err(|e| AkaveError::InternalError(format!("SystemTime error: {}", e)))?
                        .as_secs()
                        .saturating_add(24 * 60 * 60);
                    let deadline = U256::from(deadline_secs);
                    let chunk_cid = cid::Cid::from_str(&ipc_chunk.cid)
                        .map_err(|e| AkaveError::InternalError(e.to_string()))?;
                    let node_id = PeerId::from_str(&block_1mb.node_id)
                        .map_err(|e| AkaveError::InternalError(e.to_string()))?;
                    let node_id_bytes = node_id.to_bytes();
                    if node_id_bytes.len() < 38 {
                        return Err(AkaveError::InternalError(format!(
                            "Invalid peer id bytes length: {}",
                            node_id_bytes.len()
                        )));
                    }
                    // Match Go SDK/Node conversion: bytes32 node id is extracted from bytes[6..38].
                    let mut node_id_32 = [0u8; 32];
                    node_id_32.copy_from_slice(&node_id_bytes[6..38]);
                    let bucket_id_32 = bucket.id.to_bytes();
                    let chain_id = self.chain_id;
                    let (data_message, domain, data_types) = create_block_eip712_data(
                        &block_1mb.cid,
                        &chunk_cid,
                        &node_id_32,
                        &bucket_id_32,
                        self.storage.contract.address(),
                        ipc_chunk.index,
                        index as i64,
                        chain_id,
                        nonce,
                        deadline,
                    )
                    .map_err(|e| AkaveError::InternalError(e.to_string()))?;

                    log_debug!(
                        "Signing data for chunk {}, block {}",
                        ipc_chunk.index,
                        index
                    );
                    let signature = self
                        .storage
                        .client
                        .eip712_sign(domain.clone(), data_message.clone(), data_types.clone())
                        .await
                        .map_err(|e| {
                            AkaveError::InternalError(format!("Failed to sign data: {}", e))
                        })?;

                    let mut nonce_bytes = [0u8; 32];
                    nonce.to_big_endian(&mut nonce_bytes);

                    signed_blocks.push(SignedBlock {
                        data: block_1mb.data.clone(),
                        bucket_id: bucket.id.to_vec(),
                        file_name: file_name.clone(),
                        block_cid: block_1mb.cid.to_string(),
                        block_index: index as i64,
                        signature,
                        node_id_bytes,
                        node_address: block_1mb.node_address.clone(),
                        nonce_bytes: nonce_bytes.to_vec(),
                        deadline_secs: deadline_secs as i64,
                        chunk: Some(ipc_chunk.clone()),
                    });
                }

                // Phase 2: Upload all signed blocks concurrently (semaphore-bounded).
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(
                        self.max_concurrent_blocks,
                    ));
                    let upload_pool = self.connection_pool.clone();
                    let mut join_set = tokio::task::JoinSet::new();
                    for sb in signed_blocks {
                        let permit = semaphore.clone().acquire_owned().await.unwrap();
                        let pool = upload_pool.clone();
                        join_set.spawn(async move {
                            let _permit = permit;
                            AkaveSDK::upload_block_segments(
                                sb.data,
                                sb.bucket_id,
                                sb.file_name,
                                sb.block_cid,
                                sb.block_index,
                                sb.signature,
                                sb.node_id_bytes,
                                &sb.node_address,
                                sb.nonce_bytes,
                                sb.deadline_secs,
                                sb.chunk,
                                block_part_size,
                                pool,
                            )
                            .await
                        });
                    }
                    while let Some(result) = join_set.join_next().await {
                        result.map_err(AkaveError::ThreadJoinError)?.map_err(|e| {
                            AkaveError::InternalError(format!(
                                "Failed to upload block segments: {}",
                                e
                            ))
                        })?;
                    }
                }
                #[cfg(target_arch = "wasm32")]
                {
                    for sb in signed_blocks {
                        AkaveSDK::upload_block_segments(
                            sb.data,
                            sb.bucket_id,
                            sb.file_name,
                            sb.block_cid,
                            sb.block_index,
                            sb.signature,
                            sb.node_id_bytes,
                            &sb.node_address,
                            sb.nonce_bytes,
                            sb.deadline_secs,
                            sb.chunk,
                            block_part_size,
                        )
                        .await
                        .map_err(|e| {
                            AkaveError::InternalError(format!(
                                "Failed to upload block segments: {}",
                                e
                            ))
                        })?;
                    }
                }
            }

            if eof {
                break 'outer;
            }
        }

        // Build root CID using the proper UnixFS MerkleDAG root (matches Go DAGRoot).
        // DagRoot::build() returns the single chunk's CID for 1 chunk, or constructs
        // a canonical dag-pb PBNode with links to all chunk CIDs for multiple chunks.
        let root_cid = dag_root.build().map_err(AkaveError::InternalError)?;

        // Calculate file ID for blockchain operations
        let file_id = Self::calculate_file_id(&bucket.id, &file_name);

        // Wait for file to be filled before committing (matches Go SDK behavior)
        self.wait_for_file_filled(file_id, &file_name).await?;

        log_info!("File is filled, committing: {}", &file_name);
        let storage = self.storage.clone();
        let fn_commit = file_name.to_string();
        let rc_bytes = root_cid.to_bytes();
        let receipt = self
            .retry_tx(|| {
                let storage = storage.clone();
                let fn_ = fn_commit.clone();
                let rc = rc_bytes.clone();
                async move {
                    storage
                        .commit_file(
                            bucket.id,
                            fn_.clone(),
                            U256::from(encode_file_size),
                            U256::from(actual_file_size),
                            rc,
                        )
                        .await
                        .map_err(|e| AkaveError::FileOperationError {
                            operation: "commit_file".to_string(),
                            file_name: fn_,
                            message: format!(
                                "Failed to commit file to blockchain. Encoded size: {}, Actual size: {}, Error: {}",
                                encode_file_size, actual_file_size, e
                            ),
                        })
                }
            })
            .await?;

        log_info!(
            "File uploaded successfully: {} to bucket: {}",
            file_name,
            bucket_name
        );
        Ok(receipt)
    }

    /// Prepares a chunk (DAG + gRPC FileUploadChunkCreate) without touching the contract.
    #[allow(clippy::too_many_arguments)]
    async fn prepare_chunk_grpc(
        index: usize,
        data: Vec<u8>,
        original_size: usize,
        bucket_id: BucketId,
        file_name: &str,
        erasure_code: Option<&ErasureCode>,
        block_size: usize,
        client: &mut IpcNodeApiClient<ClientTransport>,
    ) -> Result<PreparedChunk, AkaveError> {
        let size = data.len();
        let block_size = if let Some(ec) = erasure_code {
            size / (ec.data_blocks + ec.parity_blocks)
        } else {
            block_size
        };

        let chunk_dag = ChunkDag::new(block_size, data);
        let mut cids: Vec<[u8; 32]> = vec![];
        let mut sizes = vec![];
        let mut chunk_blocks = vec![];

        for block in chunk_dag.blocks.iter() {
            let block_cid: [u8; 32] = block.cid.to_bytes()[4..36]
                .to_vec()
                .try_into()
                .map_err(|e| AkaveError::InvalidInput(format!("Error formatting cid: {:?}", e)))?;
            chunk_blocks.push(Block {
                cid: block.cid.to_string(),
                size: block.data.len() as i64,
            });
            cids.push(block_cid);
            sizes.push(U256::from(block.data.len()));
        }

        let chunk_cid = chunk_dag.cid;
        let encoded_size = chunk_dag.encoded_size;
        let mut upload_blocks = chunk_dag.blocks;

        let ipc_chunk = IpcChunk {
            cid: chunk_cid.to_string(),
            index: index as i64,
            size: original_size as i64,
            blocks: chunk_blocks,
        };

        let chunk_create_response = client
            .file_upload_chunk_create(IpcFileUploadChunkCreateRequest {
                chunk: Some(ipc_chunk.clone()),
                bucket_id: bucket_id.to_vec(),
                file_name: file_name.to_string(),
            })
            .await
            .map_err(|e| AkaveError::GrpcError(Box::new(e)))?
            .into_inner();

        chunk_create_response
            .blocks
            .iter()
            .enumerate()
            .for_each(|(idx, block)| {
                upload_blocks[idx].node_address = block.node_address.clone();
                upload_blocks[idx].node_id = block.node_id.clone();
                upload_blocks[idx].permit = block.permit.clone();
            });

        Ok(PreparedChunk {
            chunk_upload: IpcFileChunkUpload {
                index,
                chunk_cid,
                actual_size: original_size,
                raw_data_size: original_size,
                encoded_size,
                blocks: upload_blocks,
                bucket_id,
                file_name: file_name.to_string(),
            },
            ipc_chunk,
            cids,
            sizes,
        })
    }

    /// Upload a block in segments, similar to uploadIpcBlockSegments in the Go implementation
    ///
    /// The function splits the data into smaller segments based on block_part_size
    /// and only includes metadata with the first segment.
    ///
    /// For WASM environments, it sends requests sequentially.
    /// For native environments, it processes blocks concurrently.
    #[allow(clippy::too_many_arguments)]
    async fn upload_block_segments(
        data: Vec<u8>,
        bucket_id: Vec<u8>,
        file_name: String,
        block_cid: String,
        block_index: i64,
        signature: String,
        node_id: Vec<u8>,
        node_address: &str,
        nonce: Vec<u8>,
        deadline: i64,
        chunk: Option<IpcChunk>,
        block_part_size: usize,
        #[cfg(not(target_arch = "wasm32"))] pool: Arc<
            tokio::sync::RwLock<std::collections::HashMap<String, Channel>>,
        >,
    ) -> Result<(), AkaveError> {
        let data_len = data.len();
        if data_len == 0 {
            return Ok(());
        }

        log_debug!(
            "Uploading block segments. CID: {}, length: {}, block index: {}, part size: {}",
            block_cid,
            data_len,
            block_index,
            block_part_size
        );

        #[cfg(target_arch = "wasm32")]
        {
            let block_data = IpcFileBlockData {
                bucket_id: bucket_id.clone(),
                data: data,
                cid: block_cid.clone(),
                chunk: chunk.clone(),
                file_name: file_name.clone(),
                index: block_index,
                signature: signature.clone(),
                node_id: node_id.clone(),
                nonce: nonce.clone(),
                deadline,
            };

            log_debug!("Uploading block {}", block_index);
            let mut node_client = AkaveSDK::get_client_for_node_address(node_address)
                .await
                .map_err(|e| AkaveError::GrpcError(Box::new(e)))?; // WASM: no pool
            node_client
                .file_upload_block_unary(block_data)
                .await
                .map_err(|e| {
                    log_error!("Error uploading block: {}", e);
                    AkaveError::GrpcError(Box::new(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Failed to upload block: {}", e),
                    )))
                })?
                .into_inner();
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            // Create a stream that generates block data on demand
            let stream = tokio_stream::iter(0..(data_len.div_ceil(block_part_size))).map(
                move |segment_index| {
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
                        deadline,
                    }
                },
            );

            // Send the stream to the server
            log_debug!(
                "Streaming block segments for block {} to node {}",
                block_index,
                node_address
            );
            let mut node_client = AkaveSDK::get_client_via_pool(node_address, &pool)
                .await
                .map_err(|e| AkaveError::GrpcError(Box::new(e)))?;
            match node_client.file_upload_block(stream).await {
                Ok(response) => {
                    response.into_inner();
                }
                Err(e) => {
                    log_error!("Error uploading block: {}", e);
                    return Err(AkaveError::GrpcError(Box::new(e)));
                }
            }
        }

        log_debug!("Block segments uploaded successfully");
        Ok(())
    }

    async fn create_file_download(
        &self,
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
            .clone()
            .file_download_create(request)
            .await
            .map_err(|e| AkaveError::GrpcError(Box::new(e)))?
            .into_inner();

        let chunks = response
            .chunks
            .into_iter()
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

    async fn create_file_download_range(
        &self,
        address: &str,
        bucket_name: &str,
        file_name: &str,
        start_chunk: i64,
        end_chunk: i64,
    ) -> Result<FileDownloadResponse, AkaveError> {
        log_debug!(
            "Creating file download range for: {} in bucket: {} (chunks {}-{})",
            file_name,
            bucket_name,
            start_chunk,
            end_chunk
        );

        let request = IpcFileDownloadRangeCreateRequest {
            bucket_name: bucket_name.to_string(),
            file_name: file_name.to_string(),
            address: address.to_string(),
            start_index: start_chunk,
            end_index: end_chunk,
        };

        let response = self
            .client
            .clone()
            .file_download_range_create(request)
            .await
            .map_err(|e| AkaveError::GrpcError(Box::new(e)))?
            .into_inner();

        let chunks = response
            .chunks
            .into_iter()
            .map(|chunk| FileChunk {
                cid: chunk.cid,
                size: chunk.size,
                encoded_size: chunk.encoded_size,
                blocks: vec![],
            })
            .collect();

        log_info!(
            "File download range created successfully for: {} (chunks {}-{})",
            file_name,
            start_chunk,
            end_chunk
        );
        Ok(FileDownloadResponse { chunks })
    }

    /// Get the chunk size accounting for encryption overhead
    pub fn get_effective_chunk_size(&self, with_encryption: bool) -> usize {
        let base_size = if let Some(erasure_code) = &self.erasure_code {
            erasure_code.data_blocks * self.block_size
        } else {
            self.block_size * self.max_blocks_in_chunk
        };

        if with_encryption {
            base_size - utils::encryption::OVERHEAD
        } else {
            base_size
        }
    }

    /// Check if the SDK has default encryption configured
    pub fn has_default_encryption(&self) -> bool {
        self.default_encryption_key.is_some()
    }

    /// Helper method to setup encryption for downloads
    fn setup_download_encryption(
        &self,
        passwd: Option<&str>,
        info: &str,
    ) -> Result<Option<Encryption>, AkaveError> {
        let password = match (passwd, &self.default_encryption_key) {
            (Some(p), _) => Some(p),
            (None, Some(default_key)) => Some(default_key.as_str()),
            _ => None,
        };

        match password {
            Some(key) => {
                log_debug!("Setting up decryption key");
                Ok(Some(
                    Encryption::new(key.as_bytes(), &info).map_err(AkaveError::EncryptionError)?,
                ))
            }
            None => {
                log_debug!("No decryption key provided");
                Ok(None)
            }
        }
    }

    pub async fn download_file<W>(
        self: Arc<Self>,
        bucket_name: &str,
        file_name: &str,
        passwd: Option<&str>,
        mut writer: W,
    ) -> Result<W, AkaveError>
    where
        W: Write + Send + 'static,
    {
        // Struct for holding block data
        struct BlockData {
            index: usize,
            data: Vec<u8>,
        }

        let file_name =
            self.maybe_encrypt_metadata(file_name.to_string(), bucket_name.to_string(), None)?;
        let bucket_name =
            self.maybe_encrypt_metadata(bucket_name.to_string(), "bucket".to_string(), None)?;

        let address = self.storage.client.get_hex_address().await?;
        let info = [bucket_name.as_str(), file_name.as_str()].join("/");
        let option_encryption = self.setup_download_encryption(passwd, &info)?;

        let file_download = self
            .create_file_download(&address, &bucket_name, &file_name)
            .await?;

        if file_download.chunks.is_empty() {
            return Err(AkaveError::FileOperationError {
                operation: "download_file".to_string(),
                file_name: file_name.to_string(),
                message: "File has no chunks".to_string(),
            });
        }

        let codec = Cid::try_from(file_download.chunks[0].cid.clone())
            .map_err(|e| AkaveError::InvalidInput(e.to_string()))?
            .codec();

        let mut chunks_iter = file_download.chunks.into_iter();

        // Prepare the first future
        let first_chunk = chunks_iter.next().unwrap();

        #[cfg(not(target_arch = "wasm32"))]
        let mut current_future = {
            let this = self.clone();
            let bucket_name = bucket_name.clone();
            let file_name = file_name.clone();
            let address = address.clone();
            tokio::spawn(async move {
                this.create_chunk_download(&bucket_name, &file_name, &address, first_chunk, 0)
                    .await
            })
        };

        #[cfg(target_arch = "wasm32")]
        let mut current_future: Pin<
            Box<dyn Future<Output = Result<FileChunkDownload, AkaveError>>>,
        > = {
            let this = self.clone();
            let bucket_name = bucket_name.clone();
            let file_name = file_name.clone();
            let address = address.clone();
            Box::pin(async move {
                this.create_chunk_download(&bucket_name, &file_name, &address, first_chunk, 0)
                    .await
            })
        };

        let mut chunk_index = 0;

        loop {
            #[cfg(not(target_arch = "wasm32"))]
            let next_future_opt = if let Some(next_chunk) = chunks_iter.next() {
                let this = self.clone();
                let bucket_name = bucket_name.clone();
                let file_name = file_name.clone();
                let address = address.clone();
                let next_index = chunk_index + 1;
                Some(tokio::spawn(async move {
                    this.create_chunk_download(
                        &bucket_name,
                        &file_name,
                        &address,
                        next_chunk,
                        next_index as i64,
                    )
                    .await
                }))
            } else {
                None
            };

            #[cfg(target_arch = "wasm32")]
            let next_future_opt: Option<
                Pin<Box<dyn Future<Output = Result<FileChunkDownload, AkaveError>>>>,
            > = if let Some(next_chunk) = chunks_iter.next() {
                let this = self.clone();
                let bucket_name = bucket_name.clone();
                let file_name = file_name.clone();
                let address = address.clone();
                let next_index = chunk_index + 1;
                Some(Box::pin(async move {
                    this.create_chunk_download(
                        &bucket_name,
                        &file_name,
                        &address,
                        next_chunk,
                        next_index as i64,
                    )
                    .await
                }))
            } else {
                None
            };

            // Await the current future
            #[cfg(not(target_arch = "wasm32"))]
            let chunk_download = current_future
                .await
                .map_err(|e| AkaveError::FileError(format!("Join error: {:?}", e)))??;

            #[cfg(target_arch = "wasm32")]
            let chunk_download = current_future.await?;

            // Capture original chunk size before consuming chunk_download.
            // chunk_download.size is the pre-erasure/pre-encryption size stored by
            // the node from the IPCChunk.size field we set during upload.
            let original_chunk_size = chunk_download.size as usize;

            // --- Concurrent block downloads inside the chunk ---
            #[cfg(not(target_arch = "wasm32"))]
            let block_pool = self.connection_pool.clone();
            let mut block_futures = Vec::new();
            for (block_index, block) in chunk_download.blocks.into_iter().enumerate() {
                let address = address.clone();
                let chunk_cid = chunk_download.cid.clone();
                let bucket_name = bucket_name.clone();
                let file_name = file_name.clone();
                #[cfg(not(target_arch = "wasm32"))]
                let block_pool = block_pool.clone();

                block_futures.push(async move {
                    let mut chunk_data = vec![];
                    let req = IpcFileDownloadBlockRequest {
                        address: address.to_string(),
                        chunk_cid: chunk_cid.clone(),
                        chunk_index: chunk_index as i64,
                        block_cid: block.cid.clone(),
                        block_index: block_index as i64,
                        bucket_name: bucket_name.clone(),
                        file_name: file_name.clone(),
                    };
                    log_debug!(
                        "Downloading block {} for chunk {}",
                        block_index,
                        chunk_index
                    );
                    #[cfg(not(target_arch = "wasm32"))]
                    let mut node_client =
                        AkaveSDK::get_client_via_pool(&block.node_address, &block_pool)
                            .await
                            .map_err(|e| AkaveError::GrpcError(Box::new(e)))?;
                    #[cfg(target_arch = "wasm32")]
                    let mut node_client =
                        AkaveSDK::get_client_for_node_address(&block.node_address)
                            .await
                            .map_err(|e| AkaveError::GrpcError(Box::new(e)))?;
                    let mut stream = node_client
                        .file_download_block(req)
                        .await
                        .map_err(|e| AkaveError::GrpcError(Box::new(e)))?
                        .into_inner();

                    while let Some(mut message) = stream
                        .message()
                        .await
                        .map_err(|e| AkaveError::GrpcError(Box::new(e)))?
                    {
                        chunk_data.append(message.data.as_mut());
                    }

                    let final_data = AkaveSDK::use_download_codec(codec, chunk_data)?;

                    Ok::<BlockData, AkaveError>(BlockData {
                        index: block_index,
                        data: final_data,
                    })
                });
            }

            // Platform-specific concurrency handling for blocks
            let mut block_results: Vec<BlockData>;
            #[cfg(not(target_arch = "wasm32"))]
            {
                let semaphore = Arc::new(Semaphore::new(self.max_concurrent_blocks));
                let mut handles = Vec::new();
                for fut in block_futures {
                    let permit = semaphore
                        .clone()
                        .acquire_owned()
                        .await
                        .map_err(|e| AkaveError::InternalError(e.to_string()))?;
                    handles.push(tokio::spawn(async move {
                        let _permit = permit;
                        fut.await
                    }));
                }
                let mut results = Vec::new();
                for h in handles {
                    results.push(h.await??);
                }
                block_results = results;
            }
            #[cfg(target_arch = "wasm32")]
            {
                use futures::future::try_join_all;
                block_results = try_join_all(block_futures).await?;
            }

            // Sort blocks by index
            block_results.sort_by_key(|b| b.index);

            // Extract block data
            let block_data_vecs: Vec<Vec<u8>> = block_results.into_iter().map(|b| b.data).collect();

            // Combine blocks into a chunk
            let processed_data = if let Some(erasure_code) = &self.erasure_code {
                erasure_code.extract_data_raw(block_data_vecs, original_chunk_size)?
            } else {
                block_data_vecs.concat()
            };

            // Decrypt if needed
            let final_data = match &option_encryption {
                Some(encryption) => {
                    log_info!("Decrypting chunk: {}", chunk_index);
                    encryption
                        .decrypt(&processed_data, &format!("{}", chunk_index))
                        .map_err(AkaveError::EncryptionError)?
                }
                None => processed_data,
            };

            // Write chunk sequentially
            writer
                .write_all(&final_data)
                .map_err(|e| AkaveError::FileError(e.to_string()))?;

            // If there is no next future, we are done
            if let Some(next_future) = next_future_opt {
                current_future = next_future;
                chunk_index += 1;
            } else {
                break;
            }
        }

        Ok(writer)
    }

    /// Download a specific byte range from a file with concurrent chunk downloads
    pub async fn download_file_range<W: Write + Send>(
        self: Arc<Self>,
        bucket_name: &str,
        file_name: &str,
        start_offset: u64,
        end_offset: u64,
        passwd: Option<&str>,
        mut writer: W,
    ) -> Result<W, AkaveError> {
        let address = self.storage.client.get_hex_address().await?;

        let file_name =
            self.maybe_encrypt_metadata(file_name.to_string(), bucket_name.to_string(), None)?;
        let bucket_name =
            self.maybe_encrypt_metadata(bucket_name.to_string(), "bucket".to_string(), None)?;

        let info = [bucket_name.as_str(), file_name.as_str()].join("/");
        let option_encryption = self.setup_download_encryption(passwd, &info)?;
        let chunk_size = self.get_effective_chunk_size(option_encryption.is_some()) as u64;

        // Calculate chunk indices from byte offsets
        let start_chunk = (start_offset / chunk_size) as i64;
        let end_chunk = ((end_offset - 1) / chunk_size) as i64;

        // Create range download request
        let file_download = self
            .create_file_download_range(
                &address,
                &bucket_name,
                &file_name,
                start_chunk,
                end_chunk + 1,
            )
            .await?;

        if file_download.chunks.is_empty() {
            return Err(AkaveError::FileOperationError {
                operation: "download_file_range".to_string(),
                file_name: file_name.to_string(),
                message: "No chunks in specified range".to_string(),
            });
        }

        let codec = Cid::try_from(file_download.chunks[0].cid.clone())
            .map_err(|e| AkaveError::InvalidInput(e.to_string()))?
            .codec();

        // Process chunks concurrently
        use futures::stream::FuturesOrdered;

        let mut futures = FuturesOrdered::new();

        for (relative_idx, chunk) in file_download.chunks.into_iter().enumerate() {
            let chunk_index = start_chunk + relative_idx as i64;
            let chunk_cid = chunk.cid.clone();
            let chunk_size = chunk.size;

            let address = address.clone();
            let bucket_name = bucket_name.clone();
            let file_name = file_name.clone();
            let erasure_code = self.erasure_code.clone();

            // Clone what we need for the async block
            let self_clone = self.clone();

            futures.push_back(async move {
                log_debug!("Processing chunk {} for file: {}", chunk_index, file_name);

                let chunk_download = self_clone
                    .create_chunk_download(&bucket_name, &file_name, &address, chunk, chunk_index)
                    .await?;

                // Download all blocks for this chunk concurrently
                use futures::stream::FuturesUnordered;
                let mut block_futures = FuturesUnordered::new();

                // Create semaphore for concurrency limiting
                let semaphore = Arc::new(Semaphore::new(self_clone.max_concurrent_blocks));

                for (block_index, block) in chunk_download.blocks.into_iter().enumerate() {
                    let address = address.clone();
                    let chunk_cid = chunk_cid.clone();
                    let bucket_name = bucket_name.clone();
                    let file_name = file_name.clone();
                    let semaphore = semaphore.clone();
                    let sdk = self_clone.clone();

                    block_futures.push(async move {
                        // Acquire semaphore permit to limit concurrency
                        let _permit = semaphore.acquire().await.map_err(|e| {
                            AkaveError::InternalError(format!("Failed to acquire semaphore: {}", e))
                        })?;
                        let req = IpcFileDownloadBlockRequest {
                            address: address.clone(),
                            chunk_cid: chunk_cid.clone(),
                            chunk_index,
                            block_cid: block.cid.clone(),
                            block_index: block_index as i64,
                            bucket_name: bucket_name.clone(),
                            file_name: file_name.clone(),
                        };

                        log_debug!(
                            "Downloading block {} for chunk {}",
                            block_index,
                            chunk_index
                        );

                        let mut node_client = sdk
                            .get_node_client(&block.node_address)
                            .await
                            .map_err(|e| AkaveError::GrpcError(Box::new(e)))?;

                        let mut stream = node_client
                            .file_download_block(req)
                            .await
                            .map_err(|e| AkaveError::GrpcError(Box::new(e)))?
                            .into_inner();

                        let mut chunk_data = vec![];
                        while let Some(mut message) = stream
                            .message()
                            .await
                            .map_err(|e| AkaveError::GrpcError(Box::new(e)))?
                        {
                            chunk_data.append(message.data.as_mut());
                        }

                        // Process block data based on codec
                        let final_data = AkaveSDK::use_download_codec(codec, chunk_data)?;

                        Ok::<(usize, Vec<u8>), AkaveError>((block_index, final_data))
                    });
                }

                // Collect all blocks for this chunk (unordered)
                let mut block_results = Vec::new();

                while let Some(result) = futures::StreamExt::next(&mut block_futures).await {
                    block_results.push(result?);
                }

                // Sort blocks by index to ensure correct order
                block_results.sort_by_key(|(idx, _)| *idx);

                // Extract just the data in the correct order
                let blocks_data: Vec<Vec<u8>> =
                    block_results.into_iter().map(|(_, data)| data).collect();

                // Process with erasure coding if enabled
                let processed_data = if let Some(erasure_code) = &erasure_code {
                    erasure_code.extract_data_raw(blocks_data.clone(), chunk_size as usize)?
                } else {
                    blocks_data.concat()
                };

                Ok::<(i64, Vec<u8>), AkaveError>((chunk_index, processed_data))
            });
        }

        // Process chunks in order and handle offset trimming
        let offset_in_first_chunk = (start_offset % chunk_size) as usize;
        let total_bytes_needed = (end_offset - start_offset) as usize;
        let mut bytes_written = 0;

        while let Some(result) = futures::StreamExt::next(&mut futures).await {
            let (chunk_index, chunk_data) = result?;

            // Decrypt if needed
            let decrypted_data = match &option_encryption {
                Some(encryption) => {
                    log_info!("Decrypting chunk: {}", chunk_index);
                    encryption
                        .decrypt(&chunk_data, &format!("{}", chunk_index))
                        .map_err(AkaveError::EncryptionError)?
                }
                None => chunk_data,
            };

            // Calculate what portion of this chunk to write
            let mut data_to_write = &decrypted_data[..];

            // If this is the first chunk, skip to the offset
            if chunk_index == start_chunk {
                if offset_in_first_chunk < data_to_write.len() {
                    data_to_write = &data_to_write[offset_in_first_chunk..];
                } else {
                    continue; // Skip this chunk if offset is beyond it
                }
            }

            // If we would write more than needed, trim the data
            let remaining_bytes = total_bytes_needed - bytes_written;
            if data_to_write.len() > remaining_bytes {
                data_to_write = &data_to_write[..remaining_bytes];
            }

            // Write the data
            writer
                .write_all(data_to_write)
                .map_err(|e| AkaveError::FileError(e.to_string()))?;

            bytes_written += data_to_write.len();

            // Stop if we've written all requested bytes
            if bytes_written >= total_bytes_needed {
                break;
            }
        }

        Ok(writer)
    }

    async fn create_chunk_download(
        &self,
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
            chunk_index: index,
            address: address.to_string(),
        };
        let resp = self
            .client
            .clone()
            .file_download_chunk_create(request)
            .await
            .map_err(|e| AkaveError::GrpcError(Box::new(e)))?
            .into_inner();
        let mut blocks = vec![];
        for block in resp.blocks {
            blocks.push(FileBlockDownload {
                cid: block.cid,
                data: Vec::new(),
                node_id: block.node_id,
                permit: block.permit,
                node_address: block.node_address,
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

    /// Returns a gRPC client for the given node address, reusing a cached channel when the
    /// connection pool is enabled (native only). Falls back to a fresh connection otherwise.
    async fn get_node_client(
        &self,
        node_address: &str,
    ) -> Result<IpcNodeApiClient<ClientTransport>, AkaveError> {
        #[cfg(not(target_arch = "wasm32"))]
        return Self::get_client_via_pool(node_address, &self.connection_pool).await;
        #[cfg(target_arch = "wasm32")]
        Self::get_client_for_node_address(node_address).await
    }

    #[cfg(not(target_arch = "wasm32"))]
    async fn get_client_via_pool(
        node_address: &str,
        pool: &Arc<tokio::sync::RwLock<std::collections::HashMap<String, Channel>>>,
    ) -> Result<IpcNodeApiClient<ClientTransport>, AkaveError> {
        let address = if !node_address.starts_with("http://") {
            format!("http://{}", node_address)
        } else {
            node_address.to_string()
        };

        // Fast path: channel already cached
        {
            let guard = pool.read().await;
            if let Some(ch) = guard.get(&address) {
                return Ok(IpcNodeApiClient::new(ch.clone())
                    .max_decoding_message_size(usize::MAX)
                    .max_encoding_message_size(usize::MAX));
            }
        }

        // Slow path: create, cache, return
        let tls_config = ClientTlsConfig::new().with_native_roots();
        let channel = Channel::from_shared(address.clone())
            .map_err(|e| AkaveError::ChannelError(e.to_string()))?
            .tls_config(tls_config)
            .map_err(|e| AkaveError::ChannelError(e.to_string()))?
            .connect()
            .await
            .map_err(|e| {
                AkaveError::GrpcError(Box::new(std::io::Error::new(
                    std::io::ErrorKind::ConnectionRefused,
                    format!("Failed to connect to node {}: {}", address, e),
                )))
            })?;
        pool.write().await.insert(address, channel.clone());
        Ok(IpcNodeApiClient::new(channel)
            .max_decoding_message_size(usize::MAX)
            .max_encoding_message_size(usize::MAX))
    }

    async fn get_client_for_node_address(
        node_address: &str,
    ) -> Result<IpcNodeApiClient<ClientTransport>, AkaveError> {
        #[cfg(target_arch = "wasm32")]
        {
            // Parse node address and modify port
            let parts: Vec<&str> = node_address.split(':').collect();
            if parts.len() != 2 {
                return Err(AkaveError::InvalidInput(
                    "Invalid node address format, expected IP:PORT".to_string(),
                ));
            }

            let host = parts[0];
            let port = parts[1]
                .parse::<u16>()
                .map_err(|_| AkaveError::InvalidInput("Invalid port number".to_string()))?;

            let address = format!("http://{}:{}/grpc", host, port + 2000);

            log_debug!("Connecting to node at address: {}", address);
            let grpc_web_client = ClientTransport::new(address.into());
            let client = IpcNodeApiClient::new(grpc_web_client);
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

            let channel = Channel::from_shared(address.clone())
                .map_err(|e| AkaveError::ChannelError(e.to_string()))?
                .tls_config(tls_config)
                .map_err(|e| AkaveError::ChannelError(e.to_string()))?
                .connect()
                .await
                .map_err(|e| {
                    AkaveError::GrpcError(Box::new(std::io::Error::new(
                        std::io::ErrorKind::ConnectionRefused,
                        format!("Failed to connect to node {}: {}", address, e),
                    )))
                })?;

            let client = IpcNodeApiClient::new(channel)
                .max_decoding_message_size(usize::MAX)
                .max_encoding_message_size(usize::MAX);
            Ok(client)
        }
    }

    pub async fn set_file_public_access(
        &self,
        bucket_name: &str,
        file_name: &str,
        is_public: bool,
    ) -> Result<(), AkaveError> {
        let file_name =
            self.maybe_encrypt_metadata(file_name.to_string(), bucket_name.to_string(), None)?;
        let bucket_name =
            self.maybe_encrypt_metadata(bucket_name.to_string(), "bucket".to_string(), None)?;

        let bucket = self.storage.get_bucket_by_name(bucket_name).await?;

        let file = self.storage.get_file_by_name(bucket.id, file_name).await?;

        self.access_manager
            .change_public_access(file.id.to_bytes(), is_public)
            .await?;

        Ok(())
    }

    // Decrypts metadata that was encrypted with maybe_encrypt_metadata.
    fn maybe_decrypt_metadata(
        &self,
        value: &str,
        derivation_path: &str,
    ) -> Result<String, AkaveError> {
        if !self.use_metadata_encryption {
            return Ok(value.to_string());
        }
        let key = match &self.default_encryption_key {
            Some(k) => k.as_str(),
            None => return Ok(value.to_string()),
        };
        let encrypted_bytes = hex::decode(value)
            .map_err(|e| AkaveError::InvalidInput(format!("hex decode failed: {}", e)))?;
        let encryption = Encryption::new(key.as_bytes(), derivation_path)
            .map_err(AkaveError::EncryptionError)?;
        let plaintext = encryption
            .decrypt_deterministic(&encrypted_bytes, derivation_path)
            .map_err(AkaveError::EncryptionError)?;
        String::from_utf8(plaintext)
            .map_err(|e| AkaveError::InvalidInput(format!("utf8 decode failed: {}", e)))
    }

    // Encrypts the given metadata if metadata encryption is enabled and encryption key is set or password is given.
    fn maybe_encrypt_metadata(
        &self,
        value: String,
        derivation_path: String,
        passwd: Option<&str>,
    ) -> Result<String, AkaveError> {
        let password = match (passwd, &self.default_encryption_key) {
            (Some(p), _) => Some(p),
            (None, Some(default_key)) => Some(default_key.as_str()),
            _ => None,
        };

        match self.use_metadata_encryption {
            true => {
                let encryption = match password {
                    Some(key) => Some(
                        Encryption::new(key.as_bytes(), &derivation_path)
                            .map_err(AkaveError::EncryptionError)?,
                    ),
                    None => {
                        log_debug!("No encryption key provided");
                        None
                    }
                };
                match encryption {
                    Some(ref encryption) => {
                        let encrypted = encryption
                            .encrypt_deterministic(value.as_bytes(), &derivation_path)
                            .map_err(AkaveError::EncryptionError)?;
                        Ok(hex::encode(encrypted))
                    }

                    None => Ok(value),
                }
            }
            false => Ok(value),
        }
    }

    /// Calls IPCArchivalAPI.FileResolveBlock to get PDP location for a block.
    #[cfg(not(target_arch = "wasm32"))]
    async fn resolve_block(
        &self,
        node_address: &str,
        block_cid: &str,
    ) -> Result<PDPBlockData, AkaveError> {
        let address =
            if !node_address.starts_with("http://") && !node_address.starts_with("https://") {
                format!("http://{}", node_address)
            } else {
                node_address.to_string()
            };

        let tls_config = tonic::transport::ClientTlsConfig::new().with_native_roots();
        let channel = tonic::transport::Channel::from_shared(address.clone())
            .map_err(|e| AkaveError::ChannelError(e.to_string()))?
            .tls_config(tls_config)
            .map_err(|e| AkaveError::ChannelError(e.to_string()))?
            .connect()
            .await
            .map_err(|e| {
                AkaveError::ChannelError(format!("archival connect to {}: {}", address, e))
            })?;

        let mut client = IpcArchivalApiClient::new(channel);
        let resp = client
            .file_resolve_block(IpcFileResolveBlockRequest {
                block_cid: block_cid.to_string(),
            })
            .await;

        match resp {
            Ok(r) => {
                let pdp = r.into_inner().block.unwrap_or_default();
                Ok(PDPBlockData {
                    url: pdp.url,
                    offset: pdp.offset,
                    size: pdp.size,
                    data_set_id: pdp.data_set_id,
                })
            }
            Err(e) if e.code() == tonic::Code::NotFound => Err(AkaveError::NotFound(format!(
                "archival block missing: {}",
                block_cid
            ))),
            Err(e) => Err(AkaveError::GrpcError(Box::new(e))),
        }
    }

    /// Downloads chunk blocks via the PDP path (range HTTP download + CID verify).
    #[cfg(not(target_arch = "wasm32"))]
    async fn download_chunk_blocks2(
        &self,
        chunk_download: &FileChunkDownload,
        encryption: Option<&crate::utils::encryption::Encryption>,
        writer: &mut impl std::io::Write,
    ) -> Result<(), AkaveError> {
        let http_client = self
            .http_client
            .clone()
            .unwrap_or_else(reqwest::Client::new);
        let semaphore = Arc::new(Semaphore::new(self.max_concurrent_blocks));
        let mut handles = Vec::new();

        for block in chunk_download.blocks.iter() {
            let pdp_data = self.resolve_block(&block.node_address, &block.cid).await?;
            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let client_clone = http_client.clone();
            let block_cid = block.cid.clone();
            let url = pdp_data.url.clone();
            let offset = pdp_data.offset;
            let size = pdp_data.size;
            let handle = tokio::spawn(async move {
                let _permit = permit;
                let data =
                    crate::utils::http_ext::range_download(&client_clone, &url, offset, size)
                        .await?;
                crate::utils::cids::verify_raw(&block_cid, &data)?;
                Ok::<Vec<u8>, AkaveError>(data)
            });
            handles.push(handle);
        }

        let mut block_data_vecs: Vec<Vec<u8>> = Vec::with_capacity(handles.len());
        for handle in handles {
            let raw = handle.await.map_err(AkaveError::ThreadJoinError)??;
            let block_data = Self::use_download_codec(
                cid::Cid::from_str(&chunk_download.cid)
                    .map_err(|e| AkaveError::BlockError(e.to_string()))?
                    .codec(),
                raw,
            )?;
            block_data_vecs.push(block_data);
        }

        let original_chunk_size = chunk_download.size as usize;
        let mut data = if let Some(ec) = &self.erasure_code {
            ec.extract_data_raw(block_data_vecs, original_chunk_size)?
        } else {
            block_data_vecs.concat()
        };

        if let Some(enc) = encryption {
            data = enc
                .decrypt(&data, &format!("{}", chunk_download.index))
                .map_err(AkaveError::EncryptionError)?;
        }

        writer.write_all(&data)?;
        Ok(())
    }

    /// Downloads a file via the archival/PDP path.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn download_archival<W: std::io::Write>(
        &self,
        bucket_name: &str,
        file_name: &str,
        passwd: Option<&str>,
        mut writer: W,
    ) -> Result<W, AkaveError> {
        let file_name_enc =
            self.maybe_encrypt_metadata(file_name.to_string(), bucket_name.to_string(), None)?;
        let bucket_name_enc =
            self.maybe_encrypt_metadata(bucket_name.to_string(), "bucket".to_string(), None)?;

        let address = self.storage.client.get_hex_address().await?;
        let info = [bucket_name_enc.as_str(), file_name_enc.as_str()].join("/");

        let encryption = self.setup_download_encryption(passwd, &info)?;

        let file_download = self
            .create_file_download(&address, &bucket_name_enc, &file_name_enc)
            .await
            .map_err(|e| AkaveError::GrpcError(Box::new(e)))?;

        for (chunk_index, chunk) in file_download.chunks.into_iter().enumerate() {
            let chunk_download = self
                .create_chunk_download(
                    &bucket_name_enc,
                    &file_name_enc,
                    &address,
                    chunk,
                    chunk_index as i64,
                )
                .await?;
            self.download_chunk_blocks2(&chunk_download, encryption.as_ref(), &mut writer)
                .await?;
        }
        Ok(writer)
    }

    /// Returns archival metadata (with optional PDP location) for a file.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn archival_metadata(
        &self,
        bucket_name: &str,
        file_name: &str,
    ) -> Result<ArchivalMetadata, AkaveError> {
        let file_name_enc =
            self.maybe_encrypt_metadata(file_name.to_string(), bucket_name.to_string(), None)?;
        let bucket_name_enc =
            self.maybe_encrypt_metadata(bucket_name.to_string(), "bucket".to_string(), None)?;

        let address = self.storage.client.get_hex_address().await?;

        let file_download = self
            .create_file_download(&address, &bucket_name_enc, &file_name_enc)
            .await?;

        let mut archival_chunks = Vec::new();
        for (i, chunk) in file_download.chunks.into_iter().enumerate() {
            let chunk_cid = chunk.cid.clone();
            let encoded_size = chunk.encoded_size;
            let chunk_size = chunk.size;

            let chunk_download = self
                .create_chunk_download(&bucket_name_enc, &file_name_enc, &address, chunk, i as i64)
                .await?;

            let mut archival_blocks = Vec::new();
            let semaphore = Arc::new(Semaphore::new(self.max_concurrent_blocks));
            let mut handles = Vec::new();

            for block in chunk_download.blocks.iter() {
                let _permit = semaphore.clone().acquire_owned().await.unwrap();
                let node_address = block.node_address.clone();
                let block_cid = block.cid.clone();
                let block_size = block.data.len() as i64;

                let self_clone = Arc::new(self.clone());
                let handle: tokio::task::JoinHandle<(String, i64, Option<PDPBlockData>)> =
                    tokio::spawn(async move {
                        let _p = _permit;
                        match self_clone.resolve_block(&node_address, &block_cid).await {
                            Ok(pdp) => (block_cid, block_size, Some(pdp)),
                            Err(AkaveError::NotFound(_)) => (block_cid, block_size, None),
                            Err(_) => (block_cid, block_size, None),
                        }
                    });
                handles.push(handle);
            }

            for handle in handles {
                let (cid, size, pdp_data) = handle.await.map_err(AkaveError::ThreadJoinError)?;
                archival_blocks.push(ArchivalBlock {
                    cid,
                    size,
                    pdp_data,
                });
            }

            archival_chunks.push(ArchivalChunk {
                cid: chunk_cid,
                encoded_size,
                size: chunk_size,
                index: i as i64,
                blocks: archival_blocks,
            });
        }

        Ok(ArchivalMetadata {
            bucket_name: bucket_name_enc,
            name: file_name_enc,
            chunks: archival_chunks,
        })
    }

    /// Returns the ETH balance (in wei) for the address associated with the SDK's private key.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn get_balance(&self) -> Result<U256, AkaveError> {
        let address = self.storage.client.get_address().await?;
        self.storage
            .client
            .web3_provider
            .eth()
            .balance(address, None)
            .await
            .map_err(AkaveError::BlockchainError)
    }

    pub fn extract_block_data(cid_str: &str, data: Vec<u8>) -> Result<Vec<u8>, AkaveError> {
        let cid = cid_str
            .parse::<cid::Cid>()
            .map_err(|e| AkaveError::InvalidInput(e.to_string()))?;
        Self::use_download_codec(cid.codec(), data)
    }

    fn use_download_codec(codec: u64, chunk_data: Vec<u8>) -> Result<Vec<u8>, AkaveError> {
        Ok(match codec {
            0x55 => chunk_data,
            DAG_PROTOBUF => {
                let mut reader = BytesReader::from_bytes(&chunk_data);
                let mut msg = PbData::default();
                while !reader.is_eof() {
                    match reader.next_tag(&chunk_data) {
                        Ok(18) => {
                            msg.data = Some(
                                reader
                                    .read_bytes(&chunk_data)
                                    .map_err(|e| AkaveError::InvalidInput(e.to_string()))
                                    .map(Cow::Borrowed)?,
                            )
                        }
                        Ok(_) => {}
                        Err(_) => Err(AkaveError::InvalidInput(
                            "error decoding message".to_string(),
                        ))?,
                    }
                }
                msg.data
                    .ok_or_else(|| AkaveError::InvalidInput("Message data not found".to_string()))?
                    .into_owned()
                    .to_vec()
            }
            _default => Err(AkaveError::InvalidInput(
                "Unknown codec for decoding message".to_string(),
            ))?,
        })
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use std::{
        fs::{self, File},
        path::Path,
        sync::Arc,
    };

    use ctor::ctor;
    use env_logger::Builder;
    use log::LevelFilter;
    use pretty_assertions::{assert_eq, assert_ne};
    use uuid::Uuid;

    use crate::{
        sdk::{AkaveSDK, AkaveSDKBuilder},
        types::sdk_types::AkaveError,
    };

    const FILE_NAME_TO_TEST: &str = "1MB.txt";
    const DOWNLOAD_DESTINATION: &str = "/tmp/akave-tests/";
    const TEST_PASSWORD: &str = "testkey123";
    const TEST_KEY: &str = include_str!("blockchain/user.akvf.key");
    const TEST_AKAVE_ADDRESS: &str = "http://127.0.0.1:5000";

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
    async fn get_sdk() -> Result<AkaveSDK, AkaveError> {
        AkaveSDKBuilder::new(TEST_AKAVE_ADDRESS).build().await
    }

    // Get SDK with erasure coding only
    async fn get_sdk_with_erasure() -> Result<AkaveSDK, AkaveError> {
        AkaveSDKBuilder::new(TEST_AKAVE_ADDRESS)
            .with_erasure_coding(3, 2)
            .build()
            .await
    }

    // Get SDK with default encryption only
    async fn get_sdk_with_encryption(encrypt_metadata: bool) -> Result<AkaveSDK, AkaveError> {
        AkaveSDKBuilder::new(TEST_AKAVE_ADDRESS)
            .with_default_encryption(TEST_PASSWORD, encrypt_metadata)
            .build()
            .await
    }

    // Get SDK with both erasure coding and encryption
    async fn get_sdk_with_erasure_and_encryption(
        encrypt_metadata: bool,
    ) -> Result<AkaveSDK, AkaveError> {
        AkaveSDKBuilder::new(TEST_AKAVE_ADDRESS)
            .with_erasure_coding(3, 2)
            .with_default_encryption(TEST_PASSWORD, encrypt_metadata)
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

    const SECRET_KEY: &str = "N1PCdw3M2B1TfJhoaY2mL736p2vCUc47";

    fn encrypt_metadata_hex(value: &str, derivation: &str) -> String {
        use crate::utils::encryption::Encryption;
        let enc = Encryption::new(SECRET_KEY.as_bytes(), derivation).unwrap();
        let encrypted = enc
            .encrypt_deterministic(value.as_bytes(), derivation)
            .unwrap();
        hex::encode(encrypted)
    }

    fn exists_in_buckets(
        bucket_names: &[String],
        buckets: &[crate::types::BucketListItem],
    ) -> bool {
        bucket_names
            .iter()
            .all(|name| buckets.iter().any(|b| &b.name == name))
    }

    async fn get_sdk_with_enc_metadata() -> Result<AkaveSDK, AkaveError> {
        AkaveSDKBuilder::new(TEST_AKAVE_ADDRESS)
            .with_default_encryption(SECRET_KEY, true)
            .build()
            .await
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

    #[test]
    fn test_calculate_file_id() {
        use crate::types::BucketId;
        let cases = [
            (
                "c10fad62c0224052065576135ed2ae4d85d34432b4fb40796eadd9a991f064b9",
                "file1",
                "eea1eddf9f4be315e978c6d0d25d1b870ec0162ebf0acf173f47b738ff0cb421",
            ),
            (
                "f855c5499b442e6b57ea3ec0c260d1e23a74415451ec5a4796560cc9b7d89be0",
                "file2",
                "f8d6d1f6e7ba4f69f00df4e4b53b3e51eb8610f0774f16ea1f02162e0034487b",
            ),
            (
                "f06eac67910341b595f1ef319ca12713a79f180b96a685430d806701dc42e9aa",
                "file3",
                "3eb92385cd986662e9885c47364fa5b2f154cd6fca8d99f4aed68160064991cb",
            ),
        ];
        for (bucket_hex, name, expected) in &cases {
            let bytes: [u8; 32] = hex::decode(bucket_hex).unwrap().try_into().unwrap();
            let bucket_id = BucketId::from(bytes);
            let result = AkaveSDK::calculate_file_id(&bucket_id, name);
            assert_eq!(hex::encode(result), *expected, "file_name={name}");
        }
    }

    #[tokio::test]
    async fn test_create_sdk_client() {
        use crate::types::sdk_types::AkaveError;

        // invalid erasure coding: zero data blocks
        let Err(AkaveError::ErasureCodeError(e)) = AkaveSDKBuilder::new("")
            .with_erasure_coding(0, 1)
            .build()
            .await
        else {
            panic!("expected ErasureCodeError for zero data blocks");
        };
        assert_eq!(e.to_string(), "data and parity blocks must be > 0");

        // invalid erasure coding: zero parity blocks
        let Err(AkaveError::ErasureCodeError(e)) = AkaveSDKBuilder::new("")
            .with_erasure_coding(1, 0)
            .build()
            .await
        else {
            panic!("expected ErasureCodeError for zero parity blocks");
        };
        assert_eq!(e.to_string(), "data and parity blocks must be > 0");
    }

    #[tokio::test]
    async fn test_create_bucket() {
        let bucket_name = generate_test_bucket_name();
        println!("Testing create bucket: {}", bucket_name);

        // Test
        let sdk = get_sdk().await.unwrap();
        let bucket_resp = sdk.create_bucket(&bucket_name).await.unwrap();
        assert_eq!(bucket_resp.name, bucket_name);

        // Cleanup
        let _ = sdk.delete_bucket(&bucket_name).await;
    }

    #[tokio::test]
    async fn test_list_buckets() {
        let bucket_name = generate_test_bucket_name();
        println!("Testing list buckets");

        // Setup
        let sdk = get_sdk().await.unwrap();
        let _ = sdk.create_bucket(&bucket_name).await.unwrap();

        // Test
        let buckets = sdk.list_buckets(0, 20).await.unwrap();
        let len = buckets.buckets.len();
        println!("Found {} buckets", len);
        assert_ne!(len, 0, "there should be buckets in this account");

        // Cleanup
        let _ = sdk.delete_bucket(&bucket_name).await;
    }

    #[tokio::test]
    async fn test_view_bucket() {
        let bucket_name = generate_test_bucket_name();
        println!("Testing view bucket: {}", bucket_name);

        // Setup
        let sdk = get_sdk().await.unwrap();
        let _ = sdk.create_bucket(&bucket_name).await.unwrap();

        // Test
        let bucket = sdk.view_bucket(&bucket_name).await.unwrap();
        assert_eq!(bucket.name, bucket_name);

        // Cleanup
        let _ = sdk.delete_bucket(&bucket_name).await;
    }

    #[tokio::test]
    async fn test_delete_bucket() {
        let bucket_name = generate_test_bucket_name();
        println!("Testing delete bucket: {}", bucket_name);

        // Setup
        let sdk = get_sdk().await.unwrap();
        let _ = sdk.create_bucket(&bucket_name).await.unwrap();

        // Test delete
        let result = sdk.delete_bucket(&bucket_name).await;
        assert!(
            result.is_ok(),
            "Failed to delete bucket: {:?}",
            result.err()
        );

        // Verify deletion - this might need adjustment based on expected behavior
        // If view_bucket is expected to return an error for non-existent buckets:
        let view_result = sdk.view_bucket(&bucket_name).await;
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
        let sdk = get_sdk().await.unwrap();
        let _ = sdk.create_bucket(&bucket_name).await.unwrap();

        // Test upload
        let mut file = File::open(format!("test_files/{}", FILE_NAME_TO_TEST)).unwrap();
        let upload_result = sdk
            .upload_file(&bucket_name, FILE_NAME_TO_TEST, &mut file, None)
            .await;
        assert!(
            upload_result.is_ok(),
            "Failed to upload file: {:?}",
            upload_result.err()
        );

        // Test list files
        let file_list = sdk.list_files(&bucket_name, 0, 20).await.unwrap();
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
        // Note: delete_file may fail with InterfaceUnsupported, so we skip validation
        for file in file_list.files {
            let _ = sdk.delete_file(&bucket_name, &file.name).await;
        }
        // Skip checking if files were deleted since delete_file may not be supported

        // Cleanup
        let _ = sdk.delete_bucket(&bucket_name).await;
    }

    #[tokio::test]
    async fn test_download_file() {
        let bucket_name = generate_test_bucket_name();
        println!("Testing download file from bucket: {}", bucket_name);

        // Setup
        ensure_download_dir();
        let download_path = format!("{}{}", DOWNLOAD_DESTINATION, FILE_NAME_TO_TEST);
        let sdk = Arc::new(get_sdk().await.unwrap());
        let _ = sdk.create_bucket(&bucket_name).await.unwrap();

        let mut upload_file = File::open(format!("test_files/{}", FILE_NAME_TO_TEST)).unwrap();
        let _ = sdk
            .upload_file(&bucket_name, FILE_NAME_TO_TEST, &mut upload_file, None)
            .await
            .unwrap();

        // Clean up any previously downloaded file
        cleanup_download(&download_path);

        let file = File::create(&download_path).unwrap();

        // Test download
        let download_result = Arc::clone(&sdk)
            .download_file(&bucket_name, FILE_NAME_TO_TEST, None, file)
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
        let _ = sdk.delete_bucket(&bucket_name).await;
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
        let sdk = Arc::new(get_sdk_with_erasure().await.unwrap());
        let _ = sdk.create_bucket(&bucket_name).await.unwrap();

        let mut upload_file = File::open(format!("test_files/{}", FILE_NAME_TO_TEST)).unwrap();
        let _ = sdk
            .upload_file(&bucket_name, FILE_NAME_TO_TEST, &mut upload_file, None)
            .await
            .unwrap();

        // Clean up any previously downloaded file
        cleanup_download(&download_path);

        let download_file = File::create(&download_path).unwrap();

        // Test download
        let download_result = Arc::clone(&sdk)
            .download_file(&bucket_name, FILE_NAME_TO_TEST, None, download_file)
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
        let _ = sdk.delete_bucket(&bucket_name).await;
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
        let sdk = Arc::new(get_sdk_with_encryption(false).await.unwrap());
        let _ = sdk.create_bucket(&bucket_name).await.unwrap();

        let mut upload_file = File::open(format!("test_files/{}", FILE_NAME_TO_TEST)).unwrap();
        let _ = sdk
            .upload_file(&bucket_name, FILE_NAME_TO_TEST, &mut upload_file, None)
            .await
            .unwrap();

        // Clean up any previously downloaded file
        cleanup_download(&download_path);

        let download_file = File::create(&download_path).unwrap();

        // Test download
        let download_result = Arc::clone(&sdk)
            .download_file(&bucket_name, FILE_NAME_TO_TEST, None, download_file)
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
        let _ = sdk.delete_bucket(&bucket_name).await;
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
        let sdk = Arc::new(get_sdk_with_erasure_and_encryption(false).await.unwrap());

        // Create bucket
        println!("Creating bucket: {}", bucket_name);
        let bucket_resp = sdk.create_bucket(&bucket_name).await.unwrap();
        assert_eq!(bucket_resp.name, bucket_name);

        // Upload file (using default encryption from SDK)
        println!("Uploading file: {}", FILE_NAME_TO_TEST);
        let mut upload_file = File::open(format!("test_files/{}", FILE_NAME_TO_TEST)).unwrap();
        let _ = sdk
            .upload_file(&bucket_name, FILE_NAME_TO_TEST, &mut upload_file, None)
            .await
            .unwrap();

        // List files
        println!("listing files in bucket {}", bucket_name);
        let file_list = sdk.list_files(&bucket_name, 0, 20).await.unwrap();
        let has_test_file = file_list
            .files
            .iter()
            .any(|file| file.name == FILE_NAME_TO_TEST);
        assert!(has_test_file, "Uploaded file not found in bucket");

        // Download file (using default encryption from SDK)
        cleanup_download(&download_path); // Clean up any previously downloaded file

        let download_file = File::create(&download_path).unwrap();

        let download_result = Arc::clone(&sdk)
            .download_file(&bucket_name, FILE_NAME_TO_TEST, None, download_file)
            .await;
        assert!(download_result.is_ok());
        assert!(Path::new(&download_path).exists());
        let file = File::open(&download_path).unwrap();
        let fsize = file.metadata().unwrap().len();
        assert_eq!(fsize, 920840);

        // Cleanup
        cleanup_download(&download_path);

        // Delete file
        println!("deleting file {}", FILE_NAME_TO_TEST);
        let _ = sdk.delete_file(&bucket_name, FILE_NAME_TO_TEST);

        // Delete bucket
        println!("deleting bucket {}", bucket_name);
        let _ = sdk.delete_bucket(&bucket_name).await;
    }

    // helper cleanup function
    async fn cleanup_all_test_resources() {
        println!("Starting cleanup of all test resources...");

        let sdk = get_sdk().await.unwrap();

        // 1. List all buckets
        println!("Listing all buckets to find test buckets...");
        let buckets = match sdk.list_buckets(0, 100).await {
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
            // TODO: fixme when invalid files are deleted
            // let files = sdk.list_files(&bucket.name).await.expect("Failed to list files for specified (address, bucket_name)");

            // for file in files {
            //     println!("Deleting test file: {}, from bucket: {}", file.name, bucket.name);
            //     match sdk.delete_file(bucket.name.as_str(), file.name.as_str()).await {
            //         Ok(_) => println!("Successfully deleted file: {}", file.name),
            //         Err(e) => println!("Error deleting bucket {}: {:?}", bucket.name, e),
            //     }
            // }
            match sdk.delete_bucket(&bucket.name).await {
                Ok(_) => println!("Successfully deleted bucket: {}", bucket.name),
                Err(e) => println!("Error deleting bucket {}: {:?}", bucket.name, e),
            }
        }

        println!("Cleanup complete!");
    }

    #[tokio::test]
    async fn test_download_file_range() {
        let bucket_name = generate_test_bucket_name();
        println!("Testing download file range from bucket: {}", bucket_name);

        // Setup
        ensure_download_dir();
        let sdk = Arc::new(get_sdk().await.unwrap());
        let _ = sdk.create_bucket(&bucket_name).await.unwrap();

        // Upload a test file
        let mut upload_file = File::open(format!("test_files/{}", FILE_NAME_TO_TEST)).unwrap();
        let _ = sdk
            .upload_file(&bucket_name, FILE_NAME_TO_TEST, &mut upload_file, None)
            .await
            .unwrap();

        // Test downloading a specific range (bytes 1000-2000)
        let mut range_buffer = Vec::new();
        let download_result = sdk
            .clone()
            .download_file_range(
                &bucket_name,
                FILE_NAME_TO_TEST,
                1000,
                2000,
                None,
                &mut range_buffer,
            )
            .await;

        assert!(
            download_result.is_ok(),
            "Failed to download file range: {:?}",
            download_result.err()
        );
        assert_eq!(
            range_buffer.len(),
            1000,
            "Downloaded range should be 1000 bytes"
        );

        // Cleanup
        let _ = sdk.delete_bucket(&bucket_name).await;
    }

    #[tokio::test]
    async fn test_list_buckets_pagination() {
        let sdk = get_sdk().await.unwrap();

        // Create 10 test buckets with a unique run prefix
        let run_id: String = Uuid::new_v4()
            .to_string()
            .chars()
            .filter(|c| c.is_alphanumeric())
            .take(8)
            .collect();
        let names: Vec<String> = (0..10)
            .map(|i| format!("TEST_BUCKET_{}_{:02}", run_id, i))
            .collect();
        for name in &names {
            sdk.create_bucket(name).await.unwrap();
        }

        // Page 1 (offset=0, limit=4): expect 4 results
        let page1 = sdk.list_buckets(0, 4).await.unwrap();
        assert_eq!(page1.buckets.len(), 4, "page 1 should return 4 buckets");

        // Page 2 (offset=4, limit=4): expect 4 results
        let page2 = sdk.list_buckets(4, 4).await.unwrap();
        assert_eq!(page2.buckets.len(), 4, "page 2 should return 4 buckets");

        // Pages must not contain duplicate bucket names
        let names1: std::collections::HashSet<_> =
            page1.buckets.iter().map(|b| b.name.as_str()).collect();
        let names2: std::collections::HashSet<_> =
            page2.buckets.iter().map(|b| b.name.as_str()).collect();
        assert!(
            names1.is_disjoint(&names2),
            "pages 1 and 2 must not overlap"
        );

        // Page 3 (offset=8, limit=4): expect at least our remaining 2 buckets
        let page3 = sdk.list_buckets(8, 4).await.unwrap();
        assert!(!page3.buckets.is_empty(), "page 3 should have results");

        // Cleanup
        for name in &names {
            let _ = sdk.delete_bucket(name).await;
        }
    }

    #[tokio::test]
    async fn test_list_files_pagination() {
        let bucket_name = generate_test_bucket_name();
        let sdk = get_sdk().await.unwrap();
        sdk.create_bucket(&bucket_name).await.unwrap();

        // Upload 10 small in-memory files
        for i in 0..10usize {
            let file_name = format!("file_{:02}.txt", i);
            let mut reader = std::io::Cursor::new(vec![42u8; 1024]);
            sdk.upload_file(&bucket_name, &file_name, &mut reader, None)
                .await
                .unwrap();
        }

        // Page 1 (offset=0, limit=4): 4 files
        let page1 = sdk.list_files(&bucket_name, 0, 4).await.unwrap();
        assert_eq!(page1.files.len(), 4, "page 1 should return 4 files");

        // Page 2 (offset=4, limit=4): 4 files
        let page2 = sdk.list_files(&bucket_name, 4, 4).await.unwrap();
        assert_eq!(page2.files.len(), 4, "page 2 should return 4 files");

        // Page 3 (offset=8, limit=4): remaining 2 files
        let page3 = sdk.list_files(&bucket_name, 8, 4).await.unwrap();
        assert_eq!(page3.files.len(), 2, "page 3 should return 2 files");

        // All file names are unique across pages (no duplicates from offset bug)
        let all_files: std::collections::HashSet<_> = page1
            .files
            .iter()
            .chain(page2.files.iter())
            .chain(page3.files.iter())
            .map(|f| f.name.as_str())
            .collect();
        assert_eq!(
            all_files.len(),
            10,
            "all 10 files should appear across 3 pages"
        );

        // Cleanup
        let _ = sdk.delete_bucket(&bucket_name).await;
    }

    #[tokio::test]
    async fn test_ipc_view_bucket_with_encryption() {
        let sdk = get_sdk_with_enc_metadata().await.unwrap();
        let bucket_name = generate_test_bucket_name();
        let create_result = sdk.create_bucket(&bucket_name).await.unwrap();
        assert_eq!(
            create_result.name, bucket_name,
            "create_bucket returns plaintext name"
        );

        let encrypted_name = encrypt_metadata_hex(&bucket_name, "bucket");
        let view = sdk.view_bucket(&bucket_name).await.unwrap();
        assert_eq!(view.name, bucket_name, "view_bucket returns plaintext name");
        assert_eq!(create_result.id.to_string(), view.id, "ids match");

        // Verify encrypted name is different from plaintext
        assert_ne!(
            encrypted_name, bucket_name,
            "encrypted name differs from plaintext"
        );

        let _ = sdk.delete_bucket(&bucket_name).await;
    }

    #[tokio::test]
    async fn test_ipc_list_bucket_with_encryption() {
        let sdk = get_sdk_with_enc_metadata().await.unwrap();
        let bucket_names: Vec<String> = (0..10).map(|_| generate_test_bucket_name()).collect();

        for name in &bucket_names {
            sdk.create_bucket(name).await.unwrap();
        }

        let all = sdk.list_buckets(0, 200).await.unwrap();
        assert!(all.buckets.len() >= 10, "at least 10 buckets");
        assert!(
            exists_in_buckets(&bucket_names, &all.buckets),
            "all created buckets visible with plaintext names"
        );

        // Paginated: 4, 4, 2
        let mut collected = Vec::new();
        for offset in [0usize, 4, 8] {
            let page = sdk.list_buckets(offset as i64, 4).await.unwrap();
            collected.extend(page.buckets);
        }
        assert!(
            collected.len() >= 10,
            "paginated results include all buckets"
        );

        for name in &bucket_names {
            let _ = sdk.delete_bucket(name).await;
        }
    }

    #[tokio::test]
    async fn test_ipc_list_buckets_with_different_enc_keys() {
        let sdk_no_enc = get_sdk().await.unwrap();
        let sdk_enc1 = get_sdk_with_enc_metadata().await.unwrap();
        let sdk_enc2 = AkaveSDKBuilder::new(TEST_AKAVE_ADDRESS)
            .with_default_encryption("other_key_xyz_789", true)
            .build()
            .await
            .unwrap();

        let names_no_enc: Vec<String> = (0..5).map(|_| generate_test_bucket_name()).collect();
        let names_enc1: Vec<String> = (0..5).map(|_| generate_test_bucket_name()).collect();
        let names_enc2: Vec<String> = (0..5).map(|_| generate_test_bucket_name()).collect();

        for n in &names_no_enc {
            sdk_no_enc
                .create_bucket(n)
                .await
                .unwrap_or_else(|e| panic!("create no-enc bucket {n}: {e}"));
        }
        for n in &names_enc1 {
            sdk_enc1
                .create_bucket(n)
                .await
                .unwrap_or_else(|e| panic!("create enc1 bucket {n}: {e}"));
        }
        for n in &names_enc2 {
            sdk_enc2
                .create_bucket(n)
                .await
                .unwrap_or_else(|e| panic!("create enc2 bucket {n}: {e}"));
        }

        let list = sdk_enc1.list_buckets(0, 200).await.unwrap();
        assert!(list.buckets.len() >= 15);
        assert!(
            exists_in_buckets(&names_no_enc, &list.buckets),
            "no-enc buckets visible"
        );
        assert!(
            exists_in_buckets(&names_enc1, &list.buckets),
            "enc1 buckets decrypted correctly" // In case of revert, need to clear buckets with test_cleanup_manual or restart the node
        );
        assert!(
            !exists_in_buckets(&names_enc2, &list.buckets),
            "enc2 names not decryptable with enc1 key"
        );

        for n in &names_no_enc {
            let _ = sdk_no_enc.delete_bucket(n).await;
        }
        for n in &names_enc1 {
            let _ = sdk_enc1.delete_bucket(n).await;
        }
        for n in &names_enc2 {
            let _ = sdk_enc2.delete_bucket(n).await;
        }
    }

    #[tokio::test]
    async fn test_ipc_file_info_with_encryption() {
        let sdk = get_sdk_with_enc_metadata().await.unwrap();
        let bucket_name = generate_test_bucket_name();
        sdk.create_bucket(&bucket_name).await.unwrap();

        let file_data = vec![0u8; 1024 * 1024]; // 1 MB
        let file_name = "test_enc_fileinfo.bin";
        sdk.upload_file(
            &bucket_name,
            file_name,
            &mut std::io::Cursor::new(file_data),
            None,
        )
        .await
        .unwrap();

        let info = sdk.view_file_info(&bucket_name, file_name).await.unwrap();
        assert_eq!(info.name, file_name);
        assert_eq!(info.bucket_name, bucket_name);

        let _ = sdk.delete_bucket(&bucket_name).await;
    }

    #[tokio::test]
    async fn test_ipc_list_files_with_encryption() {
        let sdk = get_sdk_with_enc_metadata().await.unwrap();
        let bucket_name = generate_test_bucket_name();
        sdk.create_bucket(&bucket_name).await.unwrap();

        let file_names: Vec<String> = (0..10).map(|i| format!("enc_file_{:02}.bin", i)).collect();
        for name in &file_names {
            let data = vec![0u8; 1024 * 1024];
            sdk.upload_file(&bucket_name, name, &mut std::io::Cursor::new(data), None)
                .await
                .unwrap();
        }

        let all = sdk.list_files(&bucket_name, 0, 20).await.unwrap();
        assert_eq!(all.files.len(), 10);
        for file in &all.files {
            assert!(
                file_names.contains(&file.name),
                "file '{}' should be plaintext",
                file.name
            );
        }

        let _ = sdk.delete_bucket(&bucket_name).await;
    }

    #[tokio::test]
    async fn test_ipc_upload_with_chunks_batch_size() {
        for (file_size_mb, batch_size) in [(128usize, 2usize), (256, 3)] {
            let sdk = AkaveSDKBuilder::new(TEST_AKAVE_ADDRESS)
                .with_chunk_batch_size(batch_size)
                .build()
                .await
                .unwrap();
            let bucket_name = generate_test_bucket_name();
            sdk.create_bucket(&bucket_name).await.unwrap();

            let data = vec![0u8; file_size_mb * 1024 * 1024];
            let file_name = format!("batch_{}_mb.bin", file_size_mb);
            sdk.upload_file(
                &bucket_name,
                &file_name,
                &mut std::io::Cursor::new(data),
                None,
            )
            .await
            .unwrap();

            // Verify file exists after upload
            let info = sdk.view_file_info(&bucket_name, &file_name).await.unwrap();
            assert_eq!(info.name, file_name);

            let _ = sdk.delete_bucket(&bucket_name).await;
        }
    }

    #[tokio::test]
    async fn test_ipc_metadata_encryption() {
        let sdk = Arc::new(get_sdk_with_enc_metadata().await.unwrap());
        let bucket_name = generate_test_bucket_name();
        let file_name = "meta_enc_test.bin";
        let file_data = vec![42u8; 1024 * 1024];

        // CreateBucket: returned name is plaintext
        let create_resp = sdk.create_bucket(&bucket_name).await.unwrap();
        assert_eq!(create_resp.name, bucket_name);

        // ViewBucket: plaintext
        let view_resp = sdk.view_bucket(&bucket_name).await.unwrap();
        assert_eq!(view_resp.name, bucket_name);

        // ListBuckets: bucket visible with plaintext name
        let list_resp = sdk.list_buckets(0, 200).await.unwrap();
        assert!(
            list_resp.buckets.iter().any(|b| b.name == bucket_name),
            "bucket should appear in list with plaintext name"
        );

        // Upload file
        sdk.upload_file(
            &bucket_name,
            file_name,
            &mut std::io::Cursor::new(file_data.clone()),
            None,
        )
        .await
        .unwrap();

        // FileInfo: plaintext names returned
        let info = sdk.view_file_info(&bucket_name, file_name).await.unwrap();
        assert_eq!(info.name, file_name);
        assert_eq!(info.bucket_name, bucket_name);

        // ListFiles: plaintext name returned
        let files = sdk.list_files(&bucket_name, 0, 200).await.unwrap();
        assert!(files.files.iter().any(|f| f.name == file_name));

        // Download: data correct
        let downloaded = Arc::clone(&sdk)
            .download_file(&bucket_name, file_name, None, Vec::<u8>::new())
            .await
            .unwrap();
        assert_eq!(downloaded, file_data);

        // Delete file and bucket
        sdk.delete_file(&bucket_name, file_name).await.unwrap();
        sdk.delete_bucket(&bucket_name).await.unwrap();
    }

    #[tokio::test]
    async fn test_upload_same_files_after_removal() {
        let sdk = Arc::new(get_sdk().await.unwrap());
        let bucket_name = generate_test_bucket_name();
        sdk.create_bucket(&bucket_name).await.unwrap();

        let sizes_mb = [1usize, 16, 32, 8];
        let file_names: Vec<String> = sizes_mb
            .iter()
            .enumerate()
            .map(|(i, _)| format!("reupload_file_{}.bin", i))
            .collect();
        let mut file_contents: Vec<Vec<u8>> = Vec::new();

        for (name, &sz) in file_names.iter().zip(sizes_mb.iter()) {
            let data = vec![(sz % 256) as u8; sz * 1024 * 1024];
            file_contents.push(data.clone());
            sdk.upload_file(&bucket_name, name, &mut std::io::Cursor::new(data), None)
                .await
                .unwrap();
        }

        // Delete all files
        for name in &file_names {
            sdk.delete_file(&bucket_name, name).await.unwrap();
        }
        let empty = sdk.list_files(&bucket_name, 0, 20).await.unwrap();
        assert_eq!(empty.files.len(), 0, "no files after deletion");

        // Delete and recreate bucket
        sdk.delete_bucket(&bucket_name).await.unwrap();
        sdk.create_bucket(&bucket_name).await.unwrap();

        // Re-upload: first half with same names, second half with new names
        let half = file_names.len() / 2;
        let new_names: Vec<String> = (half..file_names.len())
            .map(|i| format!("reupload_new_{}.bin", i))
            .collect();
        let all_new_names: Vec<String> = file_names[..half]
            .iter()
            .cloned()
            .chain(new_names.iter().cloned())
            .collect();

        for (name, data) in all_new_names.iter().zip(file_contents.iter()) {
            sdk.upload_file(
                &bucket_name,
                name,
                &mut std::io::Cursor::new(data.clone()),
                None,
            )
            .await
            .unwrap();
        }

        let list = sdk.list_files(&bucket_name, 0, 20).await.unwrap();
        assert_eq!(list.files.len(), file_names.len(), "same number of files");

        // Download and verify contents
        for (name, original) in all_new_names.iter().zip(file_contents.iter()) {
            let buf = Arc::clone(&sdk)
                .download_file(&bucket_name, name, None, Vec::<u8>::new())
                .await
                .unwrap();
            assert_eq!(&buf, original, "downloaded content matches for {}", name);
        }

        let _ = sdk.delete_bucket(&bucket_name).await;
    }

    // ── TestIPCFileDelete ─────────────────────────────────────────────────────
    #[tokio::test]
    async fn test_delete_file() {
        let sdk = get_sdk().await.unwrap();
        let bucket_name = generate_test_bucket_name();
        sdk.create_bucket(&bucket_name).await.unwrap();

        let mut file = File::open(format!("test_files/{}", FILE_NAME_TO_TEST)).unwrap();
        sdk.upload_file(&bucket_name, FILE_NAME_TO_TEST, &mut file, None)
            .await
            .unwrap();

        // Verify file exists
        let list = sdk.list_files(&bucket_name, 0, 20).await.unwrap();
        assert!(
            list.files.iter().any(|f| f.name == FILE_NAME_TO_TEST),
            "uploaded file must appear in listing"
        );

        // Delete the file
        sdk.delete_file(&bucket_name, FILE_NAME_TO_TEST)
            .await
            .unwrap();

        // Verify it is gone
        let list_after = sdk.list_files(&bucket_name, 0, 20).await.unwrap();
        assert!(
            !list_after.files.iter().any(|f| f.name == FILE_NAME_TO_TEST),
            "file must not appear in listing after deletion"
        );

        let _ = sdk.delete_bucket(&bucket_name).await;
    }

    // ── TestIPCFileInfo ───────────────────────────────────────────────────────
    #[tokio::test]
    async fn test_file_info() {
        let sdk = get_sdk().await.unwrap();
        let bucket_name = generate_test_bucket_name();
        sdk.create_bucket(&bucket_name).await.unwrap();

        let mut file = File::open(format!("test_files/{}", FILE_NAME_TO_TEST)).unwrap();
        sdk.upload_file(&bucket_name, FILE_NAME_TO_TEST, &mut file, None)
            .await
            .unwrap();

        let info = sdk
            .view_file_info(&bucket_name, FILE_NAME_TO_TEST)
            .await
            .unwrap();

        assert_eq!(info.name, FILE_NAME_TO_TEST);
        assert_eq!(info.bucket_name, bucket_name);
        assert!(!info.root_cid.is_empty(), "root_cid must not be empty");
        assert!(info.encoded_size > 0, "encoded_size must be positive");

        let _ = sdk.delete_bucket(&bucket_name).await;
    }

    // ── TestIPCFileSetPublicAccess ────────────────────────────────────────────
    #[tokio::test]
    async fn test_file_set_public_access() {
        let sdk = get_sdk().await.unwrap();
        let bucket_name = generate_test_bucket_name();
        sdk.create_bucket(&bucket_name).await.unwrap();

        let mut file = File::open(format!("test_files/{}", FILE_NAME_TO_TEST)).unwrap();
        sdk.upload_file(&bucket_name, FILE_NAME_TO_TEST, &mut file, None)
            .await
            .unwrap();

        sdk.set_file_public_access(&bucket_name, FILE_NAME_TO_TEST, true)
            .await
            .expect("setting public access to true should succeed");

        sdk.set_file_public_access(&bucket_name, FILE_NAME_TO_TEST, false)
            .await
            .expect("setting public access to false should succeed");

        let _ = sdk.delete_bucket(&bucket_name).await;
    }

    // ── TestIPCFileUploadStateConcurrency ─────────────────────────────────────
    #[tokio::test]
    async fn test_file_upload_state_concurrency() {
        let sdk = Arc::new(get_sdk().await.unwrap());
        let bucket_name = generate_test_bucket_name();
        sdk.create_bucket(&bucket_name).await.unwrap();

        // Spawn 3 concurrent uploads of different files to the same bucket
        let mut handles = Vec::new();
        for i in 0..3usize {
            let sdk_clone = Arc::clone(&sdk);
            let bucket = bucket_name.clone();
            let name = format!("concurrent_{}.bin", i);
            handles.push(tokio::spawn(async move {
                let data = vec![(i as u8 + 1) * 17; 256 * 1024]; // 256 KiB each
                sdk_clone
                    .upload_file(&bucket, &name, &mut std::io::Cursor::new(data), None)
                    .await
            }));
        }

        let results: Vec<_> = futures::future::join_all(handles).await;
        let succeeded = results.iter().filter(|r| matches!(r, Ok(Ok(_)))).count();
        assert!(
            succeeded > 0,
            "at least one concurrent upload must succeed; results: {:?}",
            results
                .iter()
                .map(|r| r.as_ref().map(|inner| inner.is_ok()))
                .collect::<Vec<_>>()
        );

        let _ = sdk.delete_bucket(&bucket_name).await;
    }

    #[tokio::test]
    #[ignore]
    async fn test_cleanup_manual() {
        // This test is ignored by default and must be run manually with:
        // cargo test --package akave-rs --lib -- tests::test_cleanup_manual --ignored --nocapture
        cleanup_all_test_resources().await;
    }
}
