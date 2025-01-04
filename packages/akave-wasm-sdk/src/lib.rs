mod utils;
mod sdk;

use sdk::AkaveSDK;
use std::error::Error;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

pub mod ipc_node_api {
    tonic::include_proto!("ipcnodeapi");
}

async fn build_sdk() -> AkaveSDK {
    let base_url = "http://localhost:3000".to_string();
    AkaveSDK::new(&base_url).await.unwrap()
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);

    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub async fn list_buckets(address: &str) -> Result<JsValue, JsError> {
    let mut client = build_sdk().await;
    let response: Result<sdk::ipcnodeapi::IpcBucketListResponse, Box<dyn Error>> = client.list_buckets(
        address
    ).await;
    
    match response {
        Ok(bucket_list_response) => {
            Ok(serde_wasm_bindgen::to_value(&bucket_list_response)?)
        }
        Err(e) => {
            Err(JsError::new(e.to_string().as_str()))
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub async fn view_bucket(
    address: &str,
    bucket_name: &str,
) -> Result<JsValue, JsError> {
    let mut client = build_sdk().await;
    let response = client.view_bucket(
        address, bucket_name
    ).await;

    match response {
        Ok(bucket_view_response) => {
            Ok(serde_wasm_bindgen::to_value(&bucket_view_response)?)
        }
        Err(e) => {
            Err(JsError::new(e.to_string().as_str()))
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub async fn view_file_info(
    address: &str,
    bucket_name: &str,
    file_name: &str,
) -> Result<JsValue, JsError> {
    let mut client = build_sdk().await;
    let response = client.view_file_info(
        address, bucket_name, file_name
    ).await;

    match response {
        Ok(file_view_response) => {
            Ok(serde_wasm_bindgen::to_value(&file_view_response)?)
        }
        Err(e) => {
            Err(JsError::new(e.to_string().as_str()))
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub async fn list_files(
    address: &str,
    bucket_name: &str,
) -> Result<JsValue, JsError> {
    let mut client = build_sdk().await;
    let response = client.list_files(
        address, bucket_name,
    ).await;

    match response {
        Ok(file_list_response) => {
            Ok(serde_wasm_bindgen::to_value(&file_list_response)?)
        }
        Err(e) => {
            Err(JsError::new(e.to_string().as_str()))
        }
    }
}
