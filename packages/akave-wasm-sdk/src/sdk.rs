pub mod ipcnodeapi {
    tonic::include_proto!("ipcnodeapi");
}

use ipcnodeapi::ipc_file_upload_create_request::IpcBlock;
use ipcnodeapi::{
    ipc_node_api_client::IpcNodeApiClient, IpcBucketListRequest, IpcBucketViewRequest,
    IpcFileListRequest, IpcFileViewRequest,
};
use ipcnodeapi::{
    IpcBucketCreateRequest, IpcBucketCreateResponse, IpcBucketDeleteRequest,
    IpcBucketDeleteResponse, IpcBucketListResponse, IpcBucketViewResponse, IpcFileBlockData,
    IpcFileDeleteRequest, IpcFileDeleteResponse, IpcFileDownloadBlockRequest,
    IpcFileDownloadCreateRequest, IpcFileDownloadCreateResponse, IpcFileListResponse,
    IpcFileUploadCreateRequest, IpcFileUploadCreateResponse, IpcFileViewResponse, IpcFileUploadBlockResponse,
};

/// Otherwise default to grpc.
#[cfg(not(target_arch = "wasm32"))]
use tonic::transport::{Channel, ClientTlsConfig};
use tonic::Streaming;
/// Conditionally use grpc-web is target arch is wasm32.
#[cfg(target_arch = "wasm32")]
use tonic_web_wasm_client::Client as GrpcWebClient;
use web3::futures;

use crate::utils::{self, dag};
use crate::utils::file_reader::FileReader;

/// Represents the Akave SDK client
/// Akave Rust SDK should support both WASM (gRPC-Web) and native gRPC
pub struct AkaveSDK {
    client: IpcNodeApiClient<ClientTransport>,
}

#[cfg(target_arch = "wasm32")]
type ClientTransport = GrpcWebClient;

#[cfg(not(target_arch = "wasm32"))]
type ClientTransport = Channel;

impl AkaveSDK {
    /// Creates a new AkaveSDK instance
    pub async fn new(server_address: &str) -> Result<Self, Box<dyn std::error::Error>> {
        #[cfg(target_arch = "wasm32")]
        {
            let grpc_web_client = GrpcWebClient::new(server_address.into());
            let client = IpcNodeApiClient::new(grpc_web_client);
            Ok(Self { client })
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let channel = Channel::from_shared(server_address.to_string())?
                .tls_config(ClientTlsConfig::new())?
                .connect()
                .await?;
            let client = IpcNodeApiClient::new(channel);
            Ok(Self { client })
        }
    }

    /// List all buckets
    pub async fn list_buckets(
        &mut self,
        address: &str,
    ) -> Result<IpcBucketListResponse, Box<dyn std::error::Error>> {
        let request = IpcBucketListRequest {
            address: address.to_string(),
        };
        Ok(self.client.bucket_list(request).await?.into_inner())
    }

    /// View a bucket
    pub async fn view_bucket(
        &mut self,
        address: &str,
        bucket_name: &str,
    ) -> Result<IpcBucketViewResponse, Box<dyn std::error::Error>> {
        let request = IpcBucketViewRequest {
            bucket_name: bucket_name.to_string(),
            address: address.to_string(),
        };
        Ok(self.client.bucket_view(request).await?.into_inner())
    }

    /// List files in a bucket
    pub async fn list_files(
        &mut self,
        address: &str,
        bucket_name: &str,
    ) -> Result<IpcFileListResponse, Box<dyn std::error::Error>> {
        let request = IpcFileListRequest {
            bucket_name: bucket_name.to_string(),
            address: address.to_string(),
        };
        Ok(self.client.file_list(request).await?.into_inner())
    }

    /// View file information
    pub async fn view_file_info(
        &mut self,
        address: &str,
        bucket_name: &str,
        file_name: &str,
    ) -> Result<IpcFileViewResponse, Box<dyn std::error::Error>> {
        let request = IpcFileViewRequest {
            bucket_name: bucket_name.to_string(),
            file_name: file_name.to_string(),
            address: address.to_string(),
        };
        Ok(self.client.file_view(request).await?.into_inner())
    }

    // Create a new bucket
    pub async fn create_bucket(
        &mut self,
        bucket_name: &str,
    ) -> Result<IpcBucketCreateResponse, Box<dyn std::error::Error>> {
        let request = IpcBucketCreateRequest {
            name: bucket_name.to_string(),
        };

        Ok(self.client.bucket_create(request).await?.into_inner())
    }

    // Delete an existing bucket
    pub async fn delete_bucket(
        &mut self,
    ) -> Result<IpcBucketDeleteResponse, Box<dyn std::error::Error>> {
        // TODO: Check if bucket is empty
        let request = IpcBucketDeleteRequest {}; // TODO: Something's missing here

        Ok(self.client.bucket_delete(request).await?.into_inner())
    }

    // Delete an existing file
    pub async fn delete_file(
        &mut self,
        bucket_id: Vec<u8>,
        transaction: Vec<u8>,
        file_name: &str,
    ) -> Result<IpcFileDeleteResponse, Box<dyn std::error::Error>> {
        let request = IpcFileDeleteRequest {
            bucket_id: bucket_id,
            transaction,
            name: file_name.to_string(),
        };

        Ok(self.client.file_delete(request).await?.into_inner())
    }

    pub async fn upload_file_create(
        &mut self,
        blocks: Vec<IpcBlock>,
        root_cid: &str,
        size: i64,
    ) -> Result<IpcFileUploadCreateResponse, Box<dyn std::error::Error>> {
        // FIXME: This method should receive the file,
        // break it down and create an object ready to be uploaded
        // upload_file_create(bucket_name, file).
        // encrypt the file
        // split the file (dag? merkel?)
        // Call the grpc (IpcFileUploadCreateRequest)
        // Prepare a FileUpload object with all of this to be
        // used in a (to be made) Upload function

        let request = IpcFileUploadCreateRequest {
            blocks,
            root_cid: root_cid.to_string(),
            size,
        };

        Ok(self.client.file_upload_create(request).await?.into_inner())
    }


    // TODO: this is the most vanilla version of file upload
    //          USE WITH CAUTION
    pub async fn uploadFileBasic(
        &mut self,
        file_path: &str,
    ) -> Result<IpcFileUploadBlockResponse, Box<dyn std::error::Error>> {

        let file_reader = utils::file_reader::create_reader();

        // TODO: enable stream reads
        let file_blob: Vec<u8> = file_reader.read_file(file_path).await?;

        // TODO: Improve dag construction mechanics
        let (dag, root_cid) = utils::dag::DagBuilder::create_dag(&file_blob)?;

        // TODO: Improve conversion between dag//IpcBlock//IpcBlockData
        let blocks = utils::dag::DagBuilder::to_ipc_blocks(&dag);
    
        let request = IpcFileUploadCreateRequest {
            blocks,
            root_cid: root_cid.to_string(),
            size: file_blob.len() as i64, // TODO: funny, should double check
        };

        // Create the upload alloc
        let _ = self.client.file_upload_create(request).await?.into_inner();

        // TODO: a block is copied 3 times in an upload, fix
        let block_data = utils::dag::DagBuilder::to_ipc_block_data(&dag);

        // TODO: check this is the correct way to stream a file
        let block_stream = futures::stream::iter(block_data);
        
        // TODO: should not return response, should check and return an SDK friendly value
        Ok(self.client.file_upload_block(block_stream).await?.into_inner())
    }

    async fn upload_file_block(&mut self, cid: &str, data: Vec<u8>) {
        // FIXME: To be used in the Upload function mentioned in upload_file_create
        let request = IpcFileBlockData {
            cid: cid.to_string(),
            data,
        };

        // FIXME: How to send streams?
    }

    pub async fn download_file_create(
        &mut self,
        address: &str,
        bucket_name: &str,
        file_name: &str,
    ) -> Result<IpcFileDownloadCreateResponse, Box<dyn std::error::Error>> {
        let request = IpcFileDownloadCreateRequest {
            address: address.to_string(),
            bucket_name: bucket_name.to_string(),
            file_name: file_name.to_string(),
        };

        Ok(self
            .client
            .file_download_create(request)
            .await?
            .into_inner())
    }

    pub async fn download_file_block(
        &mut self,
        block_cid: &str,
    ) -> Result<Streaming<IpcFileBlockData>, Box<dyn std::error::Error>> {
        let request = IpcFileDownloadBlockRequest {
            block_cid: block_cid.to_string(),
        };

        Ok(self
            .client
            .file_download_block(request)
            .await
            .unwrap()
            .into_inner())
        // TODO: build the file from this stream
    }
}
