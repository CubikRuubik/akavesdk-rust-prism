mod utils;

use ipc_node_api::{
    ipc_node_api_client::IpcNodeApiClient, IpcBucketListRequest, IpcBucketViewRequest,
    IpcFileListRequest, IpcFileViewRequest,
};
use tonic_web_wasm_client::Client;

use std::future::Future;
use tonic::{Response, Status};

use wasm_bindgen::prelude::*;

pub mod ipc_node_api {
    tonic::include_proto!("ipcnodeapi");
}

fn build_client() -> IpcNodeApiClient<Client> {
    let base_url = "http://localhost:3000".to_string();
    let wasm_client = Client::new(base_url);

    IpcNodeApiClient::new(wasm_client)
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
    let mut client = build_client();
    let response = client.bucket_list(IpcBucketListRequest {
        address: address.to_owned(),
    });

    resolve_response(response).await
}

#[wasm_bindgen]
pub async fn view_bucket(
    address: &str,
    bucket_name: &str,
) -> Result<JsValue, serde_wasm_bindgen::Error> {
    let mut client = build_client();
    let response = client.bucket_view(IpcBucketViewRequest {
        address: address.to_owned(),
        bucket_name: bucket_name.to_owned(),
    });

    resolve_response(response).await
}

#[wasm_bindgen]
pub async fn view_file_info(
    address: &str,
    bucket_name: &str,
    file_name: &str,
) -> Result<JsValue, serde_wasm_bindgen::Error> {
    let mut client = build_client();
    let response = client.file_view(IpcFileViewRequest {
        address: address.to_owned(),
        bucket_name: bucket_name.to_owned(),
        file_name: file_name.to_owned(),
    });

    resolve_response(response).await
}

#[wasm_bindgen]
pub async fn list_files(
    address: &str,
    bucket_name: &str,
) -> Result<JsValue, serde_wasm_bindgen::Error> {
    let mut client = build_client();
    let response = client.file_list(IpcFileListRequest {
        address: address.to_owned(),
        bucket_name: bucket_name.to_owned(),
    });

    resolve_response(response).await
}
