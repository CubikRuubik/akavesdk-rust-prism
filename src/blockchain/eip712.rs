use std::{collections::HashMap, fmt, str::FromStr};

use thiserror::Error;
use web3::{
    signing::{keccak256, Key, SecretKey},
    types::{Address, H256, U256},
};

use crate::{
    blockchain::eip712_types::{Domain, TypedData},
    log_debug,
};

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
    log_debug!(
        "hashing data {:?}, domain {:?}, types: {:?}",
        data_message,
        domain,
        data_types
    );

    let hash = hash_typed_data(domain, data_message, data_types)?;

    // Sign the hash with web3's signing
    let signature = private_key
        .sign_message(hash.as_bytes())
        .map_err(|e| Error::SigningError(format!("Failed to sign hash: {}", e)))?;

    // Convert the signature to bytes - signature has r,s,v components
    let mut sig_bytes = [0u8; 65];
    // Copy r component (32 bytes)
    sig_bytes[0..32].copy_from_slice(&signature.r.to_fixed_bytes());
    // Copy s component (32 bytes)
    sig_bytes[32..64].copy_from_slice(&signature.s.to_fixed_bytes());
    // Set v component (1 byte) and adjust according to EIP-712 standard
    sig_bytes[64] = (signature.v + 27) as u8;

    Ok(sig_bytes)
}

/// Recover the signer address from a signature
#[allow(dead_code)]
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

    // Calculate the recovery ID (0 or 1) from the v value
    // In Ethereum, v is typically 27 or 28, which maps to recovery ID 0 or 1
    let recovery_id = if v >= 27 {
        (v - 27) as i32
    } else {
        v as i32 // Already a recovery ID
    };

    // Prepare signature for recovery
    let mut sig_bytes = [0u8; 64]; // Only r and s components needed
    sig_bytes[0..32].copy_from_slice(r.as_bytes());
    sig_bytes[32..64].copy_from_slice(s.as_bytes());

    // Use web3 recover function with the hash, signature bytes, and recovery ID
    let address = web3::signing::recover(hash.as_bytes(), &sig_bytes, recovery_id)
        .map_err(|e| Error::RecoveryError(format!("Failed to recover address: {}", e)))?;

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
        map.insert(
            "name".to_string(),
            serde_json::Value::String(domain.name.clone()),
        );
        map.insert(
            "version".to_string(),
            serde_json::Value::String(domain.version.clone()),
        );
        map.insert(
            "chainId".to_string(),
            serde_json::json!(domain.chain_id.to_string()),
        );
        map.insert(
            "verifyingContract".to_string(),
            serde_json::json!(format!("{:?}", domain.verifying_contract)),
        );
        map
    };

    // Hash the domain
    let domain_hash = encode_data("EIP712Domain", &domain_message, &domain_types)?;
    log_debug!(
        "domain hash: encoded: {:?}, raw: {:?}",
        hex::encode(domain_hash.clone()),
        domain_hash
    );
    // Hash the data
    let data_hash = encode_data("StorageData", data_message, data_types)?;
    log_debug!(
        "data hash: encoded: {:?}, raw: {:?}",
        hex::encode(data_hash.clone()),
        data_hash
    );

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
            let str_val = value
                .as_str()
                .ok_or_else(|| Error::EncodingError(format!("Expected string, got {:?}", value)))?;
            Ok(keccak256(str_val.as_bytes()).to_vec())
        }

        "bytes" => {
            let bytes_str = value.as_str().ok_or_else(|| {
                Error::EncodingError(format!("Expected string for bytes, got {:?}", value))
            })?;
            let bytes = hex::decode(bytes_str.trim_start_matches("0x"))
                .map_err(|_| Error::EncodingError("Invalid bytes format".to_string()))?;
            log_debug!(
                "bytes: {:?}, keccak256: {:?}",
                bytes,
                keccak256(&bytes).to_vec()
            );
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
            let num_str = value.as_str().map(|s| s.to_string()).unwrap_or_else(|| {
                value
                    .as_u64()
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

        _ => Err(Error::EncodingError(format!(
            "Unsupported type: {}",
            type_name
        ))),
    }
}

#[cfg(test)]
mod tests {
    use cid::multibase::Base;
    use web3::{signing::SecretKeyRef, types::H160};

    use super::*;
    use crate::utils::peer_id::PeerId;

    #[test]
    fn test_sign_and_recover() {
        // Private key from the test vector
        let private_key =
            SecretKey::from_str("59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d")
                .unwrap();

        // Create domain based on the provided test data
        let domain = Domain {
            name: "Storage".to_string(),
            version: "1".to_string(),
            chain_id: U256::from(31337),
            verifying_contract: Address::from_str("1234567890123456789012345678901234567890")
                .unwrap(),
        };

        // Create data types for StorageData
        let mut data_types = HashMap::new();
        data_types.insert(
            "StorageData".to_string(),
            vec![
                TypedData {
                    name: "chunkCID".to_string(),
                    r#type: "bytes".to_string(),
                },
                TypedData {
                    name: "blockCID".to_string(),
                    r#type: "bytes32".to_string(),
                },
                TypedData {
                    name: "chunkIndex".to_string(),
                    r#type: "uint256".to_string(),
                },
                TypedData {
                    name: "blockIndex".to_string(),
                    r#type: "uint8".to_string(),
                },
                TypedData {
                    name: "nodeId".to_string(),
                    r#type: "bytes32".to_string(),
                },
                TypedData {
                    name: "nonce".to_string(),
                    r#type: "uint256".to_string(),
                },
                TypedData {
                    name: "deadline".to_string(),
                    r#type: "uint256".to_string(),
                },
                TypedData {
                    name: "bucketId".to_string(),
                    r#type: "bytes32".to_string(),
                },
            ],
        );

        // Create data message with the test data
        let mut data_message = HashMap::new();

        // Convert hex string to bytes for chunkCID
        let chunk_cid =
            cid::Cid::from_str("bafybeicccfs4u5nmkosg57m4a5k3h4yfuyhk3ftwrgyl4wpsq5maanokiu")
                .unwrap();
        let chunk_cid_hex = hex::encode(chunk_cid.to_bytes());
        data_message.insert(
            "chunkCID".to_string(),
            serde_json::json!(format!("0x{}", chunk_cid_hex)),
        );
        log_debug!(
            "chunk cidstr: {}, hex: {:?}, bytes: {:?}, encoded: {:?}",
            chunk_cid.to_string(),
            chunk_cid_hex,
            chunk_cid.to_bytes(),
            encode_value(&serde_json::json!(format!("0x{}", chunk_cid_hex)), "bytes").unwrap()
        );

        // Convert hex string to bytes for blockCID
        let block_cid =
            cid::Cid::from_str("bafybeia3hparel3smf5n5lw6glchwi7e7olkzhwvh6uuj2tmojtexfr2cu")
                .unwrap();
        let block_cid_hex = block_cid.to_string_of_base(Base::Base16Lower).unwrap();
        let mut bcid = [0u8; 32];
        let bytes = block_cid.to_bytes();
        bcid.copy_from_slice(&bytes[4..]); // trim prefix 4 bytes
        data_message.insert(
            "blockCID".to_string(),
            serde_json::json!(format!("0x{}", hex::encode(bcid))),
        );
        log_debug!(
            "block cid str: {}, hex: {:?}, bytes: {:?}, encoded: {:?}",
            block_cid.to_string(),
            block_cid_hex,
            bcid,
            encode_value(
                &serde_json::json!(format!("0x{}", hex::encode(bcid))),
                "bytes32"
            )
            .unwrap()
        );

        // Numeric values are straightforward√
        data_message.insert(
            "chunkIndex".to_string(),
            serde_json::Value::Number(serde_json::Number::from(1)),
        );
        data_message.insert(
            "blockIndex".to_string(),
            serde_json::Value::Number(serde_json::Number::from(1)),
        );

        // Convert hex string to bytes for nodeId
        let node_id =
            PeerId::from_str("12D3KooWBPkG43Vjb3Rp2PFHYRgKkhAaMZCXAMRVb3M7PrQN2fC5").unwrap();
        let node_id_bytes = node_id.to_bytes();
        let mut node_id_32 = [0u8; 32];
        node_id_32.copy_from_slice(&node_id_bytes[6..38]);
        let node_id_hex = hex::encode(node_id_32);
        data_message.insert(
            "nodeId".to_string(),
            serde_json::json!(format!("0x{}", node_id_hex)),
        );
        log_debug!(
            "node id str: {}, hex: {:?}, bytes: {:?}, encoded: {:?}",
            node_id.to_string(),
            node_id_hex,
            node_id_32,
            encode_value(&serde_json::json!(format!("0x{}", node_id_hex)), "bytes32").unwrap()
        );

        data_message.insert(
            "nonce".to_string(),
            serde_json::Value::String("1234567890".to_string()),
        );
        data_message.insert(
            "deadline".to_string(),
            serde_json::Value::String("1234567999".to_string()),
        );
        let bucket_id = [1u8; 32];
        data_message.insert(
            "bucketId".to_string(),
            serde_json::json!(format!("0x{}", hex::encode(bucket_id))),
        );

        let (auto_data_message, auto_domain, auto_data_types) =
            crate::blockchain::eip712_utils::create_block_eip712_data(
                &block_cid,
                &chunk_cid,
                &node_id_32,
                &bucket_id,
                H160::from_str("0x1234567890123456789012345678901234567890").unwrap(),
                1,
                1,
                U256::from(31337),
                U256::from(1234567890),
                U256::from(1234567999),
            )
            .unwrap();
        assert_eq!(
            data_message, auto_data_message,
            "qData message does not match"
        );
        assert_eq!(domain, auto_domain, "Domain does not match");
        assert_eq!(data_types, auto_data_types, "Data types do not match");

        // Sign the message
        log::info!(
            "Signing message: {:?} with pk: {:?}",
            data_message,
            private_key
        );
        let signature = sign_typed_data(&private_key, &domain, &data_message, &data_types).unwrap();

        // Print the signature for debugging
        println!(
            "Signature: hex: 0x{}, raw: {:?}",
            hex::encode(&signature),
            signature
        );

        // Recover signer from the signature
        let recovered_address =
            recover_signer_address(&signature, &domain, &data_message, &data_types).unwrap();

        // Expected address from the private key
        let skref = SecretKeyRef::new(&private_key);
        let expected_address = skref.address();

        // Verify that the recovered address matches the expected one
        assert_eq!(
            recovered_address, expected_address,
            "Recovered address doesn't match the expected address"
        );
    }

    #[test]
    fn test_sign_against_contract_vectors() {
        let private_key =
            SecretKey::from_str("ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80")
                .unwrap();

        struct Vec {
            chunk_cid: &'static str,
            block_cid: &'static str,
            node_id: &'static str,
            nonce: u64,
            deadline: u64,
            bucket_id: &'static str,
            storage_address: &'static str,
            expected_sig: &'static str,
        }

        let cases = [
            Vec {
                chunk_cid: "86b258127d599eb74c729f97",
                block_cid: "c00612ae8af29b5437ba40df50c46c0175c69b6dc3b3014ed19bda51e318f0f3",
                node_id: "5a604f924e185f6ec5754156e331e9d52df8a669de7e1a060b90e636e0e9e818",
                nonce: 3456789012,
                deadline: 1759859212,
                bucket_id: "930c2de1e6a9a0726f2d7bde19428453d9fdc11fa5c98205ce9b9e794bbd93a2",
                storage_address: "4e7B1E9c3214C973Ff2fc680A9789E8579a5eD9d",
                expected_sig: "726683359604ffe042e73afd7adef9b7f6e13ffd0078999d31bd1cc8c119e1e8324d44cffdc2f771912e500c522082ee94e5f30ac5844c06497e3c49dab8b6de1b",
            },
            Vec {
                chunk_cid: "edf5fb5fdd325e462cd806f2",
                block_cid: "fbeeb197dd90574c97d5993fab0610403197db0f18133033755ec39cab7596c9",
                node_id: "3a59ed631290287c86c90777b2d45926c1a860b1e90828963358d72fa8834389",
                nonce: 2345678901,
                deadline: 1759862780,
                bucket_id: "95f7f023dbf92b2ab036280c44037485c0deec1d854046443bae8ae16c37bc86",
                storage_address: "23618e81E3f5cdF7f54C3d65f7FBc0aBf5B21E8f",
                expected_sig: "47569b36d69bde9e8953cc8c6a01599f0a307850d25e9101c4b1338fbf562d58017bd4ecae535eb330ea7c7ca710fb0055d9d3697e2ebc18902aa32d252eb7361c",
            },
            Vec {
                chunk_cid: "2e3adffef0437b35f247022b",
                block_cid: "fc785a432d1c6d45671f60ed36f44378f63ae4fbbf4ef2a9f0d4951e77e81272",
                node_id: "050f9e0347ebfbdcf50fddf89713b7f37e667d19279d9f550fa7b93237ce29fa",
                nonce: 1234567890,
                deadline: 1759866325,
                bucket_id: "a928e74732b6ca5fd1bf7f3eedfdca3c578a05297157e239e7f7861de2b40f42",
                storage_address: "9965507D1a55bcC2695C58ba16FB37d819B0A4dc",
                expected_sig: "8ccd5143f4b87e898021c4b3a4bf73e3e8d6e8b97e39106374fac72be610629463a0ba6fc4c975c41fbb1ad3940f76a30e6cb916a8e01d09afbe24538ce151ca1b",
            },
        ];

        let data_types = {
            let mut m = HashMap::new();
            m.insert(
                "StorageData".to_string(),
                vec![
                    TypedData {
                        name: "chunkCID".to_string(),
                        r#type: "bytes".to_string(),
                    },
                    TypedData {
                        name: "blockCID".to_string(),
                        r#type: "bytes32".to_string(),
                    },
                    TypedData {
                        name: "chunkIndex".to_string(),
                        r#type: "uint256".to_string(),
                    },
                    TypedData {
                        name: "blockIndex".to_string(),
                        r#type: "uint8".to_string(),
                    },
                    TypedData {
                        name: "nodeId".to_string(),
                        r#type: "bytes32".to_string(),
                    },
                    TypedData {
                        name: "nonce".to_string(),
                        r#type: "uint256".to_string(),
                    },
                    TypedData {
                        name: "deadline".to_string(),
                        r#type: "uint256".to_string(),
                    },
                    TypedData {
                        name: "bucketId".to_string(),
                        r#type: "bytes32".to_string(),
                    },
                ],
            );
            m
        };

        for tc in &cases {
            let domain = Domain {
                name: "Storage".to_string(),
                version: "1".to_string(),
                chain_id: U256::from(31337u64),
                verifying_contract: Address::from_str(tc.storage_address).unwrap(),
            };

            let mut msg = HashMap::new();
            msg.insert(
                "chunkCID".to_string(),
                serde_json::json!(format!("0x{}", tc.chunk_cid)),
            );
            msg.insert(
                "blockCID".to_string(),
                serde_json::json!(format!("0x{}", tc.block_cid)),
            );
            msg.insert(
                "chunkIndex".to_string(),
                serde_json::Value::Number(0.into()),
            );
            msg.insert(
                "blockIndex".to_string(),
                serde_json::Value::Number(0.into()),
            );
            msg.insert(
                "nodeId".to_string(),
                serde_json::json!(format!("0x{}", tc.node_id)),
            );
            msg.insert(
                "nonce".to_string(),
                serde_json::Value::String(tc.nonce.to_string()),
            );
            msg.insert(
                "deadline".to_string(),
                serde_json::Value::String(tc.deadline.to_string()),
            );
            msg.insert(
                "bucketId".to_string(),
                serde_json::json!(format!("0x{}", tc.bucket_id)),
            );

            let sig = sign_typed_data(&private_key, &domain, &msg, &data_types)
                .unwrap_or_else(|e| panic!("sign failed for chunkCID={}: {e}", tc.chunk_cid));
            assert_eq!(
                tc.expected_sig,
                hex::encode(&sig),
                "signature mismatch for chunkCID={}",
                tc.chunk_cid
            );
        }
    }
}
