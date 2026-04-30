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
use crate::{
    blockchain::eip712_types::{Domain, TypedData},
    log_debug, log_error, log_info,
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

#[derive(Clone)]
pub struct BlockchainProvider {
    pub web3_provider: Web3<ProviderType>,
    confirmations: usize,
    // Native environment private key for signing
    #[cfg(not(target_arch = "wasm32"))]
    key: Option<SecretKey>,
}

impl BlockchainProvider {
    pub fn new(
        _rpc_url: &str,
        confirmations: Option<usize>,
        #[cfg(not(target_arch = "wasm32"))] private_key: Option<&str>,
    ) -> Result<BlockchainProvider, Error> {
        let confirmations_opt = confirmations.unwrap_or(0);

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
                        log_info!("BlockchainProvider initialized successfully for WASM");
                        Ok(Self {
                            web3_provider,
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
            let pvkey = match private_key {
                Some(key) => key.to_string(),
                None => std::env::var("AKAVE_PRIVATE_KEY")
                    .map_err(|_| Error::Decoder(
                        "AKAVE_PRIVATE_KEY environment variable not set. Please set it to your private key.".to_string()
                    ))?,
            };
            let transport = ProviderType::new(_rpc_url);

            match transport {
                Ok(transport_option) => {
                    let web3_provider = Web3::new(transport_option);
                    log_debug!("Creating contract instance");
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
                        key: Some(key),
                        confirmations: confirmations_opt,
                    })
                }
                Err(e) => {
                    log_error!("Failed to get HTTP web3 transport: {}", e);
                    Err(Error::Transport(TransportError::Message(
                        "failed to get http web3 transport".to_string(),
                    )))
                }
            }
        }
    }

    pub async fn call_contract_with_confirmations(
        &self,
        contract: &Contract<ProviderType>,
        function_name: &str,
        params: impl Tokenize + Clone + Send + 'static,
        options: Option<Options>,
    ) -> Result<TransactionReceipt, ProviderError> {
        log_debug!(
            "Calling contract function: {} with confirmations",
            function_name
        );
        let eth = self.web3_provider.eth();

        // Send transaction and get hash
        let hash = self
            .call_contract(contract, function_name, params, options)
            .await?;
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
                .map_err(ProviderError::BlockNumberError)?;
            log_debug!("Current block number: {}", current_block);

            // Get transaction receipt
            let receipt = eth
                .transaction_receipt(hash)
                .await
                .map_err(ProviderError::TransactionError)?;

            match receipt {
                Some(receipt) => {
                    match receipt.status {
                        Some(status) => {
                            if status.low_u64() == 0 {
                                log_error!("Transaction failed with status 0");
                                #[cfg(not(target_arch = "wasm32"))]
                                if let Some(reason) =
                                    self.get_revert_reason(receipt.transaction_hash).await
                                {
                                    return Err(ProviderError::ContractRevert(reason));
                                }
                                return Err(ProviderError::TransactionFailedStatus {
                                    tx_hash: format!("{:?}", receipt.transaction_hash),
                                    function: function_name.to_string(),
                                });
                            }
                        }
                        None => {
                            log_error!("Transaction failed with unknown status");
                            return Err(ProviderError::TransactionUnknownStatus);
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

    pub async fn call_contract(
        &self,
        contract: &Contract<ProviderType>,
        function_name: &str,
        params: impl Tokenize + Clone,
        options: Option<Options>,
    ) -> Result<H256, ProviderError> {
        let txopts = match options {
            Some(opts) => opts,
            None => Options {
                gas: Some(U256::from(1_000_000u64)),
                ..Default::default()
            },
        };

        #[cfg(not(target_arch = "wasm32"))]
        {
            let key = self.key.as_ref().ok_or_else(|| {
                ProviderError::KeyError("Missing key for signed call".to_string())
            })?;
            let key_ref = SecretKeyRef::new(key);

            let hash = contract
                .signed_call(function_name, params, txopts, key_ref)
                .await
                .map_err(ProviderError::Web3CallError)?;

            Ok(hash)
        }

        #[cfg(target_arch = "wasm32")]
        {
            let address = self
                .web3_provider
                .eth()
                .accounts()
                .await
                .map_err(|e| ProviderError::AddressError(e))?[0];
            log_debug!(
                "Calling contract function: {} with confirmations, with address: {}",
                function_name,
                address
            );
            return Ok(contract
                .call(function_name, params, address, txopts)
                .await
                .map_err(|e| ProviderError::ContractCallError(e))?);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    async fn get_revert_reason(&self, hash: H256) -> Option<String> {
        let eth = self.web3_provider.eth();
        let tx = eth
            .transaction(web3::types::TransactionId::Hash(hash))
            .await
            .ok()
            .flatten()?;
        let call_req = web3::types::CallRequest {
            from: tx.from,
            to: tx.to,
            data: Some(tx.input),
            ..Default::default()
        };
        match eth.call(call_req, None).await {
            Err(web3::Error::Rpc(rpc_err)) => {
                if let Some(data_val) = &rpc_err.data {
                    if let Some(hex_str) = data_val.as_str() {
                        if let Ok(bytes) = hex::decode(hex_str.trim_start_matches("0x")) {
                            if let Some(name) =
                                crate::blockchain::contract_errors::decode_revert_reason(&bytes)
                            {
                                return Some(name);
                            }
                        }
                    }
                }
                crate::blockchain::contract_errors::extract_error_from_message(&rpc_err.message)
            }
            _ => None,
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
                .map_err(ProviderError::AccountError)?;

            if accounts.is_empty() {
                log_error!("No accounts available. Please connect your wallet.");
                return Err(ProviderError::NoAccountsAvailable);
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
                        .map_err(ProviderError::AddressError)?;

                    if accounts.is_empty() {
                        return Err(ProviderError::NoAccountsAvailable);
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
                        .map_err(|e| ProviderError::EncodeError(Box::new(e)))?;
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
                .map_err(ProviderError::AccountError)?;
            if accounts.is_empty() {
                return Err(ProviderError::NoAccountsAvailable);
            }

            // Call the provider's eth_signTypedData_v4 method
            // Prepare parameters for the JSON-RPC call
            let account = format!("{:?}", accounts[0]);
            let typed_data_json = serde_json::to_string(&eip712_request)
                .map_err(|e| ProviderError::EncodeError(Box::new(e)))?;

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
                .map_err(|e| ProviderError::Web3CallError(e))?
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
    TransactionError(#[source] web3::Error),
    #[error("transaction confirmation timeout: {0}")]
    TransactionConfirmTimeout(String),
    #[error("block number error: {0}")]
    BlockNumberError(#[source] web3::Error),
    #[error("contract call error: {0}")]
    ContractCallError(#[source] web3::contract::Error),
    #[error("web3 call error: {0}")]
    Web3CallError(#[source] web3::Error),
    #[error("configuration error: {0}")]
    ConfigurationError(String),
    #[error("address error")]
    AddressError(#[source] web3::Error),
    #[error("encode error")]
    EncodeError(#[source] Box<dyn std::error::Error + Send + Sync>),
    #[error("failed to get accounts")]
    AccountError(#[source] web3::Error),
    #[cfg(not(target_arch = "wasm32"))]
    #[error("key error: {0}")]
    KeyError(String),
    #[error("no accounts available")]
    NoAccountsAvailable,
    #[error("serialization error")]
    SerializationError(#[from] serde_json::Error),
    #[error("invalid file id: expected 32 bytes, got {0} bytes")]
    InvalidFileId(usize),
    #[error("transaction failed with status 0: tx_hash={tx_hash}, function={function}")]
    TransactionFailedStatus { tx_hash: String, function: String },
    #[error("transaction receipt has unknown status")]
    TransactionUnknownStatus,
    #[error("offset out of bounds")]
    OffsetOutOfBounds,
    #[cfg(not(target_arch = "wasm32"))]
    #[error("{0}")]
    ContractRevert(String),
}

/// Returns None if the error is an offset-out-of-bounds contract revert (treat as empty page),
/// or Some(err) for any other error.
pub fn ignore_offset_error(err: ProviderError) -> Option<ProviderError> {
    if matches!(err, ProviderError::OffsetOutOfBounds) {
        None
    } else {
        Some(err)
    }
}

/// Checks a web3 contract call error string for the OffsetOutOfBounds selector (0x9605a010).
pub fn is_offset_out_of_bounds_error(err: &web3::contract::Error) -> bool {
    let msg = err.to_string();
    msg.contains("0x9605a010") || msg.contains("OffsetOutOfBounds")
}

/// Maps a ContractCallError to OffsetOutOfBounds if it matches selector 0x9605a010.
pub fn map_contract_error(err: web3::contract::Error) -> ProviderError {
    if is_offset_out_of_bounds_error(&err) {
        ProviderError::OffsetOutOfBounds
    } else {
        ProviderError::ContractCallError(err)
    }
}
