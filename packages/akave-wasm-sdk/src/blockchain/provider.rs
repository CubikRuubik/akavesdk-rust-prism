use web3::{error::TransportError, Error, Web3};

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

pub struct BlockchainProvider {
    pub web3_provider: Web3<ProviderType>,
}

impl BlockchainProvider {
    fn new(rpc_url: &str) -> Result<BlockchainProvider, Error> {
        #[cfg(target_arch = "wasm32")]
        {
            let provider = Provider::default();
            match provider {
                Ok(provider_option) => match provider_option {
                    Some(provider) => {
                        let transport = Provider::new(provider);
                        let web3 = web3::Web3::new(transport);
                        Ok(Self {
                            web3_provider: web3,
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
                Ok(transport_option) => Ok(Self {
                    web3_provider: Web3::new(transport_option),
                }),
                Err(_) => Err(Error::Transport(TransportError::Message(format!(
                    "failed to get http web3 transport"
                )))),
            }
        }
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
