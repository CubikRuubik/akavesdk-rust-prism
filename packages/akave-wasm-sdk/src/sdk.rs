pub mod ipcnodeapi {
    tonic::include_proto!("ipcnodeapi");
}
use alloy::rpc::types::request;
use prost::bytes;
use sha2::digest::typenum::uint;
#[cfg(not(target_arch = "wasm32"))]
use std::fs::File;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_file_reader::WebSysFile as File;
use web3::types::{TransactionReceipt, H256};

use ipcnodeapi::ipc_file_upload_create_request::IpcBlock;
use ipcnodeapi::{
    ipc_node_api_client::IpcNodeApiClient, IpcBucketListRequest, IpcBucketViewRequest,
    IpcFileListRequest, IpcFileViewRequest,
};
use ipcnodeapi::{
    ConnectionParamsRequest, ConnectionParamsResponse, IpcBucketCreateRequest,
    IpcBucketCreateResponse, IpcBucketDeleteRequest, IpcBucketDeleteResponse,
    IpcBucketListResponse, IpcBucketViewResponse, IpcFileBlockData, IpcFileDeleteRequest,
    IpcFileDeleteResponse, IpcFileDownloadBlockRequest, IpcFileDownloadCreateRequest,
    IpcFileListResponse, IpcFileUploadBlockResponse, IpcFileUploadCreateRequest,
    IpcFileViewResponse,
};

use crate::blockchain::provider::BlockchainProvider;
use crate::blockchain::response_types::BucketResponse;
use crate::utils::dag::DagBuilder;
use crate::utils::file_chunker::FileChunker;
use crate::utils::file_size::FileSize;

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
    connection_params: ConnectionParamsResponse,
    storage: BlockchainProvider,
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
            let mut client = IpcNodeApiClient::new(grpc_web_client);
            let connection_params = client
                .connection_params(ConnectionParamsRequest {})
                .await?
                .into_inner();
            let storage = BlockchainProvider::new(
                &connection_params.dial_uri,
                &connection_params.contract_address,
            );
            Ok(Self {
                client,
                connection_params,
                storage: storage.unwrap(),
            })
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let channel = Channel::from_shared(server_address.to_string())?
                .tls_config(ClientTlsConfig::new())?
                .connect()
                .await?;
            let mut client = IpcNodeApiClient::new(channel);
            let connection_params = client
                .connection_params(ConnectionParamsRequest {})
                .await?
                .into_inner();
            let storage = BlockchainProvider::new(
                &connection_params.dial_uri,
                &connection_params.contract_address,
            );

            Ok(Self {
                client,
                connection_params,
                storage: storage.unwrap(),
            })
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
    ) -> Result<BucketResponse, Box<dyn std::error::Error>> {
        self.storage.create_bucket(bucket_name.into()).await?;
        self.storage.get_bucket_by_name(bucket_name.into()).await
    }

    // Delete an existing bucket
    pub async fn delete_bucket(
        &mut self,
        address: &str,
        bucket_name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // TODO: Check if bucket is empty
        let bucket = self.view_bucket(address, bucket_name).await?;
        let bucket_id = hex::decode(bucket.id.clone())?;
        self.storage
            .delete_bucket(bucket_id, bucket_name.into())
            .await?;
        Ok(())
    }

    // Delete an existing file
    pub async fn delete_file(
        &mut self,
        bucket_id: Vec<u8>,
        transaction: Vec<u8>,
        file_name: &str,
    ) -> Result<IpcFileDeleteResponse, Box<dyn std::error::Error>> {
        let request = IpcFileDeleteRequest {
            bucket_id,
            transaction,
            name: file_name.to_string(),
        };

        Ok(self.client.file_delete(request).await?.into_inner())
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

        let root_cid = dag.root_cid()?;

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

    async fn download_file_block(
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

    pub async fn download_file(
        &mut self,
        address: &str,
        bucket_name: &str,
        file_name: &str,
        key: &str,
    ) -> Vec<u8> {
        let request = IpcFileDownloadCreateRequest {
            address: address.to_string(),
            bucket_name: bucket_name.to_string(),
            file_name: file_name.to_string(),
        };
        let file_download = self
            .client
            .file_download_create(request)
            .await
            .unwrap()
            .into_inner();

        let blocks_download = file_download.blocks;

        let mut file_data: Vec<u8> = vec![];
        let mut file_iter = blocks_download.iter();
        while let Some(block) = file_iter.next() {
            let mut a = self.download_file_block(&block.cid).await.unwrap();
            let message = a.message().await.unwrap().expect("Error receiving stream");
            let mut data = message.data;

            file_data.append(&mut data);
        }
        file_data
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use crate::{
        sdk::AkaveSDK,
        utils::encryption::{decrypt, derive_key, encrypt},
    };
    use pretty_assertions::{assert_eq, assert_ne, assert_str_eq};
    use std::future::Future; // crate for test-only use. Cannot be used in non-test code.

    const ADDRESS: &str = "0x7975eD6b732D1A4748516F66216EE703f4856759";
    const BUCKET_TO_TEST: &str = "TEST_BUCKET_v2";

    fn get_sdk() -> impl Future<Output = Result<AkaveSDK, Box<(dyn std::error::Error + 'static)>>> {
        AkaveSDK::new("http://connect.akave.ai:5500")
    }

    async fn test_create_bucket() {
        println!("Test 1: create bucket {}", BUCKET_TO_TEST);
        let mut sdk = get_sdk().await.unwrap();
        let bucket_resp = sdk.create_bucket(BUCKET_TO_TEST).await.unwrap();
        // println!("{}", bucket_resp.name);
        assert_eq!(bucket_resp.name, BUCKET_TO_TEST);
    }

    async fn test_list_buckets() {
        println!("Test 2: List all buckets");
        let mut sdk = get_sdk().await.unwrap();
        let buckets = sdk.list_buckets(ADDRESS).await.unwrap();
        let len = buckets.buckets.len();
        assert_ne!(len, 0, "there's buckets in this account");
    }

    async fn test_view_bucket() {
        println!("Test 3: Get {} bucket info", BUCKET_TO_TEST);
        let mut sdk = get_sdk().await.unwrap();
        let bucket = sdk.view_bucket(ADDRESS, BUCKET_TO_TEST).await.unwrap();
        assert_eq!(bucket.name, BUCKET_TO_TEST);
    }

    async fn test_delete_bucket() {
        println!("Test 4: Delete {} bucket", BUCKET_TO_TEST);
        let mut sdk = get_sdk().await.unwrap();
        let _ = sdk.delete_bucket(ADDRESS, BUCKET_TO_TEST).await;
        let bucket = sdk.view_bucket(ADDRESS, BUCKET_TO_TEST).await.unwrap();
        assert_ne!(
            bucket.name, BUCKET_TO_TEST,
            "There's still a bucket called {}",
            BUCKET_TO_TEST
        );
    }

    async fn test_encryption() {
        let data = "This is a phrase to test!! This is a phrase to test!! This is a phrase to test!! This is a phrase to test!! This is a phrase to test!! This is a phrase to test!!This is a phrase to test!!This is a phrase to test!!This is a phrase to test!!This is a phrase to test!! This is a phrase to test!! This is a phrase to test!! This is a phrase to test!!This is a phrase to test!!This is a phrase to test!!This is a phrase to test!! This is a phrase to test!!";
        let password = "TestPassword";
        let index: u64 = 1;
        let info = vec![BUCKET_TO_TEST, "file_name"].join("/");
        let key = derive_key(password.as_bytes(), info.as_bytes()).unwrap();
        let encrypted = encrypt(&key, data.as_bytes(), &index.to_be_bytes()).unwrap();
        let decrypted = decrypt(&key, &encrypted, &index.to_be_bytes()).unwrap();
        let decrypted_string = String::from_utf8(decrypted).unwrap();

        println!("DECRYPTED: {}", decrypted_string);
    }

    #[tokio::test]
    async fn test_all() {
        //test_create_bucket().await;
        //test_list_buckets().await;
        //test_view_bucket().await;
        // test_delete_bucket().await;
        test_encryption().await;
    }

    /* #[tokio::test]
    async fn test_list_buckets() {
        let mut sdk = get_sdk().await.unwrap();
        let buckets = sdk.list_buckets(ADDRESS).await.unwrap();
        let len = buckets.buckets.len();
        assert_ne!(len, 0, "there's buckets in this account");
    } */

    /*     #[tokio::test]
    async fn test_view_bucket() {
        let mut sdk = get_sdk().await.unwrap();
        let bucket = sdk.view_bucket(ADDRESS, BUCKET_TO_TEST).await.unwrap();

        assert_str_eq!(
            bucket.name,
            BUCKET_TO_TEST,
            "there's bucket and it's called {}",
            BUCKET_TO_TEST
        )
    }

    #[tokio::test]
    async fn test_simple_upload() {
        let sdk = get_sdk().await;
    } */
}
