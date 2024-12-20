mod utils;

use ipc_node_api::{ipc_node_api_client::IpcNodeApiClient, IpcBucketListRequest};
use tonic_web_wasm_client::Client;

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

#[wasm_bindgen]
pub fn greet(name: &str) {
    let mut final_str: String = "Hello, ".to_owned();
    final_str.push_str(name);
    alert(&final_str);
}

#[wasm_bindgen]
pub async fn list_buckets(address: &str) -> Result<JsValue, serde_wasm_bindgen::Error> {
    log("TEST LOG!!!");
    let mut client = build_client();
    let response = client.bucket_list(IpcBucketListRequest {
        address: address.to_owned(),
    });
    match response.await {
        Ok(buckets_resp) => {
            let buckets_list = buckets_resp.into_inner();
            log(&buckets_list.clone().buckets.first().unwrap().name);
            return serde_wasm_bindgen::to_value(&buckets_list);
        }
        Err(status) => {
            return serde_wasm_bindgen::to_value(&status.message());
        }
    }
}
