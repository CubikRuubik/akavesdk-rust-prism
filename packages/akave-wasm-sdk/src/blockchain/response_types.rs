use web3::{
    contract::tokens::Detokenize,
    ethabi::Token,
    types::{Address, U256},
};

#[derive(Debug)]
pub struct DeleteBucketResponse {}

#[derive(Debug)]
pub struct BucketResponse {
    pub id: [u8; 32],
    pub name: String,
    pub created_at: U256,
    pub owner: Address,
    pub files: Vec<[u8; 32]>,
}

// #[derive(Detokenize)]
/* pub struct ContractResponse {}

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
 */
