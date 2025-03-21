// Standard library imports
use std::time::Duration;

// External crate imports (general)
use futures::StreamExt;
use web3::{
    contract::{tokens::Tokenize, Contract, Options},
    error::TransportError,
    types::{TransactionReceipt, H160, H256, U256},
    Error, Web3,
};

// Internal imports
use super::ipc_types::{BucketResponse, FileResponse};

// Target-specific imports
#[cfg(target_arch = "wasm32")]
mod wasm_imports {
    pub use web3::transports::eip_1193::{Eip1193, Provider};
}

#[cfg(target_arch = "wasm32")]
use wasm_imports::*;

#[cfg(not(target_arch = "wasm32"))]
mod native_imports {
    pub use web3::transports::http::Http;
    pub use web3::signing::{Key, SecretKey, SecretKeyRef};
    pub use std::str::FromStr;
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
    poll_interval: Duration,
    confirmations: usize,
    #[cfg(not(target_arch = "wasm32"))]
    key: Option<SecretKey>,
}

impl BlockchainProvider {
    pub fn new(
        _rpc_url: &str,
        access_address: &str,
        poll_interval: Option<Duration>,
        confirmations: Option<usize>,
    ) -> Result<BlockchainProvider, Error> {
        let poll_interval_opt = match poll_interval {
            Some(value) => value,
            None => Duration::from_millis(100),
        };

        let confirmations_opt = match confirmations {
            Some(value) => value,
            None => 1,
        };
        #[cfg(target_arch = "wasm32")]
        {
            let provider = Provider::default();
            match provider {
                Ok(provider_option) => match provider_option {
                    Some(provider) => {
                        let transport = web3::transports::eip_1193::Eip1193::new(provider);
                        let web3_provider = web3::Web3::new(transport);
                        let akave = Contract::from_json(
                            web3_provider.eth(),
                            access_address.parse::<H160>().unwrap(),
                            include_bytes!("contract.json"),
                        )
                        .unwrap();
                        Ok(Self {
                            web3_provider,
                            akave,
                            poll_interval: poll_interval_opt,
                            confirmations: confirmations_opt,
                            #[cfg(not(target_arch = "wasm32"))]
                            key: None,
                        })
                    }
                    None => Err(Error::Transport(TransportError::Message(format!(
                        "failed to build eip_1193 web3 transport",
                    )))),
                },

                Err(_) => Err(Error::Transport(TransportError::Message(format!(
                    "failed to get eip_1193 wallet provider"
                )))),
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let pvkey: &str = include_str!("user.akvf.key");
            let transport = ProviderType::new(_rpc_url);

            match transport {
                Ok(transport_option) => {
                    let web3_provider = Web3::new(transport_option);
                    let akave = Contract::from_json(
                        web3_provider.eth(),
                        access_address.parse::<H160>().unwrap(),
                        include_bytes!("contract.json"),
                    )
                    .unwrap();

                    let key = SecretKey::from_str(pvkey).unwrap();

                    Ok(Self {
                        web3_provider,
                        akave,
                        key: Some(key),
                        poll_interval: poll_interval_opt,
                        confirmations: confirmations_opt,
                    })
                }
                Err(_) => Err(Error::Transport(TransportError::Message(format!(
                    "failed to get http web3 transport"
                )))),
            }
        }
    }

    async fn call_contract_with_confirmations(
        &self,
        function_name: &str,
        params: impl Tokenize,
    ) -> Result<TransactionReceipt, Box<dyn std::error::Error>> {
        let eth = self.web3_provider.eth();
    
        let filter_stream = self
            .web3_provider
            .eth_filter()
            .create_blocks_filter()
            .await?
            .stream(self.poll_interval)
            .skip(self.confirmations - 1);
    
        // Use tokio::pin! for native
        #[cfg(not(target_arch = "wasm32"))]
        tokio::pin!(filter_stream);

        // Use futures::pin_mut! for wasm
        #[cfg(target_arch = "wasm32")]
        futures::pin_mut!(filter_stream);
    
        let hash = self.call_contract(function_name, params).await?;

        while let Some(result_log) = filter_stream.next().await {
            println!("log received.");
            let log: H256 = match result_log {
                Ok(log) => log,
                Err(e) => {
                    println!("{}", e);
                    continue;
                }
            };
            println!("log: {}", log);
            println!("current block number: {}", eth.block_number().await?);

            let receipt = eth.transaction_receipt(hash).await?;
            let check = receipt.and_then(|receipt| receipt.block_number);

            match check {
                Some(confirmation_block_number) => {
                    let block_number = eth.block_number().await?;
                    println!("tx: {}, bs: {}", confirmation_block_number, block_number);
                    if confirmation_block_number.low_u64() + (self.confirmations - 1) as u64
                        <= block_number.low_u64()
                    {
                        break;
                    }
                }
                None => {} // TODO: Add some sort of timeout?
            }
        }

        match eth.transaction_receipt(hash).await? {
            Some(receipt_option) => Ok(receipt_option),
            None => Err("Error getting tx receipt")?,
        }
    }

    async fn call_contract(
        &self,
        function_name: &str,
        params: impl Tokenize,
    ) -> Result<H256, Box<dyn std::error::Error>> {
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
            let address = self.web3_provider.eth().accounts().await?[0];
            return Ok(self.akave.call(function_name, params, address, txopts).await?);
        }
    }
    

    pub async fn get_address(&self) -> Result<H160, Box<dyn std::error::Error>> {
        #[cfg(target_arch = "wasm32")] {
           Ok(self.web3_provider.eth().accounts().await?[0])
        }
        
        #[cfg(not(target_arch = "wasm32"))] {
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
        let id: [u8; 32] = bucket_id.try_into().expect("bucket_id error");
        self.call_contract_with_confirmations(CREATE_FILE, (id, file_name))
            .await
    }

    pub async fn commit_file(
        &self,
        bucket_id: [u8; 32],
        file_name: String,
        size: U256,
        root_cid: Vec<u8>,
    ) -> Result<TransactionReceipt, Box<dyn std::error::Error>> {
        self.call_contract_with_confirmations(COMMIT_FILE, (bucket_id, file_name, size, root_cid))
            .await
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
        // let r_cid: [u8; 32] = root_cid.try_into().expect("root_cid error");
        self.call_contract_with_confirmations(
            ADD_FILE_CHUNK,
            (root_cid, bucket_id, file_name, size, cids, sizes, index),
        )
        .await
    }

    pub async fn create_bucket(
        &self,
        bucket_name: String,
    ) -> Result<TransactionReceipt, Box<dyn std::error::Error>> {
        self.call_contract_with_confirmations(CREATE_BUCKET, (bucket_name,))
            .await
    }

    pub async fn delete_bucket(
        &self,
        bucket_id: Vec<u8>,
        bucket_name: String,
        bucket_idx: U256,
    ) -> Result<TransactionReceipt, Box<dyn std::error::Error>> {
        /* let id: &[u8] = &bucket_id[..]; */
        let id: [u8; 32] = bucket_id.try_into().expect("bucket_id error");
        self.call_contract_with_confirmations(DELETE_BUCKET, (id, bucket_name, bucket_idx))
            .await
    }

    pub async fn get_bucket_by_name(
        &self,
        bucket_name: String,
    ) -> Result<BucketResponse, Box<dyn std::error::Error>> {
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
        Ok(result)
    }

    pub async fn get_bucket_index_by_name(
        &self,
        bucket_name: String,
    ) -> Result<U256, Box<dyn std::error::Error>> {
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
        Ok(result)
    }

    pub async fn delete_file(
        &self,
        file_name: String,
        bucket_id: Vec<u8>,
    ) -> Result<TransactionReceipt, Box<dyn std::error::Error>> {
        // let parsed_file_id: [u8; 32] = file_id.try_into().expect("file_id error");
        let parsed_bucket_id: [u8; 32] = bucket_id.clone().try_into().expect("bucket_id error");

        let file = self
            .get_file_by_name(bucket_id, file_name.to_string())
            .await?;
        let file_idx = self
            .get_file_index_by_name(file.name, file.id.to_vec().clone())
            .await?;
        self.call_contract_with_confirmations(
            DELETE_FILE,
            (file.id, parsed_bucket_id, file_name, file_idx),
        )
        .await
    }

    pub async fn get_file_index_by_name(
        &self,
        file_name: String,
        file_id: Vec<u8>,
    ) -> Result<U256, Box<dyn std::error::Error>> {
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
        Ok(result)
    }

    pub async fn get_file_by_name(
        &self,
        bucket_id: Vec<u8>,
        file_name: String,
    ) -> Result<FileResponse, Box<dyn std::error::Error>> {
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
        Ok(result)
    }

    async fn sign_message(&self, str: String) -> Result<String, Error> {
        println!("Calling accounts.");
        let accounts = self.web3_provider.eth().accounts().await?;

        println!("Accounts: {:?}", accounts);
        let signed = self
            .web3_provider
            .personal()
            .sign(str.into(), accounts[0], "".into())
            .await?;
        Ok(signed.to_string().into())
    }
}
