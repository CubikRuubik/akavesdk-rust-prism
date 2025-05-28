use std::io::Cursor;

use crate::blockchain::ipc_types::BucketResponse;
use crate::panic_handler::initialize_panic_handler;
use crate::sdk::{AkaveSDK, AkaveSDKBuilder};
use crate::sdk_types::{
    BucketListResponse, BucketViewResponse, FileListResponse, FileViewResponse,
};
use crate::{log_debug, log_error, log_info};

use js_sys::Uint8Array;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct AkaveWebSDK {
    sdk: AkaveSDK,
}

#[wasm_bindgen]
pub struct AkaveWebSDKBuilder {
    inner_builder: AkaveSDKBuilder,
}

#[wasm_bindgen]
impl AkaveWebSDKBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(server_address: &str) -> Self {
        initialize_panic_handler();
        Self {
            inner_builder: AkaveSDKBuilder::new(server_address),
        }
    }

    #[wasm_bindgen(js_name = "withErasureCoding")]
    pub fn with_erasure_coding(mut self, data_blocks: usize, parity_blocks: usize) -> Self {
        self.inner_builder = self
            .inner_builder
            .with_erasure_coding(data_blocks, parity_blocks);
        self
    }

    #[wasm_bindgen(js_name = "withDefaultEncryption")]
    pub fn with_default_encryption(mut self, encryption_key: &str) -> Self {
        self.inner_builder = self.inner_builder.with_default_encryption(encryption_key);
        self
    }

    #[wasm_bindgen(js_name = "withBlockSize")]
    pub fn with_block_size(mut self, block_size: usize) -> Self {
        self.inner_builder = self.inner_builder.with_block_size(block_size);
        self
    }

    #[wasm_bindgen(js_name = "withMinBucketLength")]
    pub fn with_min_bucket_length(mut self, min_bucket_name_length: usize) -> Self {
        self.inner_builder = self
            .inner_builder
            .with_min_bucket_length(min_bucket_name_length);
        self
    }

    #[wasm_bindgen(js_name = "withMaxBlocksInChunk")]
    pub fn with_max_blocks_in_chunk(mut self, max_blocks_in_chunk: usize) -> Self {
        self.inner_builder = self
            .inner_builder
            .with_max_blocks_in_chunk(max_blocks_in_chunk);
        self
    }

    #[wasm_bindgen(js_name = "withBlockPartSize")]
    pub fn with_block_part_size(mut self, block_part_size: usize) -> Self {
        self.inner_builder = self.inner_builder.with_block_part_size(block_part_size);
        self
    }

    #[wasm_bindgen(js_name = "build")]
    pub async fn build(self) -> Result<AkaveWebSDK, JsError> {
        log_info!("Building AkaveWebSDK with configured options");

        match self.inner_builder.build().await {
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
}

#[wasm_bindgen]
impl AkaveWebSDK {
    pub async fn new() -> Result<AkaveWebSDK, JsError> {
        log_info!("Initializing AkaveWebSDK with default settings");
        initialize_panic_handler();
        AkaveWebSDKBuilder::new("http://localhost:3000")
            .build()
            .await
    }

    #[wasm_bindgen(constructor)]
    pub async fn new_with_endpoint(endpoint: &str) -> Result<AkaveWebSDK, JsError> {
        log_info!("Initializing AkaveWebSDK with endpoint: {}", endpoint);
        initialize_panic_handler();
        AkaveWebSDKBuilder::new(endpoint).build().await
    }

    #[wasm_bindgen(js_name = "listBuckets")]
    pub async fn list_buckets(&mut self, address: &str) -> Result<BucketListResponse, JsError> {
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
    ) -> Result<BucketViewResponse, JsError> {
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
    ) -> Result<FileViewResponse, JsError> {
        log_debug!(
            "Viewing file info: {} in bucket: {} for address: {}",
            file_name,
            bucket_name,
            address
        );
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
    ) -> Result<FileListResponse, JsError> {
        log_debug!(
            "Listing files in bucket: {} for address: {}",
            bucket_name,
            address
        );
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
    pub async fn create_bucket(&mut self, bucket_name: &str) -> Result<BucketResponse, JsError> {
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
    pub async fn delete_bucket(&mut self, address: &str, bucket_name: &str) -> Result<(), JsError> {
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
        log_debug!(
            "Deleting file: {} from bucket: {} for address: {}",
            file_name,
            bucket_name,
            address
        );
        let response = self.sdk.delete_file(address, bucket_name, file_name).await;
        match response {
            Ok(_) => {
                log_info!(
                    "Successfully deleted file: {} from bucket: {}",
                    file_name,
                    bucket_name
                );
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
        data: &[u8],
    ) -> Result<(), JsError> {
        log_debug!("Uploading file: {} to bucket: {}", file_name, bucket_name);

        let mut reader = Cursor::new(data.to_vec());

        let response = self
            .sdk
            .upload_file(bucket_name, file_name, &mut reader, None)
            .await;

        match response {
            Ok(_) => {
                log_info!(
                    "Successfully uploaded file: {} to bucket: {}",
                    file_name,
                    bucket_name
                );
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
    ) -> Result<Uint8Array, JsError> {
        log_debug!(
            "Downloading file: {} from bucket: {} for address: {}",
            file_name,
            bucket_name,
            address
        );

        let mut data = Vec::new();

        data = self
            .sdk
            .download_file(address, bucket_name, file_name, None, data)
            .await
            .map_err(|e| JsError::new(&format!("Download failed: {:?}", e)))?
            .to_vec();

        // Return as Uint8Array to JS
        Ok(js_sys::Uint8Array::from(&data[..]))
    }
}
