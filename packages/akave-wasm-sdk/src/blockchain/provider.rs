use std::fmt::format;

use web3::error::TransportError;
use web3::{Error, Transport, Web3};
use web3::transports::eip_1193::*;

use super::BlockchainProvider;


pub struct BlockchainProvider{
   web3_provider: Web3<dyn Transport>
}

impl BlockchainProvider {
    fn new(provider_url: &str, chain_id: usize) -> Result<BlockchainProvider, dyn Error> {
        let provider = Provider::default();
        match provider {
            Ok(provider_option) => {
                match provider_option {
                    Ok(provider) => {
                        let transport = web3::transports::eip_1193::Eip1193::new(provider);
                        let web3 = web3::Web3::new(transport);
                        Ok(Self{web3_provider: web3});
                    },
                    Err(_) => Err(Error::Transport(TransportError::Message(format("failed to build eip1193 web3 transport"))))
                };
            },
            Err(_) => Err(Error::Transport(TransportError::Message(format!("failed to get wallet provider")))),
        }
    }
    async fn sign_message(& self, str: String) -> Result<String, Error> {
        println!("Calling accounts.");
        let accounts = self.web3_provider.eth().accounts().await?;

        println!("Accounts: {:?}", accounts);
        let signed = self.web3_provider.personal().sign(str.into(), accounts[0], "".into()).await?;
        Ok(signed.to_string().into())
    }
    
}