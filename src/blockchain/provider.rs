// Standard library imports
use std::time::Duration;

// External crate imports (general)
use web3::{
    contract::{tokens::Tokenize, Contract, Options},
    error::TransportError,
    types::{TransactionReceipt, H160, H256, U256},
    Error, Web3,
};

// Internal imports
use super::ipc_types::{BucketResponse, FileResponse};
use crate::{log_debug, log_error, log_info};

// Target-specific imports
#[cfg(target_arch = "wasm32")]
mod wasm_imports {
    pub use web3::transports::eip_1193::{Eip1193, Provider};
}

#[cfg(target_arch = "wasm32")]
use wasm_imports::*;

#[cfg(not(target_arch = "wasm32"))]
mod native_imports {
    pub use std::str::FromStr;
    pub use web3::signing::{Key, SecretKey, SecretKeyRef};
    pub use web3::transports::http::Http;
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
const COMMIT_FILE: &str = "commitFile";
const CREATE_FILE: &str = "createFile";
const DELETE_FILE: &str = "deleteFile";
const GET_FILE_INDEX_BY_NAME: &str = "getFileIndexById";
const GET_FILE_BY_NAME: &str = "getFileByName";

pub(crate) struct BlockchainProvider {
    pub web3_provider: Web3<ProviderType>,
    pub akave: Contract<ProviderType>,
    confirmations: usize,
    #[cfg(not(target_arch = "wasm32"))]
    key: Option<SecretKey>,
}

//TODO(samhassan): use sdk specific errors instead of dyn Error.
impl BlockchainProvider {
    pub fn new(
        _rpc_url: &str,
        access_address: &str,
        confirmations: Option<usize>,
    ) -> Result<BlockchainProvider, Error> {
        log_debug!(
            "Initializing BlockchainProvider with access address: {}",
            access_address
        );

        let confirmations_opt = match confirmations {
            Some(value) => value,
            None => 0,
        };

        #[cfg(target_arch = "wasm32")]
        {
            let provider = Provider::default();
            match provider {
                Ok(provider_option) => match provider_option {
                    Some(provider) => {
                        log_debug!("Creating WASM EIP-1193 transport");
                        let transport = web3::transports::eip_1193::Eip1193::new(provider);
                        let web3_provider = web3::Web3::new(transport);
                        log_debug!("Creating contract instance");
                        let akave = Contract::from_json(
                            web3_provider.eth(),
                            access_address.parse::<H160>().unwrap(),
                            include_bytes!("contract.json"),
                        )
                        .unwrap();
                        log_info!("BlockchainProvider initialized successfully for WASM");
                        Ok(Self {
                            web3_provider,
                            akave,
                            confirmations: confirmations_opt,
                            #[cfg(not(target_arch = "wasm32"))]
                            key: None,
                        })
                    }
                    None => {
                        log_error!(
                            "Failed to build EIP-1193 web3 transport: No provider available"
                        );
                        Err(Error::Transport(TransportError::Message(format!(
                            "failed to build eip_1193 web3 transport",
                        ))))
                    }
                },
                Err(e) => {
                    log_error!(
                        "Failed to get EIP-1193 wallet provider: {}",
                        e.as_string().get_or_insert_default()
                    );
                    Err(Error::Transport(TransportError::Message(format!(
                        "failed to get eip_1193 wallet provider"
                    ))))
                }
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            log_debug!("Creating native HTTP transport");
            let pvkey: &str = include_str!("user.akvf.key");
            let transport = ProviderType::new(_rpc_url);

            match transport {
                Ok(transport_option) => {
                    let web3_provider = Web3::new(transport_option);
                    log_debug!("Creating contract instance");
                    let akave = Contract::from_json(
                        web3_provider.eth(),
                        access_address.parse::<H160>().unwrap(),
                        include_bytes!("contract.json"),
                    )
                    .unwrap();

                    let key = SecretKey::from_str(pvkey).unwrap();

                    log_info!("BlockchainProvider initialized successfully for native");
                    Ok(Self {
                        web3_provider,
                        akave,
                        key: Some(key),
                        confirmations: confirmations_opt,
                    })
                }
                Err(e) => {
                    log_error!("Failed to get HTTP web3 transport: {}", e);
                    Err(Error::Transport(TransportError::Message(format!(
                        "failed to get http web3 transport"
                    ))))
                }
            }
        }
    }

    async fn call_contract_with_confirmations(
        &self,
        function_name: &str,
        params: impl Tokenize,
    ) -> Result<TransactionReceipt, Box<dyn std::error::Error>> {
        log_debug!(
            "Calling contract function: {} with confirmations",
            function_name
        );
        let eth = self.web3_provider.eth();

        // Send transaction and get hash
        let hash = self.call_contract(function_name, params).await?;
        log_debug!("Transaction hash: {}", hash);

        // Initial backoff parameters
        let mut backoff_ms = 1000; // Start with 1 second
        let max_backoff_ms = 10000; // Max 10 seconds
        let max_attempts = 60; // 1 minute total with max backoff
        let mut attempts = 0;

        loop {
            // Check if we've exceeded max attempts
            if attempts >= max_attempts {
                log_error!("Transaction confirmation timeout after 1 minute");
                return Err("Transaction confirmation timeout".into());
            }

            // Get current block number
            let current_block = eth.block_number().await?;
            log_debug!("Current block number: {}", current_block);

            // Get transaction receipt
            let receipt = eth.transaction_receipt(hash).await?;

            match receipt {
                Some(receipt) => {
                    if let Some(confirmation_block) = receipt.block_number {
                        if current_block.low_u64() < confirmation_block.low_u64() {
                            log_debug!("Current block number is less than confirmation block number, waiting for confirmation");
                            continue;
                        }
                        let blocks_since_confirmation =
                            current_block.low_u64() - confirmation_block.low_u64();
                        log_debug!("Blocks since confirmation: {}", blocks_since_confirmation);

                        if blocks_since_confirmation >= self.confirmations as u64 {
                            log_info!(
                                "Transaction confirmed with {} blocks",
                                blocks_since_confirmation
                            );
                            return Ok(receipt);
                        }
                    }
                }
                None => {
                    log_debug!("Transaction receipt not found yet");
                }
            }

            // Simple jitter using block number
            let jitter = (current_block.low_u64() % 1000) as u64;
            let sleep_duration = std::time::Duration::from_millis(backoff_ms + jitter);

            #[cfg(target_arch = "wasm32")]
            {
                use gloo_timers::future::sleep;
                sleep(sleep_duration).await;
            }

            #[cfg(not(target_arch = "wasm32"))]
            {
                tokio::time::sleep(sleep_duration).await;
            }

            // Increase backoff time, but cap it
            backoff_ms = (backoff_ms * 2).min(max_backoff_ms);
            attempts += 1;
        }
    }

    async fn call_contract(
        &self,
        function_name: &str,
        params: impl Tokenize,
    ) -> Result<H256, Box<dyn std::error::Error>> {
        log_debug!("Calling contract function: {}", function_name);
        let txopts = Options {
            gas: Some(U256::from(500000)),
            ..Default::default()
        };

        #[cfg(not(target_arch = "wasm32"))]
        {
            let key = self.key.as_ref().ok_or("Missing key for signed call")?;
            let key_ref = SecretKeyRef::new(key);

            let hash = self
                .akave
                .signed_call(function_name, params, txopts, key_ref)
                .await?;

            return Ok(hash);
        }

        #[cfg(target_arch = "wasm32")]
        {
            log_debug!(
                "Calling contract function: {} with confsssirmations",
                function_name
            );
            let address = self.web3_provider.eth().accounts().await?[0];
            log_debug!(
                "Calling contract function: {} with confirmations, with address: {}",
                function_name,
                address
            );
            return Ok(self
                .akave
                .call(function_name, params, address, txopts)
                .await?);
        }
    }

    pub async fn get_address(&self) -> Result<H160, Box<dyn std::error::Error>> {
        log_debug!("Getting provider address");
        #[cfg(target_arch = "wasm32")]
        {
            Ok(self.web3_provider.eth().accounts().await?[0])
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            match self.key {
                Some(key) => Ok(SecretKeyRef::new(&key).address()),
                None => Ok(self.web3_provider.eth().accounts().await?[0]),
            }
        }
    }

    pub async fn create_file(
        &self,
        bucket_id: Vec<u8>,
        file_name: String,
    ) -> Result<TransactionReceipt, Box<dyn std::error::Error>> {
        let file_name_clone = file_name.clone();
        log_debug!(
            "Creating file: {} in bucket: {:?}",
            file_name_clone,
            bucket_id
        );
        let id: [u8; 32] = bucket_id.try_into().expect("bucket_id error");
        let result = self
            .call_contract_with_confirmations(CREATE_FILE, (id, file_name))
            .await;
        match &result {
            Ok(_) => log_info!("File created successfully: {}", file_name_clone),
            Err(e) => log_error!("Failed to create file: {}", e),
        }
        result
    }

    pub async fn commit_file(
        &self,
        bucket_id: [u8; 32],
        file_name: String,
        size: U256,
        root_cid: Vec<u8>,
    ) -> Result<TransactionReceipt, Box<dyn std::error::Error>> {
        let file_name_clone = file_name.clone();
        log_debug!(
            "Committing file: {} in bucket: {:?}",
            file_name_clone,
            bucket_id
        );
        let result = self
            .call_contract_with_confirmations(COMMIT_FILE, (bucket_id, file_name, size, root_cid))
            .await;
        match &result {
            Ok(_) => log_info!("File committed successfully: {}", file_name_clone),
            Err(e) => log_error!("Failed to commit file: {}", e),
        }
        result
    }

    pub async fn add_file_chunk(
        &self,
        root_cid: Vec<u8>,
        bucket_id: [u8; 32],
        file_name: String,
        size: U256,
        cids: Vec<[u8; 32]>,
        sizes: Vec<U256>,
        index: U256,
    ) -> Result<TransactionReceipt, Box<dyn std::error::Error>> {
        let file_name_clone = file_name.clone();
        log_debug!(
            "Adding file chunk for file: {} in bucket: {:?}",
            file_name_clone,
            bucket_id
        );
        let result = self
            .call_contract_with_confirmations(
                ADD_FILE_CHUNK,
                (root_cid, bucket_id, file_name, size, cids, sizes, index),
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

    pub async fn create_bucket(
        &self,
        bucket_name: String,
    ) -> Result<TransactionReceipt, Box<dyn std::error::Error>> {
        let bucket_name_clone = bucket_name.clone();
        log_debug!("Creating bucket: {}", bucket_name_clone);
        let result = self
            .call_contract_with_confirmations(CREATE_BUCKET, (bucket_name,))
            .await;
        match &result {
            Ok(_) => log_info!("Bucket created successfully: {}", bucket_name_clone),
            Err(e) => log_error!("Failed to create bucket: {}", e),
        }
        result
    }

    pub async fn delete_bucket(
        &self,
        bucket_id: Vec<u8>,
        bucket_name: String,
        bucket_idx: U256,
    ) -> Result<TransactionReceipt, Box<dyn std::error::Error>> {
        let bucket_name_clone = bucket_name.clone();
        log_debug!(
            "Deleting bucket: {} with ID: {:?}",
            bucket_name_clone,
            bucket_id
        );
        let id: [u8; 32] = bucket_id.try_into().expect("bucket_id error");
        let result = self
            .call_contract_with_confirmations(DELETE_BUCKET, (id, bucket_name, bucket_idx))
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
    ) -> Result<BucketResponse, Box<dyn std::error::Error>> {
        let bucket_name_clone = bucket_name.clone();
        log_debug!("Getting bucket by name: {}", bucket_name_clone);
        let address = self.get_address().await?;
        let result: BucketResponse = self
            .akave
            .query(
                GET_BUCKET_BY_NAME,
                (bucket_name,),
                address,
                Options::default(),
                None,
            )
            .await
            .unwrap();
        log_info!("Retrieved bucket details for: {}", bucket_name_clone);
        Ok(result)
    }

    pub async fn get_bucket_index_by_name(
        &self,
        bucket_name: String,
    ) -> Result<U256, Box<dyn std::error::Error>> {
        let bucket_name_clone = bucket_name.clone();
        log_debug!("Getting bucket index for name: {}", bucket_name_clone);
        let address = self.get_address().await?;
        let result: U256 = self
            .akave
            .query(
                GET_BUCKET_INDEX_BY_NAME,
                (bucket_name, address),
                address,
                Options::default(),
                None,
            )
            .await?;
        log_info!("Retrieved bucket index for: {}", bucket_name_clone);
        Ok(result)
    }

    pub async fn delete_file(
        &self,
        file_name: String,
        bucket_id: Vec<u8>,
    ) -> Result<TransactionReceipt, Box<dyn std::error::Error>> {
        let file_name_clone = file_name.clone();
        log_debug!(
            "Deleting file: {} from bucket: {:?}",
            file_name_clone,
            bucket_id
        );
        let parsed_bucket_id: [u8; 32] = bucket_id.clone().try_into().expect("bucket_id error");

        let file = self
            .get_file_by_name(bucket_id, file_name.to_string())
            .await?;
        let file_idx = self
            .get_file_index_by_name(file.name, file.id.to_vec().clone())
            .await?;
        let result = self
            .call_contract_with_confirmations(
                DELETE_FILE,
                (file.id, parsed_bucket_id, file_name, file_idx),
            )
            .await;
        match &result {
            Ok(_) => log_info!("File deleted successfully: {}", file_name_clone),
            Err(e) => log_error!("Failed to delete file: {}", e),
        }
        result
    }

    pub async fn get_file_index_by_name(
        &self,
        file_name: String,
        file_id: Vec<u8>,
    ) -> Result<U256, Box<dyn std::error::Error>> {
        let file_name_clone = file_name.clone();
        log_debug!("Getting file index for name: {}", file_name_clone);
        let address = self.get_address().await?;
        let parsed_id: [u8; 32] = file_id.try_into().expect("file_id error");
        let result: U256 = self
            .akave
            .query(
                GET_FILE_INDEX_BY_NAME,
                (file_name, parsed_id),
                address,
                Options::default(),
                None,
            )
            .await?;
        log_info!("Retrieved file index for: {}", file_name_clone);
        Ok(result)
    }

    pub async fn get_file_by_name(
        &self,
        bucket_id: Vec<u8>,
        file_name: String,
    ) -> Result<FileResponse, Box<dyn std::error::Error>> {
        let file_name_clone = file_name.clone();
        log_debug!(
            "Getting file by name: {} from bucket: {:?}",
            file_name_clone,
            bucket_id
        );
        let address = self.get_address().await?;
        let parsed_id: [u8; 32] = bucket_id.try_into().expect("bucket_id error");
        let result = self
            .akave
            .query(
                GET_FILE_BY_NAME,
                (parsed_id, file_name),
                address,
                Options::default(),
                None,
            )
            .await
            .unwrap();
        log_info!("Retrieved file details for: {}", file_name_clone);
        Ok(result)
    }

    async fn sign_message(&self, str: String) -> Result<String, Error> {
        log_debug!("Signing message");
        let accounts = self.web3_provider.eth().accounts().await?;
        log_debug!("Got accounts: {:?}", accounts);
        let signed = self
            .web3_provider
            .personal()
            .sign(str.into(), accounts[0], "".into())
            .await?;
        log_info!("Message signed successfully");
        Ok(signed.to_string().into())
    }
}
