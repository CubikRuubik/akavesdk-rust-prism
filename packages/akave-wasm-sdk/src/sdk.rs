pub mod ipcnodeapi {
    tonic::include_proto!("ipcnodeapi");
}
#[cfg(not(target_arch = "wasm32"))]
use std::fs::File;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_file_reader::WebSysFile as File;

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
    IpcFileUploadBlockResponse, IpcFileUploadCreateRequest, IpcFileUploadCreateResponse,
    IpcFileViewResponse,
};

use crate::utils::dag::DagBuilder;
use crate::utils::file_chunker::FileChunker;

/// Otherwise default to grpc.
#[cfg(not(target_arch = "wasm32"))]
use tonic::transport::{Channel, ClientTlsConfig};
use tonic::Streaming;
/// Conditionally use grpc-web is target arch is wasm32.
#[cfg(target_arch = "wasm32")]
use tonic_web_wasm_client::Client as GrpcWebClient;

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
    pub async fn upload_file_basic(
        &mut self,
        address: &str,
        bucket_name: &str,
        file: File,
    ) -> Result<IpcFileUploadBlockResponse, Box<dyn std::error::Error>> {
        let file_size = file.size() as i64;
        let chunker = FileChunker::new(file, None);

        // TODO: Improve dag construction mechanics
        let mut dag = DagBuilder::new(chunker);

        let mut blocks = vec![];
        let mut blocks_data = vec![];

        // TODO: This could be more compact (collect implementation?)

        while let Some((block, block_data)) = dag.next() {
            blocks.push(block);
            blocks_data.push(block_data);
        }

        let root_cid = dag.root_cid();

        // insert root block
        // TODO: find a better way to do this
        blocks.insert(
            0,
            IpcBlock {
                cid: root_cid.clone(),
                size: 0,
            },
        );

        let request = IpcFileUploadCreateRequest {
            blocks,
            root_cid: root_cid,
            size: file_size as i64, // TODO: funny, should double check
        };

        // Create the upload alloc
        let _ = self.client.file_upload_create(request).await?.into_inner();

        // TODO: check this is the correct way to stream a file
        let block_stream = futures::stream::iter(blocks_data);

        // TODO: update on the blockchain: Solidity -> function addFile(bytes cid, bytes32 bucketId, string name, uint256 size, bytes[] cids, uint256[] sizes) returns(bytes32, bytes32[])
        // wait for transaction
        // call self.client.FileUploadCreate with the blocks cids and sizes

        // TODO: should not return response, should check and return an SDK friendly value
        Ok(self
            .client
            .file_upload_block(block_stream)
            .await?
            .into_inner())
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
