pub mod ipcnodeapi {
    tonic::include_proto!("ipcnodeapi");
}

use alloy::hex;
use cid::{
    multibase::Base,
    multihash::{Code, MultihashDigest},
    Cid,
};
use ipcnodeapi::ipc_chunk::Block;
use prost_wkt_types::Timestamp;

#[cfg(not(target_arch = "wasm32"))]
use std::fs::File;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_file_reader::WebSysFile as File;
use web3::types::{TransactionReceipt, U256};

use ipcnodeapi::{
    ipc_node_api_client::IpcNodeApiClient, ConnectionParamsRequest, IpcBucketListRequest,
    IpcBucketListResponse, IpcBucketViewRequest, IpcBucketViewResponse, IpcChunk, IpcFileBlockData,
    IpcFileDeleteRequest, IpcFileDeleteResponse, IpcFileListRequest,
    IpcFileUploadChunkCreateRequest, IpcFileViewRequest, IpcFileViewResponse,
};

use crate::blockchain::response_types::BucketResponse;
use crate::utils::dag::{DagBuilder, FileBlockUpload};
use crate::utils::encryption::Encryption;
use crate::utils::splitter::Splitter;
use crate::{blockchain::provider::BlockchainProvider, utils::dag::DAG_PROTOBUF};

/// Otherwise default to grpc.
#[cfg(not(target_arch = "wasm32"))]
use tonic::transport::{Channel, ClientTlsConfig};

/// Conditionally use grpc-web is target arch is wasm32.
#[cfg(target_arch = "wasm32")]
use tonic_web_wasm_client::Client as GrpcWebClient;

const ENCRYPTION_OVERHEAD: usize = 32;
const BLOCK_SIZE: usize = (1.0 * 1e6) as usize;
const MIN_BUCKET_NAME_LENGTH: usize = 3;
const MIN_FILE_SIZE: usize = 127;
const MAX_BLOCKS_IN_CHUNK: usize = 32;

/// Represents the Akave SDK client
/// Akave Rust SDK should support both WASM (gRPC-Web) and native gRPC

#[cfg(target_arch = "wasm32")]
type ClientTransport = GrpcWebClient;

#[cfg(not(target_arch = "wasm32"))]
type ClientTransport = Channel;

struct IpcFileListItem {
    pub root_cid: String,
    pub name: String,
    pub encoded_size: i64,
    pub created_at: Timestamp,
}

struct IpcFileChunkUpload {
    pub index: usize,
    pub chunk_cid: Cid,
    pub actual_size: usize,
    pub raw_data_size: usize,
    pub proto_node_size: usize,
    pub blocks: Vec<FileBlockUpload>,
    pub bucket_id: [u8; 32],
    pub file_name: String,
}

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
                &connection_params.storage_address,
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
                &connection_params.storage_address,
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
    ) -> Result<Vec<IpcFileListItem>, Box<dyn std::error::Error>> {
        let request = IpcFileListRequest {
            bucket_name: bucket_name.to_string(),
            address: address.to_string(),
        };
        let files = self.client.file_list(request).await?.into_inner();
        Ok(files
            .list
            .iter()
            .map(|file| IpcFileListItem {
                root_cid: file.root_cid.clone(),
                created_at: file.created_at.unwrap(),
                encoded_size: file.encoded_size,
                name: file.name.clone(),
            })
            .collect())
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
        &mut self,
        bucket_id: Vec<u8>,
        file_name: &str,
    ) -> Result<TransactionReceipt, Box<dyn std::error::Error>> {
        self.storage
            .create_file(bucket_id, file_name.to_string())
            .await
    }

    pub async fn upload_file(
        &mut self,
        bucket_name: &str,
        file_name: &str,
        file: File,
        passwd: &str,
    ) -> Result<TransactionReceipt, Box<dyn std::error::Error>> {
        // GET BUCKET
        if bucket_name.is_empty() {
            return Err("Empty bucket name")?;
        }

        let bucket = self
            .storage
            .get_bucket_by_name(bucket_name.to_string())
            .await?;

        let resp = self.create_file_upload(bucket.id.to_vec(), file_name).await;

        if resp.is_ok() {
            println!("File created successfully");
        }
        // SPLIT FILE INTO 32MB AND ENCRYPT DATA
        let info = vec![bucket_name, file_name].join("/");
        let encryption = Encryption::new(passwd.as_bytes(), info.as_bytes())?;
        let chunk_size = (BLOCK_SIZE * MAX_BLOCKS_IN_CHUNK) as u64;
        let chunker = Splitter::new(file, chunk_size, Some(encryption));
        if chunker.size() == 0 {
            return Err("Empty file".into());
        }
        // ITERATE OVER 32MB CHUNKS
        let mut enum_blocks = chunker.into_iter().enumerate();

        let root_hasher = Code::Sha2_256;
        let mut root_hash = None;
        let mut file_size: usize = 0;

        while let Some((idx, Ok(block))) = enum_blocks.next() {
            // CREATE CHUNK UPLOAD
            let (chunk, _, ipc_chunk) = self
                .create_chunk(idx, block.to_vec(), bucket.id, file_name)
                .await?;
            // INCREMENT FILE SIZE
            file_size += chunk.actual_size;
            // ADD CHUNK TO DAG ROOT
            root_hash = Some(root_hasher.digest(&chunk.chunk_cid.to_bytes()));
            // UPLOAD CHUNK
            self.upload_chunk(chunk, bucket.id.to_vec(), file_name.to_string(), ipc_chunk)
                .await?;
        }
        // GENERATES DAG ROOT CID
        let root_cid = Cid::new_v1(DAG_PROTOBUF, root_hash.unwrap());
        // GET FILE METADATA FROM CONTRACT
        // TODO: let file_meta = self.storage.get_file_by_name(bucket.id, file_name);
        // COMMIT FILE TO CONTRACT
        let receipt = self
            .storage
            .commit_file(
                bucket.id,
                file_name.to_string(),
                U256::from(file_size),
                root_cid.to_bytes(),
            )
            .await?;
        // RETURN
        Ok(receipt) // TODO: Improve response
    }

    async fn create_chunk(
        &mut self,
        index: usize,
        data: Vec<u8>,
        bucket_id: [u8; 32],
        file_name: &str,
    ) -> Result<(IpcFileChunkUpload, TransactionReceipt, IpcChunk), Box<dyn std::error::Error>>
    {
        // BUILD A NEW DAG
        let block_size = BLOCK_SIZE;
        let size = data.len();
        let mut dag = DagBuilder::new(data, block_size);
        // GET CIDS AND SIZES FROM to_ipc_proto_chunk

        let mut blocks = vec![];
        let mut cids: Vec<[u8; 32]> = vec![];
        let mut sizes = vec![];
        let mut chunk_blocks = vec![];

        while let Some(block) = dag.next() {
            let block_cid = block.cid.to_bytes()[4..36]
                .to_vec()
                .try_into()
                .expect("Error formatting cid");
            chunk_blocks.push(Block {
                cid: block.cid.to_string(),
                size: block.data.len() as i64,
            });
            blocks.push(block);
            cids.push(block_cid);
            sizes.push(U256::from(size));
        }

        let chunk_cid = dag.root_cid()?;

        println!("{}", chunk_cid.to_bytes()[0]);
        println!("{}", chunk_cid.to_string());
        println!("{}", chunk_cid.to_string_of_base(Base::Base64)?);
        println!("{}", hex::encode(chunk_cid.to_bytes()));
        println!("{:#?}", chunk_cid.version());
        let ipc_chunk = IpcChunk {
            cid: chunk_cid.to_string(),
            index: index as i64,
            size: size as i64,
            blocks: chunk_blocks,
        };
        // CALL grpc FileUploadChunkCreate
        let chunk_create_request = IpcFileUploadChunkCreateRequest {
            chunk: Some(ipc_chunk.clone()),
            bucket_id: bucket_id.to_vec(),
            file_name: file_name.to_string(),
        };
        println!("failing here? 1");
        let chunk_create_response = self
            .client
            .file_upload_chunk_create(chunk_create_request)
            .await?
            .into_inner();
        println!("failing here? 2");
        // UPDATE DAG INFO WITH RESPONSE FROM GRPC
        chunk_create_response
            .blocks
            .iter()
            .enumerate()
            .for_each(|(idx, block)| {
                blocks[idx].node_address = block.node_address.clone();
                blocks[idx].node_id = block.node_id.clone();
                blocks[idx].permit = block.permit.clone();
            });
        // CALL CONTRACT AddFileChunk
        let receipt = self
            .storage
            .add_file_chunk(
                chunk_cid.to_bytes(),
                bucket_id,
                file_name.to_string(),
                size.into(),
                cids,
                sizes,
                index.into(),
            )
            .await?;
        // RETURN
        Ok((
            IpcFileChunkUpload {
                index,
                chunk_cid,
                actual_size: size,
                raw_data_size: size,
                proto_node_size: size,
                blocks,
                bucket_id,
                file_name: file_name.to_string(),
            },
            receipt,
            ipc_chunk,
        ))
    }

    async fn upload_chunk(
        &mut self,
        chunk: IpcFileChunkUpload,
        bucket_id: Vec<u8>,
        file_name: String,
        proto_chunk: IpcChunk,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let blocks = chunk.blocks.into_iter();

        let mut blocks_upload = vec![];
        let mut iter_blocks = blocks.into_iter().enumerate();
        while let Some((idx, block)) = iter_blocks.next() {
            let chunk = proto_chunk.clone();
            println!("{:#?}", block.cid.to_string());

            blocks_upload.push(IpcFileBlockData {
                bucket_id: bucket_id.clone(),
                data: block.data,
                cid: block.cid.to_string(),
                chunk: Some(chunk),
                file_name: file_name.clone(),
                index: idx as i64,
            });
        }

        let block_stream = futures::stream::iter(blocks_upload);
        let resp = self
            .client
            .file_upload_block(block_stream)
            .await?
            .into_inner();

        Ok(())
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use crate::sdk::AkaveIpcSDK;
    use pretty_assertions::{assert_eq, assert_ne};
    use std::fs::File; // crate for test-only use. Cannot be used in non-test code.

    const ADDRESS: &str = "0x7975eD6b732D1A4748516F66216EE703f4856759";
    const BUCKET_TO_TEST: &str = "TEST_BUCKET_v12";

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

    async fn test_upload_file() {
        println!("Test 5: upload a file to {}", BUCKET_TO_TEST);
        let mut sdk = get_sdk().await.unwrap();
        let file = File::open("foo.txt").unwrap();
        let _ = sdk
            .upload_file(BUCKET_TO_TEST, "foo.txt", file, "passwd")
            .await
            .unwrap();
    }

    async fn test_list_files() {
        println!("Test 6: List all files in a bucket");
        let mut sdk = get_sdk().await.unwrap();
        let files = sdk.list_files(ADDRESS, BUCKET_TO_TEST).await.unwrap();
        let len = files.len();
        assert_ne!(len, 0, "there's files in this account");
        files.into_iter().for_each(|file| {
            println!("{}", file.name);
            println!("{}", file.created_at);
        });
    }

    #[tokio::test]
    async fn test_all() {
        test_create_bucket().await;
        // test_list_buckets().await;
        // test_view_bucket().await;
        // test_delete_bucket().await;
        test_upload_file().await;
        test_list_files().await;
    }
}
