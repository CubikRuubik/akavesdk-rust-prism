pub mod proto {
    tonic::include_proto!("ipcnodeapi");
}

/* use client::proto::{ipcnodeapi_client::IPCNodeAPIClient, IPCNodeAPIRequest};
use tonic::Code;
use tonic_web_wasm_client::Client;
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

wasm_bindgen_test_configure!(run_in_browser);

fn build_client() -> IPCNodeAPIClient<Client> {
    let base_url = "http://connect.akave.ai:5500".to_string();
    let wasm_client = Client::new(base_url);

    IPCNodeAPIClient::new(wasm_client)
}
 */
