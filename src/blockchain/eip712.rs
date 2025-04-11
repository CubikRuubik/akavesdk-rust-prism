// Copyright (C) 2025 Akave
// See LICENSE for copying information.

//! Module providing EIP-712 signing functionality using web3

use std::{collections::HashMap, fmt, str::FromStr};
use web3::{
    signing::{keccak256, recover, Key, SecretKey},
    types::{Address, H256, U256},
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::log_debug;

/// Error type for EIP-712 signing operations
#[derive(Error, Debug)]
pub enum Error {
    #[error("web3 error: {0}")]
    Web3Error(#[from] web3::Error),
    
    #[error("signing error: {0}")]
    SigningError(String),
    
    #[error("value encoding error: {0}")]
    EncodingError(String),
    
    #[error("recovery error: {0}")]
    RecoveryError(String),
}

/// TypedData contains data type and name.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypedData {
    pub name: String,
    pub r#type: String,
}

/// Domain represents the domain separator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Domain {
    pub name: String,
    pub version: String,
    #[serde(rename = "chainId")]
    pub chain_id: U256,
    #[serde(rename = "verifyingContract")]
    pub verifying_contract: Address,
}

impl fmt::Display for Domain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Domain {{ name: {}, version: {}, chainId: {}, verifyingContract: {} }}",
            self.name, self.version, self.chain_id, self.verifying_contract
        )
    }
}

/// Sign signs data with private key according to EIP-712
pub fn sign_typed_data(
    private_key: &SecretKey,
    domain: &Domain,
    data_message: &HashMap<String, serde_json::Value>,
    data_types: &HashMap<String, Vec<TypedData>>,
) -> Result<[u8; 65], Error> {
    log_debug!("hashing data {:?}, domain: {:?}, types: {:?}", data_message, domain, data_types);
    let hash = hash_typed_data(domain, data_message, data_types)?;
    log_debug!("full hash: encoded: {:?}, raw: {:?}", hash, hash.as_bytes());
    

    log_debug!("signing hash hex: {:?}, raw: {:?}", hash, hash.as_bytes());
    // Sign the hash with web3's signing
    let signature = private_key.sign_message(hash.as_bytes())
        .map_err(|e| Error::SigningError(format!("Failed to sign hash: {}", e)))?;
    
    // Convert the signature to bytes - signature has r,s,v components
    let mut sig_bytes = [0u8; 65];
    // Copy r component (32 bytes)
    sig_bytes[0..32].copy_from_slice(&signature.r.to_fixed_bytes());
    // Copy s component (32 bytes)
    sig_bytes[32..64].copy_from_slice(&signature.s.to_fixed_bytes());
    // Set v component (1 byte) and adjust according to EIP-712 standard
    log_debug!("v component: {}", signature.v);
    sig_bytes[64] = (signature.v + 27) as u8;
    log_debug!("signature bytes: {:?}", sig_bytes);
    
    Ok(sig_bytes)
}

/// Recover the signer address from a signature
pub fn recover_signer_address(
    signature: &[u8; 65],
    domain: &Domain,
    data_message: &HashMap<String, serde_json::Value>,
    data_types: &HashMap<String, Vec<TypedData>>,
) -> Result<Address, Error> {
    let hash = hash_typed_data(domain, data_message, data_types)?;
    
    // Extract r, s, and v components
    let r = H256::from_slice(&signature[0..32]);
    let s = H256::from_slice(&signature[32..64]);
    let v = signature[64];
    
    log_debug!("signature bytes: {:?}", signature);
    log_debug!("recovery attempt with v: {}", v);
    
    // Calculate the recovery ID (0 or 1) from the v value
    // In Ethereum, v is typically 27 or 28, which maps to recovery ID 0 or 1
    let recovery_id = if v >= 27 {
        (v - 27) as i32
    } else {
        v as i32 // Already a recovery ID
    };
    
    log_debug!("Using recovery_id: {}", recovery_id);
    
    // Prepare signature for recovery
    let mut sig_bytes = [0u8; 64]; // Only r and s components needed
    sig_bytes[0..32].copy_from_slice(&r.as_bytes());
    sig_bytes[32..64].copy_from_slice(&s.as_bytes());
    
    // Use web3 recover function with the hash, signature bytes, and recovery ID
    let address = web3::signing::recover(hash.as_bytes(), &sig_bytes, recovery_id)
        .map_err(|e| Error::RecoveryError(format!("Failed to recover address: {}", e)))?;
    
    log_debug!("Recovered address: {:?}", address);
    
    Ok(address)
}

/// Create a hash of typed data according to EIP-712 specification
fn hash_typed_data(
    domain: &Domain,
    data_message: &HashMap<String, serde_json::Value>,
    data_types: &HashMap<String, Vec<TypedData>>,
) -> Result<H256, Error> {
    // Define domain types - this is standard for EIP-712
    let domain_types: HashMap<String, Vec<TypedData>> = {
        let mut map = HashMap::new();
        map.insert(
            "EIP712Domain".to_string(),
            vec![
                TypedData {
                    name: "name".to_string(),
                    r#type: "string".to_string(),
                },
                TypedData {
                    name: "version".to_string(),
                    r#type: "string".to_string(),
                },
                TypedData {
                    name: "chainId".to_string(),
                    r#type: "uint256".to_string(),
                },
                TypedData {
                    name: "verifyingContract".to_string(),
                    r#type: "address".to_string(),
                },
            ],
        );
        map
    };

    // Convert domain to a serializable map
    let domain_message: HashMap<String, serde_json::Value> = {
        let mut map = HashMap::new();
        map.insert("name".to_string(), serde_json::Value::String(domain.name.clone()));
        map.insert("version".to_string(), serde_json::Value::String(domain.version.clone()));
        map.insert("chainId".to_string(), serde_json::json!(domain.chain_id.to_string()));
        map.insert("verifyingContract".to_string(), serde_json::json!(format!("{:?}", domain.verifying_contract)));
        map
    };

    // Hash the domain 
    let domain_hash = encode_data("EIP712Domain", &domain_message, &domain_types)?;
    log_debug!("domain hash: encoded: {:?}, raw: {:?}", hex::encode(domain_hash.clone()), domain_hash);
    // Hash the data
    let data_hash = encode_data("StorageData", data_message, data_types)?;
    log_debug!("data hash: encoded: {:?}, raw: {:?}", hex::encode(data_hash.clone()), data_hash);

    // Combine according to EIP-712
    let mut raw_data = vec![0x19, 0x01];
    raw_data.extend_from_slice(&domain_hash);
    raw_data.extend_from_slice(&data_hash);

    Ok(H256::from_slice(&keccak256(&raw_data)))
}

/// Encode type as a string
fn encode_type(primary_type: &str, types: &HashMap<String, Vec<TypedData>>) -> String {
    let mut buffer = String::new();
    buffer.push_str(primary_type);
    buffer.push('(');

    let mut first = true;
    if let Some(fields) = types.get(primary_type) {
        for field in fields {
            if !first {
                buffer.push(',');
            }
            buffer.push_str(&field.r#type);
            buffer.push(' ');
            buffer.push_str(&field.name);
            first = false;
        }
    }

    buffer.push(')');
    buffer
}

/// Compute type hash
fn type_hash(primary_type: &str, types: &HashMap<String, Vec<TypedData>>) -> H256 {
    let encoded = encode_type(primary_type, types);
    H256::from_slice(&keccak256(encoded.as_bytes()))
}

/// Encode data according to EIP-712
fn encode_data(
    primary_type: &str,
    data: &HashMap<String, serde_json::Value>,
    types: &HashMap<String, Vec<TypedData>>,
) -> Result<Vec<u8>, Error> {
    let type_hash = type_hash(primary_type, types);
    
    let mut encoded_data = vec![type_hash.as_bytes().to_vec()];
    
    if let Some(fields) = types.get(primary_type) {
        for field in fields {
            let value = data.get(&field.name).ok_or_else(|| {
                Error::EncodingError(format!("Field {} not found in data", field.name))
            })?;
            
            let encoded_value = encode_value(value, &field.r#type)?;
            encoded_data.push(encoded_value);
        }
    }
    
    // Flatten and hash
    let mut concatenated = Vec::new();
    for data in &encoded_data {
        concatenated.extend_from_slice(data);
    }
    
    Ok(keccak256(&concatenated).to_vec())
}

/// Encode a value based on its type
fn encode_value(value: &serde_json::Value, type_name: &str) -> Result<Vec<u8>, Error> {
    match type_name {
        "string" => {
            let str_val = value.as_str().ok_or_else(|| {
                Error::EncodingError(format!("Expected string, got {:?}", value))
            })?;
            Ok(keccak256(str_val.as_bytes()).to_vec())
        }
        
        "bytes" => {
            let bytes_str = value.as_str().ok_or_else(|| {
                Error::EncodingError(format!("Expected string for bytes, got {:?}", value))
            })?;
            let bytes = hex::decode(&bytes_str.trim_start_matches("0x"))
                .map_err(|_| Error::EncodingError("Invalid bytes format".to_string()))?;
            Ok(keccak256(&bytes).to_vec())
        }
        
        "bytes32" => {
            let bytes_str = value.as_str().ok_or_else(|| {
                Error::EncodingError(format!("Expected string for bytes32, got {:?}", value))
            })?;
            let bytes32 = H256::from_str(bytes_str)
                .map_err(|_| Error::EncodingError("Invalid bytes32 format".to_string()))?;
            let mut buf = [0u8; 32];
            buf.copy_from_slice(bytes32.as_bytes());
            Ok(buf.to_vec())
        }
        
        "uint8" => {
            let num = value.as_u64().ok_or_else(|| {
                Error::EncodingError(format!("Expected number for uint8, got {:?}", value))
            })? as u8;
            let mut buf = [0u8; 32];
            buf[31] = num;
            Ok(buf.to_vec())
        }
        
        "uint64" => {
            let num = value.as_u64().ok_or_else(|| {
                Error::EncodingError(format!("Expected number for uint64, got {:?}", value))
            })?;
            let mut buf = [0u8; 32];
            buf[24..32].copy_from_slice(&num.to_be_bytes());
            Ok(buf.to_vec())
        }
        
        "uint256" => {
            let num_str = value.as_str()
                .map(|s| s.to_string())
                .unwrap_or_else(|| {
                    value.as_u64()
                        .map(|n| n.to_string())
                        .unwrap_or_else(|| "0".to_string())
                });
            let num = U256::from_dec_str(&num_str)
                .map_err(|_| Error::EncodingError("Invalid uint256 value".to_string()))?;
            let mut buf = [0u8; 32];
            num.to_big_endian(&mut buf);
            Ok(buf.to_vec())
        }
        
        "address" => {
            let addr_str = value.as_str().ok_or_else(|| {
                Error::EncodingError(format!("Expected string for address, got {:?}", value))
            })?;

            let addr = Address::from_str(addr_str)
                .map_err(|_| Error::EncodingError("Invalid address format".to_string()))?;
            let mut buf = [0u8; 32];
            buf[12..32].copy_from_slice(addr.as_bytes());
            Ok(buf.to_vec())
        }
        
        _ => Err(Error::EncodingError(format!("Unsupported type: {}", type_name))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use web3::signing::SecretKeyRef;

    #[test]
    fn test_sign_and_recover_with_specific_input() {
        // Private key from the test vector
        let private_key = SecretKey::from_str(
            "4fdee5a3f9362020dd747162674ada0ca9a0f90f6fd2fc69b03e0f932fc4216c"
        ).unwrap();
        
        // Create domain based on the provided test data
        let domain = Domain {
            name: "Storage".to_string(),
            version: "1".to_string(),
            chain_id: U256::from(31337),
            verifying_contract: Address::from_str("e7f1725E7734CE288F8367e1Bb143E90bb3F0512").unwrap(),
        };
        
        // Create data types for StorageData
        let mut data_types = HashMap::new();
        data_types.insert(
            "StorageData".to_string(),
            vec![
                TypedData { name: "chunkCID".to_string(), r#type: "bytes".to_string() },
                TypedData { name: "blockCID".to_string(), r#type: "bytes32".to_string() },
                TypedData { name: "chunkIndex".to_string(), r#type: "uint256".to_string() },
                TypedData { name: "blockIndex".to_string(), r#type: "uint8".to_string() },
                TypedData { name: "nodeId".to_string(), r#type: "bytes".to_string() },
                TypedData { name: "nonce".to_string(), r#type: "uint256".to_string() },
            ],
        );
        
        // Create data message with the test data
        let mut data_message = HashMap::new();
        
        // Convert hex string to bytes for chunkCID
        let chunk_cid_hex = "01701220f3f1253834109022674bf317f0991c42f71474ab99acabe8a158b3d77f46209f";
        let chunk_cid_bytes = hex::decode(chunk_cid_hex).unwrap();
        data_message.insert(
            "chunkCID".to_string(), 
            serde_json::json!(chunk_cid_hex));
        log_debug!("chunk cid bytes: {:?}", chunk_cid_bytes);
        
        // Convert hex string to bytes for blockCID
        let block_cid_hex = "f3f1253834109022674bf317f0991c42f71474ab99acabe8a158b3d77f46209f";
        let block_cid_bytes = hex::decode(block_cid_hex).unwrap();
        data_message.insert(
            "blockCID".to_string(), 
            serde_json::json!(format!("0x{}", block_cid_hex)));
        log_debug!("block cid bytes: {:?}", block_cid_bytes);
        
        // Numeric values are straightforward
        data_message.insert("chunkIndex".to_string(), serde_json::Value::Number(serde_json::Number::from(0)));
        data_message.insert("blockIndex".to_string(), serde_json::Value::Number(serde_json::Number::from(0)));
        
        // Convert hex string to bytes for nodeId
        let node_id_hex = "002408011220f3b11f9768c269198a4f6524989b2b10973adb619d3c3f31f8a83ce3dc405709";
        let node_id_bytes = hex::decode(node_id_hex).unwrap();
        data_message.insert(
            "nodeId".to_string(), 
            serde_json::json!(format!("0x{}", node_id_hex)));
        log_debug!("node id bytes: {:?}", node_id_bytes);
        
        data_message.insert("nonce".to_string(), serde_json::Value::Number(serde_json::Number::from(3)));
        
        // Sign the message
        log::info!("Signing message: {:?} with pk: {:?}", data_message, private_key);
        let signature = sign_typed_data(&private_key, &domain, &data_message, &data_types).unwrap();
        
        // Print the signature for debugging
        println!("Signature: hex: 0x{}, raw: {:?}", hex::encode(&signature), signature);
        
        // Recover signer from the signature
        let recovered_address = recover_signer_address(&signature, &domain, &data_message, &data_types).unwrap();
        
        // Expected address from the private key
        let skref = SecretKeyRef::new(&private_key);
        let expected_address = skref.address();
        
        println!("Recovered address: {}", recovered_address);
        println!("Expected address: {}", expected_address);
        
        // Verify that the recovered address matches the expected one
        assert_eq!(recovered_address, expected_address, "Recovered address doesn't match the expected address");
    }

    #[test]
    fn test_hash_verification() {
        // Create domain based on the provided test data
        let domain = Domain {
            name: "Storage".to_string(),
            version: "1".to_string(),
            chain_id: U256::from(31337),
            verifying_contract: Address::from_str("e7f1725E7734CE288F8367e1Bb143E90bb3F0512").unwrap(),
        };
        
        // Create data types for StorageData
        let mut data_types = HashMap::new();
        data_types.insert(
            "StorageData".to_string(),
            vec![
                TypedData { name: "chunkCID".to_string(), r#type: "bytes".to_string() },
                TypedData { name: "blockCID".to_string(), r#type: "bytes32".to_string() },
                TypedData { name: "chunkIndex".to_string(), r#type: "uint256".to_string() },
                TypedData { name: "blockIndex".to_string(), r#type: "uint8".to_string() },
                TypedData { name: "nodeId".to_string(), r#type: "bytes".to_string() },
                TypedData { name: "nonce".to_string(), r#type: "uint256".to_string() },
            ],
        );
        
        // Create data message with the test data
        let mut data_message = HashMap::new();
        data_message.insert(
            "chunkCID".to_string(), 
            serde_json::Value::String("0x01701220f3f1253834109022674bf317f0991c42f71474ab99acabe8a158b3d77f46209f".to_string())
        );
        data_message.insert(
            "blockCID".to_string(), 
            serde_json::Value::String("0xf3f1253834109022674bf317f0991c42f71474ab99acabe8a158b3d77f46209f".to_string())
        );
        data_message.insert("chunkIndex".to_string(), serde_json::Value::Number(serde_json::Number::from(0)));
        data_message.insert("blockIndex".to_string(), serde_json::Value::Number(serde_json::Number::from(0)));
        data_message.insert(
            "nodeId".to_string(), 
            serde_json::Value::String("0x002408011220f3b11f9768c269198a4f6524989b2b10973adb619d3c3f31f8a83ce3dc405709".to_string())
        );
        data_message.insert("nonce".to_string(), serde_json::Value::Number(serde_json::Number::from(3)));
        
        // Get the hash
        let hash = hash_typed_data(&domain, &data_message, &data_types).unwrap();
        
        println!("EIP-712 Hash: 0x{}", hex::encode(hash.as_bytes()));
        
        // Test individual encodings for debugging
        let domain_types: HashMap<String, Vec<TypedData>> = {
            let mut map = HashMap::new();
            map.insert(
                "EIP712Domain".to_string(),
                vec![
                    TypedData {
                        name: "name".to_string(),
                        r#type: "string".to_string(),
                    },
                    TypedData {
                        name: "version".to_string(),
                        r#type: "string".to_string(),
                    },
                    TypedData {
                        name: "chainId".to_string(),
                        r#type: "uint256".to_string(),
                    },
                    TypedData {
                        name: "verifyingContract".to_string(),
                        r#type: "address".to_string(),
                    },
                ],
            );
            map
        };

        let domain_message: HashMap<String, serde_json::Value> = {
            let mut map = HashMap::new();
            map.insert("name".to_string(), serde_json::Value::String(domain.name.clone()));
            map.insert("version".to_string(), serde_json::Value::String(domain.version.clone()));
            map.insert("chainId".to_string(), serde_json::json!(domain.chain_id.to_string()));
            map.insert("verifyingContract".to_string(), serde_json::json!(Address::from_str("e7f1725E7734CE288F8367e1Bb143E90bb3F0512").unwrap()));
            map
        };

        // Print encoded domain
        let encoded_domain_type = encode_type("EIP712Domain", &domain_types);
        println!("Encoded domain type: {}", encoded_domain_type);
        
        println!("Domain type hash: 0x{}", hex::encode(type_hash("EIP712Domain", &domain_types).as_bytes()));
        
        let domain_hash = encode_data("EIP712Domain", &domain_message, &domain_types).unwrap();
        println!("Domain hash: 0x{}", hex::encode(&domain_hash));
        
        // Print encoded data type
        let encoded_data_type = encode_type("StorageData", &data_types);
        println!("Encoded data type: {}", encoded_data_type);
        
        println!("StorageData type hash: 0x{}", hex::encode(type_hash("StorageData", &data_types).as_bytes()));
        
        let data_hash = encode_data("StorageData", &data_message, &data_types).unwrap();
        println!("Data hash: 0x{}", hex::encode(&data_hash));
    }
}