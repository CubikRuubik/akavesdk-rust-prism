mod utils;
mod transport_client;
mod sdk;

use ipc_node_api::{
    ipc_node_api_client::IpcNodeApiClient, IpcBucketListRequest, IpcBucketViewRequest,
    IpcFileListRequest, IpcFileViewRequest,
};
use sdk::AkaveSDK;
use tonic_web_wasm_client::Client;

use std::future::Future;
use tonic::{Response, Status};

use wasm_bindgen::{convert::FromWasmAbi, prelude::*};

pub mod ipc_node_api {
    tonic::include_proto!("ipcnodeapi");
}

async fn build_sdk() -> AkaveSDK {
    let base_url = "http://localhost:3000".to_string();
    AkaveSDK::new(&base_url, true).await.unwrap()
}

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);

    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

pub async fn resolve_response<T: serde::Serialize>(
    fut: impl Future<Output = Result<Response<T>, Status>>,
) -> Result<JsValue, serde_wasm_bindgen::Error> {
    match fut.await {
        Ok(resp) => {
            let resp_content = resp.into_inner();
            return serde_wasm_bindgen::to_value(&resp_content);
        }
        Err(status) => {
            return serde_wasm_bindgen::to_value(&status.message());
        }
    }
}

#[wasm_bindgen]
pub async fn list_buckets(address: &str) -> Result<JsValue, serde_wasm_bindgen::Error> {
    let mut client = build_sdk().await;
    let response = client.list_buckets(
        address
    ).await.unwrap();

    Ok(serde_wasm_bindgen::to_value(&response)?)
}

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
