mod utils;
mod sdk;

use sdk::AkaveSDK;

#[cfg(target_arch = "wasm32")]
use tonic_web_wasm_client::Client;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{convert::FromWasmAbi, prelude::*};

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
pub async fn list_buckets(address: &str) -> Result<JsValue, serde_wasm_bindgen::Error> {
    let mut client = build_sdk().await;
    let response = client.list_buckets(
        address
    ).await.unwrap();

    Ok(serde_wasm_bindgen::to_value(&response)?)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub async fn view_bucket(
    address: &str,
    bucket_name: &str,
) -> Result<JsValue, serde_wasm_bindgen::Error> {
    let mut client = build_sdk().await;
    let response = client.view_bucket(
        address, bucket_name
    ).await.unwrap();

    Ok(serde_wasm_bindgen::to_value(&response)?)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub async fn view_file_info(
    address: &str,
    bucket_name: &str,
    file_name: &str,
) -> Result<JsValue, serde_wasm_bindgen::Error> {
    let mut client = build_sdk().await;
    let response = client.view_file_info(
        address, bucket_name, file_name
    ).await.unwrap();

    Ok(serde_wasm_bindgen::to_value(&response)?)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub async fn list_files(
    address: &str,
    bucket_name: &str,
) -> Result<JsValue, serde_wasm_bindgen::Error> {
    let mut client = build_sdk().await;
    let response = client.list_files(
        address, bucket_name,
    ).await.unwrap();

    Ok(serde_wasm_bindgen::to_value(&response)?)
}
