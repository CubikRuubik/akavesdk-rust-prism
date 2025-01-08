use crate::sdk::ipcnodeapi;
use crate::sdk::AkaveSDK;

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
    ) -> Result<ipcnodeapi::IpcFileListResponse, JsError> {
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
    ) -> Result<ipcnodeapi::IpcBucketCreateResponse, JsError> {
        // TODO: this needs a blockchain transaction
        // FIXME: Although there's this call in the grpc, in akave code they dont use it to create a bucket?
        let response = self.sdk.create_bucket(bucket_name).await;
        match response {
            Ok(bucket_create_response) => Ok(bucket_create_response),
            Err(e) => Err(JsError::new(e.to_string().as_str())),
        }
    }

    #[wasm_bindgen(js_name = "deleteBucket")] // typescript convection is camelCase
    pub async fn delete_bucket(&mut self) -> Result<ipcnodeapi::IpcBucketDeleteResponse, JsError> {
        // TODO: this needs a blockchain transaction
        // FIXME: Although there's this call in the grpc, in akave code they dont use it to create a bucket?
        let response = self.sdk.delete_bucket().await;
        match response {
            Ok(bucket_delete_response) => Ok(bucket_delete_response),
            Err(e) => Err(JsError::new(e.to_string().as_str())),
        }
    }

    #[wasm_bindgen(js_name = "deleteFile")] // typescript convection is camelCase
    pub async fn delete_file(
        &mut self,
        bucket_id: Vec<u8>,
        transaction: Vec<u8>,
        file_name: &str,
    ) -> Result<ipcnodeapi::IpcFileDeleteResponse, JsError> {
        // TODO: this needs a blockchain transaction
        // FIXME: Although there's this call in the grpc, in akave code they dont use it to create a bucket?
        let response = self
            .sdk
            .delete_file(bucket_id, transaction, file_name)
            .await;
        match response {
            Ok(file_delete_response) => Ok(file_delete_response),
            Err(e) => Err(JsError::new(e.to_string().as_str())),
        }
    }

    #[wasm_bindgen(js_name = "uploadFileCreate")] // typescript convection is camelCase
    pub async fn upload_file_create(
        &mut self,
        blocks: Vec<ipcnodeapi::ipc_file_upload_create_request::IpcBlock>,
        root_cid: &str,
        size: i64,
    ) -> Result<ipcnodeapi::IpcFileUploadCreateResponse, JsError> {
        // TODO: this needs a blockchain transaction
        let response = self.sdk.upload_file_create(blocks, root_cid, size).await;
        match response {
            Ok(up_file_create_response) => Ok(up_file_create_response),
            Err(e) => Err(JsError::new(e.to_string().as_str())),
        }
    }

    // TODO: upload_file_block

    #[wasm_bindgen(js_name = "downloadFileCreate")] // typescript convection is camelCase
    pub async fn download_file_create(
        &mut self,
        address: &str,
        bucket_name: &str,
        file_name: &str,
    ) -> Result<ipcnodeapi::IpcFileDownloadCreateResponse, JsError> {
        let response = self
            .sdk
            .download_file_create(address, bucket_name, file_name)
            .await;
        match response {
            Ok(file_download_create_response) => Ok(file_download_create_response),
            Err(e) => Err(JsError::new(e.to_string().as_str())),
        }
    }

    #[wasm_bindgen(js_name = "downloadFile")] // typescript convection is camelCase
    pub async fn download_file(
        &mut self,
        file_download: ipcnodeapi::IpcFileDownloadCreateResponse,
    ) {
        let cid = file_download.blocks.first().unwrap().cid.clone();
        // TODO: A lot, recostruct the file, decrypt, etc..
        // FIXME: Although there's this call in the grpc, in akave code they dont use it to create a bucket?
        let response = self.sdk.download_file_block(&cid).await;
        let resp = match response {
            Ok(bucket_delete_response) => Ok(bucket_delete_response),
            Err(e) => Err(JsError::new(e.to_string().as_str())),
        };
    }

    // download_file_block
}
