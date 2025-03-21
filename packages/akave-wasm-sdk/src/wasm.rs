use crate::blockchain::ipc_types::BucketResponse;
use crate::panic_handler::initialize_panic_handler;
use crate::sdk::ipcnodeapi;
use crate::sdk::AkaveIpcSDK as AkaveSDK;
use crate::sdk_types::IpcFileList;
use crate::{log_info, log_error, log_debug};

use wasm_bindgen::prelude::*;
use wasm_bindgen_file_reader::WebSysFile;
use web_sys::File;

#[wasm_bindgen]
pub(crate) struct AkaveWebSDK {
    sdk: AkaveSDK,
}

#[wasm_bindgen]
impl AkaveWebSDK {
    pub async fn new() -> Result<AkaveWebSDK, JsError> {
        log_info!("Initializing AkaveWebSDK");
        initialize_panic_handler();
        Self::new_with_endpoint("http://localhost:3000").await
    }

    #[wasm_bindgen(constructor)]
    pub async fn new_with_endpoint(endpoint: &str) -> Result<AkaveWebSDK, JsError> {
        log_info!("Initializing AkaveWebSDK with endpoint: {}", endpoint);
        initialize_panic_handler();

        match AkaveSDK::new(endpoint).await {
            Ok(sdk) => {
                log_info!("AkaveWebSDK initialized successfully");
                Ok(AkaveWebSDK { sdk })
            }
            Err(e) => {
                log_error!("Failed to initialize SDK: {}", e);
                Err(JsError::new(&format!("Failed to initialize SDK: {}", e)))
            }
        }
    }

    #[wasm_bindgen(js_name = "listBuckets")]
    pub async fn list_buckets(
        &mut self,
        address: &str,
    ) -> Result<ipcnodeapi::IpcBucketListResponse, JsError> {
        log_debug!("Listing buckets for address: {}", address);
        let response = self.sdk.list_buckets(address).await;
        match response {
            Ok(bucket_list_response) => {
                log_info!("Successfully retrieved bucket list");
                Ok(bucket_list_response)
            }
            Err(e) => {
                log_error!("Failed to list buckets: {}", e);
                Err(JsError::new(e.to_string().as_str()))
            }
        }
    }

    #[wasm_bindgen(js_name = "viewBucket")]
    pub async fn view_bucket(
        &mut self,
        address: &str,
        bucket_name: &str,
    ) -> Result<ipcnodeapi::IpcBucketViewResponse, JsError> {
        log_debug!("Viewing bucket: {} for address: {}", bucket_name, address);
        let response = self.sdk.view_bucket(address, bucket_name).await;
        match response {
            Ok(bucket_view_response) => {
                log_info!("Successfully retrieved bucket details");
                Ok(bucket_view_response)
            }
            Err(e) => {
                log_error!("Failed to view bucket: {}", e);
                Err(JsError::new(e.to_string().as_str()))
            }
        }
    }

    #[wasm_bindgen(js_name = "viewFileInfo")]
    pub async fn view_file_info(
        &mut self,
        address: &str,
        bucket_name: &str,
        file_name: &str,
    ) -> Result<ipcnodeapi::IpcFileViewResponse, JsError> {
        log_debug!("Viewing file info: {} in bucket: {} for address: {}", file_name, bucket_name, address);
        let response = self
            .sdk
            .view_file_info(address, bucket_name, file_name)
            .await;

        match response {
            Ok(file_view_response) => {
                log_info!("Successfully retrieved file details");
                Ok(file_view_response)
            }
            Err(e) => {
                log_error!("Failed to view file info: {}", e);
                Err(JsError::new(e.to_string().as_str()))
            }
        }
    }

    #[wasm_bindgen(js_name = "listFiles")]
    pub async fn list_files(
        &mut self,
        address: &str,
        bucket_name: &str,
    ) -> Result<IpcFileList, JsError> {
        log_debug!("Listing files in bucket: {} for address: {}", bucket_name, address);
        let response = self.sdk.list_files(address, bucket_name).await;

        match response {
            Ok(file_list_response) => {
                log_info!("Successfully retrieved file list");
                Ok(file_list_response)
            }
            Err(e) => {
                log_error!("Failed to list files: {}", e);
                Err(JsError::new(e.to_string().as_str()))
            }
        }
    }

    #[wasm_bindgen(js_name = "createBucket")]
    pub async fn create_bucket(
        &mut self,
        bucket_name: &str,
    ) -> Result<BucketResponse, JsError> {
        log_debug!("Creating bucket: {}", bucket_name);
        let response = self.sdk.create_bucket(bucket_name).await;
        match response {
            Ok(bucket_create_response) => {
                log_info!("Successfully created bucket: {}", bucket_name);
                Ok(bucket_create_response)
            }
            Err(e) => {
                log_error!("Failed to create bucket: {}", e);
                Err(JsError::new(e.to_string().as_str()))
            }
        }
    }

    #[wasm_bindgen(js_name = "deleteBucket")]
    pub async fn delete_bucket(
        &mut self,
        address: &str,
        bucket_name: &str,
    ) -> Result<(), JsError> {
        log_debug!("Deleting bucket: {} for address: {}", bucket_name, address);
        let response = self.sdk.delete_bucket(address, bucket_name).await;
        match response {
            Ok(_) => {
                log_info!("Successfully deleted bucket: {}", bucket_name);
                Ok(())
            }
            Err(e) => {
                log_error!("Failed to delete bucket: {}", e);
                Err(JsError::new(e.to_string().as_str()))
            }
        }
    }

    #[wasm_bindgen(js_name = "deleteFile")]
    pub async fn delete_file(
        &mut self,
        address: &str,
        bucket_name: &str,
        file_name: &str,
    ) -> Result<(), JsError> {
        log_debug!("Deleting file: {} from bucket: {} for address: {}", file_name, bucket_name, address);
        let response = self.sdk.delete_file(address, bucket_name, file_name).await;
        match response {
            Ok(_) => {
                log_info!("Successfully deleted file: {} from bucket: {}", file_name, bucket_name);
                Ok(())
            }
            Err(e) => {
                log_error!("Failed to delete file: {}", e);
                Err(JsError::new(e.to_string().as_str()))
            }
        }
    }

    #[wasm_bindgen(js_name = "uploadFile")]
    pub async fn upload_file(
        &mut self,
        bucket_name: &str,
        file_name: &str,
        file: File,
    ) -> Result<(), JsError> {
        log_debug!("Uploading file: {} to bucket: {}", file_name, bucket_name);
        let response = self.sdk.upload_file(bucket_name, file_name, WebSysFile::new(file), None).await;
        match response {
            Ok(_) => {
                log_info!("Successfully uploaded file: {} to bucket: {}", file_name, bucket_name);
                Ok(())
            }
            Err(e) => {
                log_error!("Failed to upload file: {}", e);
                Err(JsError::new(e.to_string().as_str()))
            }
        }
    }

    #[wasm_bindgen(js_name = "downloadFile")]
    pub async fn download_file(
        &mut self,
        address: &str,
        bucket_name: &str,
        file_name: &str,
        destination_path: &str,
    ) -> Result<(), JsError> {
        log_debug!("Downloading file: {} from bucket: {} for address: {}", file_name, bucket_name, address);
        let response = self
            .sdk
            .download_file(address, bucket_name, file_name, None, destination_path)
            .await;
        match response {
            Ok(_) => {
                log_info!("Successfully downloaded file: {} from bucket: {}", file_name, bucket_name);
                Ok(())
            }
            Err(e) => {
                log_error!("Failed to download file: {}", e);
                Err(JsError::new(e.to_string().as_str()))
            }
        }
    }
}
