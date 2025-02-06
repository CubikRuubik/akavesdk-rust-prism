use std::str::FromStr;

use alloy::{primitives::Address, providers::ProviderBuilder, sol};
use tonic::metadata::KeyRef;
use web3::{
    contract::{Contract, Options},
    error::TransportError,
    signing::{Key, SecretKey, SecretKeyRef},
    types::{TransactionReceipt, H160, H256},
    Error, Web3,
};

#[cfg(target_arch = "wasm32")]
use web3::transports::eip_1193::{Eip1193, Provider};

#[cfg(not(target_arch = "wasm32"))]
use web3::transports::Http;

#[cfg(not(target_arch = "wasm32"))]
use hex_literal::hex;

#[cfg(target_arch = "wasm32")]
type ProviderType = Eip1193;
#[cfg(not(target_arch = "wasm32"))]
type ProviderType = Http;

// Codegen from artifact.
sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    Akave,
    "src/blockchain/contract.json"
);

pub struct BlockchainProvider {
    pub web3_provider: Web3<ProviderType>,
    pub akave: Contract<ProviderType>,
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
            let transport = ProviderType::new(rpc_url);
            match transport {
                Ok(transport_option) => {
                    let web3_provider = Web3::new(transport_option);
                    let contract = Akave::new(
                        contract_address.parse::<Address>().unwrap(),
                        web3_provider.into(),
                    );

                    let akave = Contract::from_json(
                        web3_provider.eth(),
                        contract_address.parse::<H160>().unwrap(),
                        include_bytes!("contract.json"),
                    )
                    .unwrap();

                    Ok(Self {
                        web3_provider,
                        akave,
                    })
                }
                Err(_) => Err(Error::Transport(TransportError::Message(format!(
                    "failed to get http web3 transport"
                )))),
            }
        }
    }

    pub async fn create_bucket(&self, bucket_name: String) -> Result<TransactionReceipt, Error> {
        let accounts = self.web3_provider.eth().accounts().await?;
        /* self.akave
        .call(
            "createBucket",
            (bucket_name,),
            accounts[0],
            Options::default(),
        )
        .await; */

        /* let key =
            SecretKey::from_str("4fdee5a3f9362020dd747162674ada0ca9a0f90f6fd2fc69b03e0f932fc4216c")
                .unwrap();
        let key_ref = SecretKeyRef::new(&key);
        self.akave
            .signed_call_with_confirmations(
                "createBucket",
                (bucket_name,),
                Options::default(),
                5,
                key_ref,
            )
            .await */
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
