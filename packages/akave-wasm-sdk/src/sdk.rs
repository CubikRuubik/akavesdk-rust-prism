pub mod ipcnodeapi {
    tonic::include_proto!("ipcnodeapi");
}

use ipcnodeapi::ipc_chunk::Block;
use prost::Message;
#[cfg(not(target_arch = "wasm32"))]
use std::fs::File;
use std::time::Duration;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_file_reader::WebSysFile as File;

use ipcnodeapi::{
    ipc_file_download_create_response::Chunk, ipc_node_api_client::IpcNodeApiClient,
    ConnectionParamsRequest, IpcBucketListRequest, IpcBucketListResponse, IpcBucketViewRequest,
    IpcBucketViewResponse, IpcChunk, IpcFileBlockData, IpcFileDeleteRequest, IpcFileDeleteResponse,
    IpcFileDownloadBlockRequest, IpcFileDownloadCreateRequest, IpcFileListRequest,
    IpcFileListResponse, IpcFileUploadChunkCreateRequest, IpcFileViewRequest, IpcFileViewResponse,
};

use crate::blockchain::provider::BlockchainProvider;
use crate::blockchain::response_types::BucketResponse;
use crate::utils::dag::{DagBuilder, FileBlockUpload};
use crate::utils::encryption::Encryption;
use crate::utils::splitter::Splitter;

/// Otherwise default to grpc.
#[cfg(not(target_arch = "wasm32"))]
use tonic::transport::{Channel, ClientTlsConfig};
use tonic::Streaming;
/// Conditionally use grpc-web is target arch is wasm32.
#[cfg(target_arch = "wasm32")]
use tonic_web_wasm_client::Client as GrpcWebClient;

const ENCRYPTION_OVERHEAD: usize = 32;
const BLOCK_SIZE: usize = (1.0 * 1e6) as usize;
const MIN_BUCKET_NAME_LENGTH: usize = 3;
const MIN_FILE_SIZE: usize = 127;
const MAX_BLOCK_SIZE: usize = 32;

/// Represents the Akave SDK client
/// Akave Rust SDK should support both WASM (gRPC-Web) and native gRPC

#[cfg(target_arch = "wasm32")]
type ClientTransport = GrpcWebClient;

#[cfg(not(target_arch = "wasm32"))]
type ClientTransport = Channel;

pub struct AkaveIpcSDK {
    client: IpcNodeApiClient<ClientTransport>,
    storage: BlockchainProvider,
}

impl AkaveIpcSDK {
    /// Creates a new AkaveSDK instance
    pub async fn new(server_address: &str) -> Result<Self, Box<dyn std::error::Error>> {
        #[cfg(target_arch = "wasm32")]
        {
            let grpc_web_client = ClientTransport::new(server_address.into());
            let mut client = IpcNodeApiClient::new(grpc_web_client);
            let connection_params = client
                .connection_params(ConnectionParamsRequest {})
                .await?
                .into_inner();
            let storage = BlockchainProvider::new(
                &connection_params.dial_uri,
                &connection_params.contract_address,
                None,
                None,
            );
            Ok(Self {
                client,
                storage: storage.unwrap(),
            })
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let tls_config = ClientTlsConfig::new().with_native_roots();
            let channel = Channel::from_shared(server_address.to_string())?
                .tls_config(tls_config)?
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
                None,
                None,
            );

            Ok(Self {
                client,
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
            name: bucket_name.to_string(),
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
        if bucket_name.len() < MIN_BUCKET_NAME_LENGTH {
            return Err(format!(
                "Bucket name must have at least {} characters",
                MIN_BUCKET_NAME_LENGTH
            ))?;
        }
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
        transaction: Vec<u8>,
        bucket_name: &str,
        file_name: &str,
    ) -> Result<IpcFileDeleteResponse, Box<dyn std::error::Error>> {
        let request = IpcFileDeleteRequest {
            transaction,
            bucket_name: bucket_name.as_bytes().to_vec(),
            name: file_name.to_string(),
        };

        Ok(self.client.file_delete(request).await?.into_inner())
    }

    async fn create_file_upload(
        &self,
        bucket_id: Vec<u8>,
        file_name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let tx = self
            .storage
            .create_file(bucket_id, file_name.to_string())
            .await?;

        /* if tx.is_none() {
            return Err("Transaction timeout")?;
        } */
        Ok(())
    }

    pub async fn upload_file(
        &mut self,
        bucket_name: &str,
        file_name: &str,
        file: File,
        passwd: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if !bucket_name.is_empty() {
            return Err("Empty bucket name")?;
        }

        let bucket = self
            .storage
            .get_bucket_by_name(bucket_name.to_string())
            .await?;

        let info = vec![bucket_name, file_name].join("/");
        let encryption = Encryption::new(passwd.as_bytes(), info.as_bytes())?;
        let key_size = encryption.len();

        let chunker = Splitter::new(file, BLOCK_SIZE as u64, Some(encryption));
        let mut dag = DagBuilder::new(chunker);

        let mut file_upload_block = vec![];
        let mut enum_blocks = dag.into_iter().enumerate();
        while let Some((idx, block)) = enum_blocks.next() {
            let size = block.data.len();

            let block_size = BLOCK_SIZE; // Update this if erasure code: data.len() / (erasure_code.data_blocks + erasure_code.parity_blocks)
                                         // TODO: apply erasure code here: erasureCode.Encode(data)

            let req = IpcFileUploadChunkCreateRequest {
                chunk: Some(IpcChunk {
                    cid: block.cid.clone(),
                    index: idx as i64,
                    size: size as i64,
                    blocks: vec![Block {
                        cid: block.cid.clone(),
                        size: size as i64,
                    }],
                }),
                bucket_id: bucket.id.to_vec(),
                file_name: file_name.to_string(),
            };

            let resp = self
                .client
                .file_upload_chunk_create(req)
                .await?
                .into_inner();

            if resp.blocks.len() != 1 {
                return Err(format!(
                    "received unexpected amount of blocks {}, expected {}",
                    resp.blocks.len(),
                    1
                ))?;
            }

            let mut resp_blocks = resp.blocks.iter().enumerate();
            while let Some((index, uploaded_block)) = resp_blocks.next() {
                if block.cid != uploaded_block.cid {
                    return Err(format!("block CID mismatch at position {}", index))?;
                }
                file_upload_block.push(uploaded_block.clone());
            }

            /* let resp = self
            .storage
            .add_file_chunk(
                root_cid.encode_to_vec(),
                bucket.id.to_vec(),
                file_name.to_string(),
                size as i64,
                vec![block.cid.clone().encode_to_vec()],
                vec![size as i64],
                idx as i64,
            )
            .await?; */
        }

        todo!()
    }

    /*     pub async fn old_upload_file(
        &mut self,
        address: &str,
        bucket_name: &str,
        file_name: &str,
        file: File,
    ) -> Result<IpcFileUploadBlockResponse, Box<dyn std::error::Error>> {
        let file_size = file.size() as i64;
        let chunker = FileChunker::new(file, None);

        let bucket_info = self.view_bucket(address, bucket_name).await?;

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
            IpcFileBlockData {
                cid: root_cid.clone(),
                data: [].to_vec(),
                index: 0,
                chunk: None,
                bucket_id: bucket_info.id.as_bytes().to_vec(), // not tu sure about this
                file_name: file_name.to_string(),
            },
        );

        /* let request = IpcFileUploadChunkCreateRequest {
            chunk: todo!(),
            bucket_id: todo!(),
            file_name: todo!(),
        }; */

        // Create the upload alloc
        /* let _ = self
        .client
        .file_upload_chunk_create(request)
        .await?
        .into_inner(); */

        // TODO: check this is the correct way to stream a file

        // let block_stream = futures::stream::iter(blocks_data);

        // TODO: update on the blockchain: Solidity -> function addFile(bytes cid, bytes32 bucketId, string name, uint256 size, bytes[] cids, uint256[] sizes) returns(bytes32, bytes32[])
        // wait for transaction
        // call self.client.FileUploadCreate with the blocks cids and sizes

        // TODO: should not return response, should check and return an SDK friendly value
        /* Ok(self
        .client
        .file_upload_block(block_stream)
        .await?
        .into_inner()) */
        todo!()
    } */

    async fn download_file_block(
        &mut self,
        block_cid: String,
        chunk: &Chunk,
    ) -> Result<Streaming<IpcFileBlockData>, Box<dyn std::error::Error>> {
        let request = IpcFileDownloadBlockRequest {
            block_cid,
            chunk_cid: chunk.cid.clone(),
            chunk_index: todo!(),
            block_index: todo!(),
            bucket_name: todo!(),
            file_name: todo!(),
            address: todo!(),
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
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
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

        let blocks_download = file_download.chunks;

        let mut file_data: Vec<u8> = vec![];
        let mut file_iter = blocks_download.iter();
        let block_cid = blocks_download.first().unwrap().cid.clone();
        while let Some(block) = file_iter.next() {
            let mut stream = self
                .download_file_block(block_cid.clone(), &block)
                .await
                .unwrap();
            let message = stream
                .message()
                .await
                .unwrap()
                .expect("Error receiving stream");
            let mut data = message.data;

            file_data.append(&mut data);
        }
        Ok(file_data)
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use crate::sdk::AkaveIpcSDK;
    use pretty_assertions::{assert_eq, assert_ne, assert_str_eq};
    use std::future::Future; // crate for test-only use. Cannot be used in non-test code.

    const ADDRESS: &str = "0x7975eD6b732D1A4748516F66216EE703f4856759";
    const BUCKET_TO_TEST: &str = "TEST_BUCKET_v5";

    async fn get_sdk() -> Result<AkaveIpcSDK, Box<(dyn std::error::Error + 'static)>> {
        AkaveIpcSDK::new("http://connect.akave.ai:5500").await
    }

    async fn test_create_bucket() {
        println!("Test 1: create bucket {}", BUCKET_TO_TEST);
        let mut sdk = get_sdk().await.unwrap();
        let bucket_resp = sdk.create_bucket(BUCKET_TO_TEST).await.unwrap();
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

    #[tokio::test]
    async fn test_all() {
        test_create_bucket().await;
        // test_list_buckets().await;
        // test_view_bucket().await;
        // test_delete_bucket().await;
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
