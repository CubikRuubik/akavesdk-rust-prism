use crate::blockchain::ipc_types::BucketResponse;
use crate::sdk::ipcnodeapi;
use crate::sdk::AkaveIpcSDK as AkaveSDK;
use crate::sdk_types::IpcFileList;

use std::sync::Once;

use wasm_bindgen::prelude::*;
use wasm_bindgen_file_reader::WebSysFile;
use web_sys::File;

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

    #[wasm_bindgen(js_name = "listBuckets")] // typescript convection is camelCase
    pub async fn list_buckets(
        &mut self,
        address: &str,
    ) -> Result<ipcnodeapi::IpcBucketListResponse, JsError> {
        let response = self.sdk.list_buckets(address).await;
        match response {
            Ok(bucket_list_response) => Ok(bucket_list_response),
            Err(e) => Err(JsError::new(e.to_string().as_str())),
        }
    }

    #[wasm_bindgen(js_name = "viewBucket")] // typescript convection is camelCase
    pub async fn view_bucket(
        &mut self,
        address: &str,
        bucket_name: &str,
    ) -> Result<ipcnodeapi::IpcBucketViewResponse, JsError> {
        let response = self.sdk.view_bucket(address, bucket_name).await;
        match response {
            Ok(bucket_view_response) => Ok(bucket_view_response),
            Err(e) => Err(JsError::new(e.to_string().as_str())),
        }
    }

    #[wasm_bindgen(js_name = "viewFileInfo")] // typescript convection is camelCase
    pub async fn view_file_info(
        &mut self,
        address: &str,
        bucket_name: &str,
        file_name: &str,
    ) -> Result<ipcnodeapi::IpcFileViewResponse, JsError> {
        let response = self
            .sdk
            .view_file_info(address, bucket_name, file_name)
            .await;

        match response {
            Ok(file_view_response) => Ok(file_view_response),
            Err(e) => Err(JsError::new(e.to_string().as_str())),
        }
    }

    #[wasm_bindgen(js_name = "listFiles")] // typescript convection is camelCase
    pub async fn list_files(
        &mut self,
        address: &str,
        bucket_name: &str,
    ) -> Result<IpcFileList, JsError> {
        let response = self.sdk.list_files(address, bucket_name).await;

        match response {
            Ok(file_list_response) => Ok(file_list_response),
            Err(e) => Err(JsError::new(e.to_string().as_str())),
        }
    }

    #[wasm_bindgen(js_name = "createBucket")] // typescript convection is camelCase
    pub async fn create_bucket(
        &mut self,
        bucket_name: &str,
    ) -> Result<BucketResponse, JsError> {
        // TODO: this needs a blockchain transaction
        // FIXME: Although there's this call in the grpc, in akave code they dont use it to create a bucket?
        let response = self.sdk.create_bucket(bucket_name).await;
        match response {
            Ok(bucket_create_response) => Ok(bucket_create_response),
            Err(e) => Err(JsError::new(e.to_string().as_str())),
        }
    }

    #[wasm_bindgen(js_name = "deleteBucket")] // typescript convection is camelCase
    pub async fn delete_bucket(&mut self, address: &str, bucket_name: &str) -> Result<(), JsError> {
        // TODO: this needs a blockchain transaction
        // FIXME: Although there's this call in the grpc, in akave code they dont use it to create a bucket?
        let response = self.sdk.delete_bucket(address, bucket_name).await;
        match response {
            Ok(_) => Ok(()),
            Err(e) => Err(JsError::new(e.to_string().as_str())),
        }
    }

    #[wasm_bindgen(js_name = "deleteFile")] // typescript convection is camelCase
    pub async fn delete_file(
        &mut self,
        address: &str,
        bucket_name: &str,
        file_name: &str,
    ) -> Result<(), JsError> {

        let response = self
            .sdk
            .delete_file(address, bucket_name, file_name)
            .await;
        match response {
            Ok(_) => Ok(()),
            Err(e) => Err(JsError::new(e.to_string().as_str())),
        }
    }

    #[wasm_bindgen(js_name = "uploadFile")] // typescript convection is camelCase
    pub async fn upload_file(
        &mut self,
        bucket_name: &str,
        file: File,
        key: &str,
    ) -> Result<String, JsError> {
        let file_name = file.name().clone();
        let wf = WebSysFile::new(file);

        let response = self.sdk.upload_file(bucket_name, file_name.as_str(), wf, Some(key)).await;

        match response {
            Ok(upload_response) => Ok(upload_response.transaction_hash.to_string()),
            Err(e) => Err(JsError::new(e.to_string().as_str())),
        }
    }

    // #[wasm_bindgen(js_name = "downloadFile")] // typescript convection is camelCase
    // pub async fn download_file(
    //     &mut self,
    //     address: &str,
    //     bucket_name: &str,
    //     file_name: &str,
    //     key: &str,
    // ) -> Result<File, JsValue> {
    //     let data = self
    //         .sdk
    //         .download_file(address, bucket_name, file_name, key)
    //         .await;
    //     let js_data = JsValue::from(data);
    //     let file = web_sys::File::new_with_u8_array_sequence(&js_data, file_name);
    //     return file;
    // }
}
