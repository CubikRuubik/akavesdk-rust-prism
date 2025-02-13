use core::sync;
use std::str::FromStr;

use web3::{
    api::Eth,
    contract::{tokens::Tokenize, Contract, Options},
    error::{self, TransportError},
    signing::{Key, SecretKey, SecretKeyRef},
    types::{H160, H256, U256, U64},
    Error, Transport, Web3,
};

#[cfg(target_arch = "wasm32")]
use web3::transports::eip_1193::{Eip1193, Provider};

#[cfg(not(target_arch = "wasm32"))]
use web3::transports::http::Http;

use super::response_types::BucketResponse;

#[cfg(target_arch = "wasm32")]
type ProviderType = Eip1193;
#[cfg(not(target_arch = "wasm32"))]
type ProviderType = Http;

const CREATE_BUCKET: &str = "createBucket";
const DELETE_BUCKET: &str = "deleteBucket";
const GET_BUCKET_BY_NAME: &str = "getBucketByName";

pub struct BlockchainProvider {
    pub web3_provider: Web3<ProviderType>,
    pub akave: Contract<ProviderType>,
    key: Option<SecretKey>,
}

impl BlockchainProvider {
    pub fn new(rpc_url: &str, contract_address: &str) -> Result<BlockchainProvider, Error> {
        #[cfg(target_arch = "wasm32")]
        {
            let provider = Provider::default();
            match provider {
                Ok(provider_option) => match provider_option {
                    Some(provider) => {
                        let transport = Provider::new(provider);
                        let web3_provider = web3::Web3::new(transport);
                        let akave = Contract::from_json(
                            web3_provider.eth(),
                            contract_address.parse::<H160>().unwrap(),
                            include_bytes!("contract.json"),
                        )
                        .unwrap();
                        Ok(Self {
                            web3_provider,
                            akave,
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
            let transport = ProviderType::new(rpc_url);
            match transport {
                Ok(transport_option) => {
                    let web3_provider = Web3::new(transport_option);
                    let akave = Contract::from_json(
                        web3_provider.eth(),
                        contract_address.parse::<H160>().unwrap(),
                        include_bytes!("contract.json"),
                    )
                    .unwrap();

                    let key = SecretKey::from_str(pvkey).unwrap();

                    Ok(Self {
                        web3_provider,
                        akave,
                        key: Some(key),
                    })
                }
                Err(_) => Err(Error::Transport(TransportError::Message(format!(
                    "failed to get http web3 transport"
                )))),
            }
        }
    }

    pub async fn get_address(&self) -> Result<H160, Box<dyn std::error::Error>> {
        match self.key {
            Some(key) => Ok(SecretKeyRef::new(&key).address()),
            None => Ok(self.web3_provider.eth().accounts().await?[0]),
        }
    }

    async fn _transaction_receipt_block_number_check<T: Transport>(
        eth: Eth<T>,
        hash: H256,
    ) -> error::Result<Option<U64>> {
        let receipt = eth.transaction_receipt(hash).await?;
        Ok(receipt.and_then(|receipt| receipt.block_number))
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

        match self.key {
            Some(key) => {
                let key_ref = SecretKeyRef::new(&key);

                let hash = self
                    .akave
                    .signed_call(function_name, params, txopts, key_ref)
                    .await?;

                // cant wait for transaction confirmation for some reason
                std::thread::sleep(std::time::Duration::from_secs(5));

                Ok(hash)
            }
            None => {
                let address = self.web3_provider.eth().accounts().await?[0];
                Ok(self
                    .akave
                    .call(function_name, params, address, txopts)
                    .await?)
            }
        }
    }

    pub async fn create_bucket(
        &self,
        bucket_name: String,
    ) -> Result<H256, Box<dyn std::error::Error>> {
        self.call_contract(CREATE_BUCKET, (bucket_name,)).await
    }

    pub async fn delete_bucket(
        &self,
        bucket_id: Vec<u8>,
        bucket_name: String,
    ) -> Result<H256, Box<dyn std::error::Error>> {
        /* let id: &[u8] = &bucket_id[..]; */
        let id: [u8; 32] = bucket_id.try_into().expect("bucket_id error");

        self.call_contract(DELETE_BUCKET, (id, bucket_name)).await
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
