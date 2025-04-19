

use std::collections::HashMap;
use cid::Cid;
use libp2p::PeerId;
use web3::types::{H160, U256};
use crate::{blockchain::eip712_types::{Domain, TypedData}, log_debug};

#[cfg(target_arch = "wasm32")]
use serde_json::Value as JsonValue;

//create_block_eip712_data(&block_1mb.cid, &ipc_chunk.cid, b_node_id, self.storage.akave_storage.address(), index as i64, ipc_chunk.cid, nonce)?;
pub fn create_block_eip712_data(
    block_cid: &Cid,
    chunk_cid: &Cid,
    node_id: &PeerId,
    verifying_contract: H160,
    chunk_index: i64,
    block_index: i64,
    nonce: U256,
) -> Result<(HashMap<String, serde_json::Value>, Domain, HashMap<String, Vec<TypedData>>), Box<dyn std::error::Error>> {
    // Create domain based on the provided test data
    let domain = Domain {
        name: "Storage".to_string(),
        version: "1".to_string(),
        chain_id: U256::from(78964),
        verifying_contract: verifying_contract,
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
    let chunk_cid_hex = hex::encode(chunk_cid.to_bytes());
    data_message.insert(
        "chunkCID".to_string(), 
        serde_json::json!(format!("0x{}", chunk_cid_hex)));
    
    // Convert hex string to bytes for blockCID
    let mut bcid = [0u8; 32];
    let bytes = block_cid.to_bytes();
    bcid.copy_from_slice(&bytes[4..]); // trim prefix 4 bytes
    data_message.insert(
        "blockCID".to_string(), 
        serde_json::json!(format!("0x{}", hex::encode(bcid))));
    
    // Numeric values are straightforward√
    data_message.insert("chunkIndex".to_string(), serde_json::Value::Number(serde_json::Number::from(chunk_index)));
    data_message.insert("blockIndex".to_string(), serde_json::Value::Number(serde_json::Number::from(block_index)));
    
    // Convert hex string to bytes for nodeId
    let node_id_hex =  hex::encode(node_id.to_bytes());
    log_debug!("nodeId hex: {}, str {}", node_id_hex, node_id.to_base58());
    data_message.insert(
        "nodeId".to_string(), 
        serde_json::json!(format!("0x{}", node_id_hex)));
    data_message.insert("nonce".to_string(), serde_json::Value::Number(serde_json::Number::from(nonce.as_u64())));
    // return data message, domain message and data types
    Ok((data_message, domain, data_types))
}


/// WASM: Converts EIP-712 data into the JSON format required by MetaMask for eth_signTypedData_v4
#[cfg(target_arch = "wasm32")]
pub fn encode_eip712_message_for_wasm(
    domain: &Domain,
    message: &HashMap<String, serde_json::Value>,
    types: &HashMap<String, Vec<TypedData>>,
    primary_type: &str,
) -> serde_json::Value {
    use serde_json::json;

    // Convert Domain to JSON (camelCase)
    let domain_json = json!({
        "name": domain.name,
        "version": domain.version,
        "chainId": domain.chain_id,
        "verifyingContract": format!("0x{:x}", domain.verifying_contract),
    });

    // Convert types to JSON (as expected by MetaMask)
    let mut types_json = serde_json::Map::new();
    
    // Add EIP712Domain type definition
    let domain_fields = vec![
        json!({ "name": "name", "type": "string" }),
        json!({ "name": "version", "type": "string" }),
        json!({ "name": "chainId", "type": "uint256" }),
        json!({ "name": "verifyingContract", "type": "address" }),
    ];
    types_json.insert("EIP712Domain".to_string(), JsonValue::Array(domain_fields));
    
    // Add other type definitions
    for (type_name, fields) in types.iter() {
        let fields_json: Vec<JsonValue> = fields.iter().map(|f| {
            json!({
                "name": f.name,
                "type": f.r#type
            })
        }).collect();
        types_json.insert(type_name.clone(), JsonValue::Array(fields_json));
    }

    // Compose the final JSON structure
    let result = json!({
        "types": JsonValue::Object(types_json),
        "domain": domain_json,
        "primaryType": primary_type,
        "message": message
    });
    
    log_debug!("[WASM] EIP-712 JSON for MetaMask: {}", result);
    result
}