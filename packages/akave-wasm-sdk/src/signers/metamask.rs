use web3::error::TransportError;
use web3::Error;
use web3::transports::eip_1193::*;

pub fn get_provider() -> Result<Option<Provider>, Error> {
    let provider: Result<Option<Provider>, wasm_bindgen::JsValue> = Provider::default();
    
    match provider {
        Ok(provider_option) => Ok(provider_option),
        Err(_) => Err(Error::Transport(TransportError::Message(format!("failed to get wallet provider")))),
    }
}

pub async fn sign_message(str: String) -> Result<String, Error> {
    // get current web3 provider
    let provider = get_provider()?;

    match provider {
        Some(provider) => {
            let transport = web3::transports::eip_1193::Eip1193::new(provider);
            let web3 = web3::Web3::new(transport);
            println!("Calling accounts.");
            let accounts = web3.eth().accounts().await?;
            println!("Accounts: {:?}", accounts);
            let signed = web3.personal().sign(str.into(), accounts[0], "".into()).await?;
            Ok(signed.to_string().into())
        }
        None => {
            return Err(
                Error::Transport(
                    TransportError::Message(
                        format!("failed to get wallet provider")
                    )
                )
            )
        }
    }

}