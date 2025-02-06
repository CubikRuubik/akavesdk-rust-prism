use std::str::FromStr;

use alloy_sol_types::sol_data::Array;
use serde::Serialize;
use web3::{
    contract::{
        tokens::{Detokenize, Tokenize},
        Contract, Options,
    },
    error::TransportError,
    ethabi::{FixedBytes, Token, TupleParam, Uint},
    signing::{Key, SecretKey, SecretKeyRef},
    types::{Address, BytesArray, H160, H256, U256},
    Error, Web3,
};

#[cfg(target_arch = "wasm32")]
use web3::transports::eip_1193::{Eip1193, Provider};

#[cfg(not(target_arch = "wasm32"))]
use web3::transports::http::Http;

#[cfg(target_arch = "wasm32")]
type ProviderType = Eip1193;
#[cfg(not(target_arch = "wasm32"))]
type ProviderType = Http;

const CREATE_BUCKET: &str = "createBucket";

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
                Ok(self
                    .akave
                    .signed_call(function_name, params, txopts, key_ref)
                    .await?)
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

    pub async fn get_bucket_by_name(
        &self,
        bucket_name: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let address = self.get_address().await?;
        let result: BucketResponse = self
            .akave
            .query(
                "getBucketByName",
                (bucket_name,),
                address,
                Options::default(),
                None,
            )
            .await
            .unwrap();

        println!("{}", result.name);

        todo!()
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

#[derive(Debug)]
pub struct BucketResponse {
    pub id: [u8; 32],
    pub name: String,
    pub created_at: U256,
    pub owner: Address,
    pub files: Vec<[u8; 32]>,
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
                            Ok(file_bytes)
                        } else {
                            Err(web3::contract::Error::InterfaceUnsupported)
                        }
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(BucketResponse {
                    id: id_bytes,
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
