mod client;
mod utils;

// ipcnodeapi_client::IPCNodeAPIClient, IPCNodeAPIRequest

use client::proto::{ipc_node_api_client::IpcNodeApiClient, IpcBucketListRequest};
use tonic_web_wasm_client::Client;

use wasm_bindgen::prelude::*;

fn build_client() -> IpcNodeApiClient<Client> {
    let base_url = "http://connect.akave.ai:5500".to_string();
    let wasm_client = Client::new(base_url);

    IpcNodeApiClient::new(wasm_client)
}

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
}

#[wasm_bindgen]
pub fn greet(name: &str) {
    let mut final_str: String = "Hello, ".to_owned();
    final_str.push_str(name);
    alert(&final_str);
}

#[wasm_bindgen]
pub async fn list_buckets(address: &str) -> Result<JsValue, serde_wasm_bindgen::Error> {
    let mut client = build_client();
    let response = client.bucket_list(IpcBucketListRequest {
        address: address.to_owned(),
    });
    serde_wasm_bindgen::to_value(&response.await.unwrap().into_inner())
}
