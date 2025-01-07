use crate::sdk::ipcnodeapi;
use crate::sdk::AkaveSDK;

use std::error::Error;
use std::sync::Once;

use wasm_bindgen::prelude::*;

static INIT: Once = Once::new();

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);

    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[wasm_bindgen]
pub struct AkaveWebSDK {
    sdk: AkaveSDK,
}

#[wasm_bindgen]
impl AkaveWebSDK {
    pub async fn new() -> Result<AkaveWebSDK, JsError> {
        // Initialize panic hook only once
        INIT.call_once(|| {
            console_error_panic_hook::set_once();
        });
        Self::new_with_endpoint("http://localhost:3000").await
    }

    #[wasm_bindgen(constructor)]
    pub async fn new_with_endpoint(endpoint: &str) -> Result<AkaveWebSDK, JsError> {
        // Initialize panic hook only once
        INIT.call_once(|| {
            console_error_panic_hook::set_once();
        });

        match AkaveSDK::new(endpoint).await {
            Ok(sdk) => Ok(AkaveWebSDK { sdk }),
            Err(e) => Err(JsError::new(&format!("Failed to initialize SDK: {}", e))),
        }
    }

    pub async fn list_buckets(&mut self, address: &str) -> Result<JsValue, JsError> {
        let response: Result<ipcnodeapi::IpcBucketListResponse, Box<dyn Error>> =
            self.sdk.list_buckets(address).await;

        match response {
            Ok(bucket_list_response) => Ok(serde_wasm_bindgen::to_value(&bucket_list_response)?),
            Err(e) => Err(JsError::new(e.to_string().as_str())),
        }
    }

    pub async fn view_bucket(
        &mut self,
        address: &str,
        bucket_name: &str,
    ) -> Result<JsValue, JsError> {
        let response = self.sdk.view_bucket(address, bucket_name).await;

        match response {
            Ok(bucket_view_response) => Ok(serde_wasm_bindgen::to_value(&bucket_view_response)?),
            Err(e) => Err(JsError::new(e.to_string().as_str())),
        }
    }

    pub async fn view_file_info(
        &mut self,
        address: &str,
        bucket_name: &str,
        file_name: &str,
    ) -> Result<JsValue, JsError> {
        let response = self
            .sdk
            .view_file_info(address, bucket_name, file_name)
            .await;

        match response {
            Ok(file_view_response) => Ok(serde_wasm_bindgen::to_value(&file_view_response)?),
            Err(e) => Err(JsError::new(e.to_string().as_str())),
        }
    }

    pub async fn list_files(
        &mut self,
        address: &str,
        bucket_name: &str,
    ) -> Result<JsValue, JsError> {
        let response = self.sdk.list_files(address, bucket_name).await;

        match response {
            Ok(file_list_response) => Ok(serde_wasm_bindgen::to_value(&file_list_response)?),
            Err(e) => Err(JsError::new(e.to_string().as_str())),
        }
    }
}
