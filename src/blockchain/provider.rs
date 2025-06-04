// Standard library imports

use std::collections::HashMap;

// External crate imports (general)
use thiserror::Error;
use web3::{
    contract::{tokens::Tokenize, Contract, Options},
    error::TransportError,
    types::{TransactionReceipt, H160, H256, U256},
    Error, Web3,
};

// Internal imports
use super::ipc_types::{BucketResponse, FileResponse};
use crate::{
    blockchain::eip712_types::{Domain, TypedData},
    log_debug, log_error, log_info,
    types::BucketId,
};

// Target-specific imports
#[cfg(target_arch = "wasm32")]
mod wasm_imports {
    pub use web3::{
        transports::eip_1193::{Eip1193, Provider},
        Transport,
    };
}

#[cfg(target_arch = "wasm32")]
use wasm_imports::*;

#[cfg(not(target_arch = "wasm32"))]
mod native_imports {
    pub use std::str::FromStr;

    pub use web3::{
        signing::{Key, SecretKey, SecretKeyRef},
        transports::http::Http,
    };
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

#[derive(Clone)]
pub(crate) struct BlockchainProvider {
    pub web3_provider: Web3<ProviderType>,
    pub akave_storage: Contract<ProviderType>,
    confirmations: usize,
    // Native environment private key for signing
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
                        let storage_address = access_address.parse::<H160>().map_err(|e| {
                            Error::Decoder(format!("Invalid contract address: {}", e))
                        })?;
                        let akave_storage = Contract::from_json(
                            web3_provider.eth(),
                            storage_address,
                            include_bytes!("storage.json"),
                        )
                        .map_err(|e| {
                            Error::Decoder(format!("Failed to create contract instance: {}", e))
                        })?;
                        log_info!("Akave contract address: 0x{:x}", akave_storage.address());
                        log_info!("BlockchainProvider initialized successfully for WASM");
                        Ok(Self {
                            web3_provider,
                            akave_storage,
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
            let pvkey = std::env::var("AKAVE_PRIVATE_KEY")
                .expect("AKAVE_PRIVATE_KEY environment variable not set");
            let transport = ProviderType::new(_rpc_url);

            match transport {
                Ok(transport_option) => {
                    let web3_provider = Web3::new(transport_option);
                    log_debug!("Creating contract instance");
                    let storage_address = access_address
                        .parse::<H160>()
                        .map_err(|e| Error::Decoder(format!("Invalid contract address: {}", e)))?;
                    let akave_storage = Contract::from_json(
                        web3_provider.eth(),
                        storage_address,
                        include_bytes!("storage.json"),
                    )
                    .map_err(|e| {
                        Error::Decoder(format!("Failed to create contract instance: {}", e))
                    })?;

                    log_info!("Akave contract address: 0x{:x}", akave_storage.address());

                    let key = SecretKey::from_str(pvkey.trim())
                        .map_err(|e| {
                            log_error!("Failed to parse private key: {}. Make sure the key is a valid 64-character hex string (with or without 0x prefix)", e);
                            Error::Decoder(format!(
                                "Invalid private key format: {}. Expected 64-character hex string",
                                e
                            ))
                        })?;

                    log_info!("BlockchainProvider initialized successfully for native");
                    Ok(Self {
                        web3_provider,
                        akave_storage,
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
    ) -> Result<TransactionReceipt, ProviderError> {
        log_debug!(
            "Calling contract function: {} with confirmations",
            function_name
        );
        let eth = self.web3_provider.eth();

        // Send transaction and get hash
        let hash = self
            .call_contract(function_name, params)
            .await
            .map_err(|e| ProviderError::ContractCallError(e.to_string()))?;
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
                return Err(ProviderError::TransactionConfirmTimeout(
                    "Transaction confirmation timeout".into(),
                ));
            }

            // Get current block number
            let current_block = eth
                .block_number()
                .await
                .map_err(|e| ProviderError::BlockNumberError(e.to_string()))?;
            log_debug!("Current block number: {}", current_block);

            // Get transaction receipt
            let receipt = eth
                .transaction_receipt(hash)
                .await
                .map_err(|e| ProviderError::TransactionError(e.to_string()))?;

            match receipt {
                Some(receipt) => {
                    match receipt.status {
                        Some(status) => {
                            if status.low_u64() == 0 {
                                log_error!("Transaction failed with status 0");
                                return Err(ProviderError::TransactionError(
                                    format!(
                                        "Transaction {}-{} failed with status 0",
                                        receipt.transaction_hash, function_name
                                    )
                                    .into(),
                                ));
                            }
                        }
                        None => {
                            log_error!("Transaction failed with unknown status");
                            return Err(ProviderError::TransactionError(
                                "Transaction failed with unknown status".into(),
                            ));
                        }
                    }
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
                                "Transaction {}-{} confirmed with {} blocks",
                                receipt.transaction_hash,
                                function_name,
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
    ) -> Result<H256, ProviderError> {
        let txopts = Options {
            gas: Some(U256::from(500000)),
            ..Default::default()
        };

        #[cfg(not(target_arch = "wasm32"))]
        {
            let key = self
                .key
                .as_ref()
                .ok_or("Missing key for signed call")
                .map_err(|e| ProviderError::KeyError(e.to_string()))?;
            let key_ref = SecretKeyRef::new(key);

            let hash = self
                .akave_storage
                .signed_call(function_name, params, txopts, key_ref)
                .await
                .map_err(|e| ProviderError::ContractCallError(e.to_string()))?;

            return Ok(hash);
        }

        #[cfg(target_arch = "wasm32")]
        {
            let address = self
                .web3_provider
                .eth()
                .accounts()
                .await
                .map_err(|e| ProviderError::AddressError(e.to_string()))?[0];
            log_debug!(
                "Calling contract function: {} with confirmations, with address: {}",
                function_name,
                address
            );
            return Ok(self
                .akave_storage
                .call(function_name, params, address, txopts)
                .await
                .map_err(|e| ProviderError::ContractCallError(e.to_string()))?);
        }
    }

    pub async fn get_address(&self) -> Result<H160, ProviderError> {
        log_debug!("Getting provider address");
        #[cfg(target_arch = "wasm32")]
        {
            let accounts = self
                .web3_provider
                .eth()
                .accounts()
                .await
                .map_err(|e| ProviderError::AccountError(e.to_string()))?;

            if accounts.is_empty() {
                log_error!("No accounts available. Please connect your wallet.");
                return Err(ProviderError::AccountError(
                    "No accounts available. Please connect your wallet.".into(),
                ));
            }

            Ok(accounts[0])
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            match self.key {
                Some(key) => Ok(SecretKeyRef::new(&key).address()),
                None => {
                    let accounts = self
                        .web3_provider
                        .eth()
                        .accounts()
                        .await
                        .map_err(|e| ProviderError::AddressError(e.to_string()))?;

                    if accounts.is_empty() {
                        return Err(ProviderError::AccountError("No accounts available".into()));
                    }

                    Ok(accounts[0])
                }
            }
        }
    }

    pub async fn get_hex_address(&self) -> Result<String, ProviderError> {
        log_debug!("Getting provider hex address");
        let address = self.get_address().await?;
        Ok(format!("0x{:x}", address))
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
            .call_contract_with_confirmations(CREATE_FILE, (bucket_id.to_bytes(), file_name))
            .await;
        match &result {
            Ok(_) => log_info!("File created successfully: {}", file_name_clone),
            Err(e) => log_error!("Failed to create file: {}", e),
        }
        result
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
            .call_contract_with_confirmations(
                COMMIT_FILE,
                (
                    bucket_id.to_bytes(),
                    file_name,
                    encode_size,
                    // actual_size,
                    root_cid,
                ),
            )
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
            .call_contract_with_confirmations(
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
    ) -> Result<TransactionReceipt, ProviderError> {
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
            .call_contract_with_confirmations(
                DELETE_BUCKET,
                (bucket_id.to_bytes(), bucket_name, bucket_idx),
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
        let address = self.get_address().await?;
        let result: BucketResponse = self
            .akave_storage
            .query(
                GET_BUCKET_BY_NAME,
                (bucket_name,),
                address,
                Options::default(),
                None,
            )
            .await
            .map_err(|e| ProviderError::ContractCallError(e.to_string()))?;
        Ok(result)
    }

    pub async fn get_bucket_index_by_name(
        &self,
        bucket_name: String,
    ) -> Result<U256, ProviderError> {
        let bucket_name_clone = bucket_name.clone();
        log_debug!("Getting bucket index for name: {}", bucket_name_clone);
        let address = self.get_address().await?;
        let result: U256 = self
            .akave_storage
            .query(
                GET_BUCKET_INDEX_BY_NAME,
                (bucket_name, address),
                address,
                Options::default(),
                None,
            )
            .await
            .map_err(|e| ProviderError::ContractCallError(e.to_string()))?;
        Ok(result)
    }

    pub async fn delete_file(
        &self,
        file_name: String,
        bucket_id: BucketId,
    ) -> Result<TransactionReceipt, ProviderError> {
        let file_name_clone = file_name.clone();
        log_debug!(
            "Deleting file: {} from bucket: {}",
            file_name_clone,
            bucket_id
        );

        let file = self
            .get_file_by_name(bucket_id, file_name.to_string())
            .await?;
        let file_idx = self
            .get_file_index_by_name(file.name, file.id.to_vec().clone())
            .await?;
        let result = self
            .call_contract_with_confirmations(
                DELETE_FILE,
                (
                    file.id.to_bytes(),
                    bucket_id.to_bytes(),
                    file_name,
                    file_idx,
                ),
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
    ) -> Result<U256, ProviderError> {
        let file_name_clone = file_name.clone();
        log_debug!("Getting file index for name: {}", file_name_clone);
        let address = self.get_address().await?;
        let parsed_id: [u8; 32] = file_id.try_into().expect("file_id error");
        let result: U256 = self
            .akave_storage
            .query(
                GET_FILE_INDEX_BY_NAME,
                (file_name, parsed_id),
                address,
                Options::default(),
                None,
            )
            .await
            .map_err(|e| ProviderError::ContractCallError(e.to_string()))?;
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
        let address = self.get_address().await?;
        let result = self
            .akave_storage
            .query(
                GET_FILE_BY_NAME,
                (bucket_id.to_bytes(), file_name),
                address,
                Options::default(),
                None,
            )
            .await
            .map_err(|e| ProviderError::ContractCallError(e.to_string()))?;
        Ok(result)
    }

    /// Signs a message using EIP-712 typed data signing
    ///
    /// For native environments, it uses the private key directly
    /// For WASM environments, it forwards the request to the Ethereum provider
    pub async fn eip712_sign(
        &self,
        domain: Domain,
        message: HashMap<String, serde_json::Value>,
        types: HashMap<String, Vec<TypedData>>,
    ) -> Result<String, ProviderError> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            // Native implementation using the EIP-712 module
            if let Some(key) = &self.key {
                log_debug!("Using native EIP-712 signing with private key");
                let signature =
                    crate::blockchain::eip712::sign_typed_data(key, &domain, &message, &types)
                        .map_err(|e| ProviderError::EncodeError(e.to_string()))?;
                Ok(hex::encode(signature))
            } else {
                Err(ProviderError::EncodeError(
                    "No private key available for signing".into(),
                ))
            }
        }

        #[cfg(target_arch = "wasm32")]
        {
            // WASM implementation using the web3 provider's eth_signTypedData_v4 method
            log_debug!("Using WASM EIP-712 signing via provider");

            // Format the request according to EIP-712
            let eip712_request = crate::blockchain::eip712_utils::encode_eip712_message_for_wasm(
                &domain,
                &message,
                &types,
                "StorageData",
            );

            // Get the current account
            let accounts = self
                .web3_provider
                .eth()
                .accounts()
                .await
                .map_err(|e| ProviderError::AccountError(e.to_string()))?;
            if accounts.is_empty() {
                return Err(ProviderError::AccountError("No accounts available".into()));
            }

            // Call the provider's eth_signTypedData_v4 method
            // Prepare parameters for the JSON-RPC call
            let account = format!("{:?}", accounts[0]);
            let typed_data_json = serde_json::to_string(&eip712_request)
                .map_err(|e| ProviderError::EncodeError(e.to_string()))?;

            // Call the RPC method with proper parameters
            let params = vec![
                serde_json::Value::String(account),
                serde_json::Value::String(typed_data_json),
            ];

            log_debug!("Calling eth_signTypedData_v4 with params: {:?}", params);

            let signature_hex: String = self
                .web3_provider
                .transport()
                .execute("eth_signTypedData_v4", params)
                .await
                .map_err(|e| ProviderError::TransactionError(e.to_string()))?
                .to_string();
            log_debug!("Received signature hex: {}", signature_hex);

            // comes back with "" for some weird reason.
            let trimmed = signature_hex[1..signature_hex.len() - 1].to_string();

            // Convert hex signature to bytes (handle potential 0x prefix)
            let clean_sig = trimmed.trim_start_matches("0x").to_string();
            Ok(clean_sig)
        }
    }
}

#[derive(Error, Debug)]
pub enum ProviderError {
    #[error("transaction error: {0}")]
    TransactionError(String),
    #[error("transaction confirmation timeout: {0}")]
    TransactionConfirmTimeout(String),
    #[error("block number error: {0}")]
    BlockNumberError(String),
    #[error("contract call error: {0}")]
    ContractCallError(String),
    #[error("address error: {0}")]
    AddressError(String),
    #[error("encode error: {0}")]
    EncodeError(String),
    #[error("account error: {0}")]
    AccountError(String),
    #[cfg(not(target_arch = "wasm32"))]
    #[error("key error: {0}")]
    KeyError(String),
}
