// Standard library imports

use std::sync::Arc;

// External crate imports (general)
use web3::{
    contract::{Contract, Options},
    types::{TransactionReceipt, H160},
    Error,
};

// Internal imports
use crate::{
    blockchain::provider::{BlockchainProvider, ProviderError},
    log_debug, log_info,
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
const CHANGE_PUBLIC_ACCESS: &str = "changePublicAccess";
const DELEGATE_ACCESS: &str = "delegateAccess";
const GET_FILE_ACCESS_INFO: &str = "getFileAccessInfo";
const GET_POLICY: &str = "getPolicy";
const GET_VALIDATE_ACCESS: &str = "getValidateAccess";
const GET_VALIDATE_ACCESS_TO_BUCKET: &str = "getValidateAccessToBucket";
const IS_BUCKET_OWNER_OR_DELEGATE: &str = "isBucketOwnerOrDelegate";
const REMOVE_ACCESS: &str = "removeAccess";
const SET_BUCKET_POLICY: &str = "setBucketPolicy";
const SET_FILE_POLICY: &str = "setFilePolicy";
const SET_STORAGE_CONTRACT: &str = "setStorageContract";
const STORAGE_CONTRACT: &str = "storageContract";

#[derive(Clone)]
pub struct AccessManagerContract {
    pub client: Arc<BlockchainProvider>,
    pub contract: Contract<ProviderType>,
}

impl AccessManagerContract {
    pub fn new(
        client: Arc<BlockchainProvider>,
        access_address: &str,
    ) -> Result<AccessManagerContract, Error> {
        log_debug!(
            "Initializing BlockchainProvider with access address: {}",
            access_address
        );

        let access_manager_address = access_address
            .parse::<H160>()
            .map_err(|e| Error::Decoder(format!("Invalid contract address: {}", e)))?;

        let akave_access_manager = Contract::from_json(
            client.web3_provider.eth(),
            access_manager_address,
            include_bytes!("access_manager.json"),
        )
        .map_err(|e| Error::Decoder(format!("Failed to create contract instance: {}", e)))?;

        log_info!(
            "Akave contract address: 0x{:x}",
            akave_access_manager.address()
        );

        Ok(Self {
            client,
            contract: akave_access_manager,
        })
    }

    pub async fn change_public_access(
        &self,
        file_id: [u8; 32],
        is_public: bool,
    ) -> Result<TransactionReceipt, ProviderError> {
        log_debug!("Changing public access for file: {:?}", file_id);
        self.client
            .call_contract_with_confirmations(
                &self.contract,
                CHANGE_PUBLIC_ACCESS,
                (file_id, is_public),
                None,
            )
            .await
    }

    pub async fn delegate_access(
        &self,
        bucket_id: [u8; 32],
        delegated: H160,
    ) -> Result<TransactionReceipt, ProviderError> {
        log_debug!("Delegating access for bucket: {:?}", bucket_id);
        self.client
            .call_contract_with_confirmations(
                &self.contract,
                DELEGATE_ACCESS,
                (bucket_id, delegated),
                None,
            )
            .await
    }

    pub async fn get_file_access_info(
        &self,
        file_id: [u8; 32],
    ) -> Result<(H160, bool), ProviderError> {
        log_debug!("Getting file access info for file: {:?}", file_id);
        let address = self.client.get_address().await?;
        self.contract
            .query(
                GET_FILE_ACCESS_INFO,
                (file_id,),
                address,
                Options::default(),
                None,
            )
            .await
            .map_err(ProviderError::ContractCallError)
    }

    pub async fn get_policy(&self, file_id: [u8; 32]) -> Result<H160, ProviderError> {
        log_debug!("Getting policy for file: {:?}", file_id);
        let address = self.client.get_address().await?;
        self.contract
            .query(GET_POLICY, (file_id,), address, Options::default(), None)
            .await
            .map_err(ProviderError::ContractCallError)
    }

    pub async fn get_validate_access(
        &self,
        file_id: [u8; 32],
        user: H160,
        data: Vec<u8>,
    ) -> Result<bool, ProviderError> {
        log_debug!("Validating access for file: {:?}", file_id);
        let address = self.client.get_address().await?;
        self.contract
            .query(
                GET_VALIDATE_ACCESS,
                (file_id, user, data),
                address,
                Options::default(),
                None,
            )
            .await
            .map_err(ProviderError::ContractCallError)
    }

    /// Validates whether the given user has access to the specified bucket,
    /// using the bucket policy contract.
    pub async fn get_validate_access_to_bucket(
        &self,
        bucket_id: [u8; 32],
        user: H160,
        data: Vec<u8>,
    ) -> Result<bool, ProviderError> {
        log_debug!("Validating access for bucket: {:?}", bucket_id);
        let address = self.client.get_address().await?;
        self.contract
            .query(
                GET_VALIDATE_ACCESS_TO_BUCKET,
                (bucket_id, user, data),
                address,
                Options::default(),
                None,
            )
            .await
            .map_err(ProviderError::ContractCallError)
    }

    pub async fn is_bucket_owner_or_delegate(
        &self,
        bucket_id: [u8; 32],
        user: H160,
    ) -> Result<bool, ProviderError> {
        log_debug!(
            "Checking if user is bucket owner or delegate: {:?}",
            bucket_id
        );
        let address = self.client.get_address().await?;
        self.contract
            .query(
                IS_BUCKET_OWNER_OR_DELEGATE,
                (bucket_id, user),
                address,
                Options::default(),
                None,
            )
            .await
            .map_err(ProviderError::ContractCallError)
    }

    pub async fn remove_access(
        &self,
        bucket_id: [u8; 32],
        delegated: H160,
    ) -> Result<TransactionReceipt, ProviderError> {
        log_debug!("Removing access for bucket: {:?}", bucket_id);
        self.client
            .call_contract_with_confirmations(
                &self.contract,
                REMOVE_ACCESS,
                (bucket_id, delegated),
                None,
            )
            .await
    }

    pub async fn set_bucket_policy(
        &self,
        bucket_id: [u8; 32],
        policy_contract: H160,
    ) -> Result<TransactionReceipt, ProviderError> {
        log_debug!("Setting bucket policy for bucket: {:?}", bucket_id);
        self.client
            .call_contract_with_confirmations(
                &self.contract,
                SET_BUCKET_POLICY,
                (bucket_id, policy_contract),
                None,
            )
            .await
    }

    pub async fn set_file_policy(
        &self,
        file_id: [u8; 32],
        policy_contract: H160,
    ) -> Result<TransactionReceipt, ProviderError> {
        log_debug!("Setting file policy for file: {:?}", file_id);
        self.client
            .call_contract_with_confirmations(
                &self.contract,
                SET_FILE_POLICY,
                (file_id, policy_contract),
                None,
            )
            .await
    }

    pub async fn set_storage_contract(
        &self,
        storage_address: H160,
    ) -> Result<TransactionReceipt, ProviderError> {
        log_debug!("Setting storage contract address: {:?}", storage_address);
        self.client
            .call_contract_with_confirmations(
                &self.contract,
                SET_STORAGE_CONTRACT,
                (storage_address,),
                None,
            )
            .await
    }

    pub async fn storage_contract(&self) -> Result<H160, ProviderError> {
        log_debug!("Getting storage contract address");
        let address = self.client.get_address().await?;
        self.contract
            .query(STORAGE_CONTRACT, (), address, Options::default(), None)
            .await
            .map_err(ProviderError::ContractCallError)
    }
}
