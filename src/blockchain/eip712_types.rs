use serde::{Deserialize, Serialize};
use web3::types::{Address, U256};

/// TypedData contains data type and name.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TypedData {
    pub name: String,
    pub r#type: String,
}

/// Domain represents the domain separator.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Domain {
    pub name: String,
    pub version: String,
    #[serde(rename = "chainId")]
    pub chain_id: U256,
    #[serde(rename = "verifyingContract")]
    pub verifying_contract: Address,
}
