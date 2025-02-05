use web3::error::TransportError;
use web3::transports::eip_1193::*;
use web3::Error;

use crate::blockchain::provider::BlockchainProvider;

impl BlockchainProvider<Eip1193> {
    fn new(provider_url: &str, chain_id: usize) -> Result<BlockchainProvider<Eip1193>, Error> {
        let provider = Provider::default();
        match provider {
            Ok(provider_option) => match provider_option {
                Some(provider) => {
                    let transport = web3::transports::eip_1193::Eip1193::new(provider);
                    let web3 = web3::Web3::new(transport);
                    Ok(Self {
                        web3_provider: web3,
                    })
                }
                None => Err(Error::Transport(TransportError::Message(format!(
                    "failed to build web3 transport",
                )))),
            },

            Err(_) => Err(Error::Transport(TransportError::Message(format!(
                "failed to get wallet provider"
            )))),
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
