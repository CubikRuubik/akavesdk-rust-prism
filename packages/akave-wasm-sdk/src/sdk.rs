pub mod ipcnodeapi {
    tonic::include_proto!("ipcnodeapi");
}

use ipcnodeapi::ipc_chunk::Block;
use sha2::{Digest, Sha256};

#[cfg(not(target_arch = "wasm32"))]
use std::fs::File;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_file_reader::WebSysFile as File;
use web3::types::TransactionReceipt;

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
const MAX_BLOCKS_IN_CHUNK: usize = 32;

/// Represents the Akave SDK client
/// Akave Rust SDK should support both WASM (gRPC-Web) and native gRPC

#[cfg(target_arch = "wasm32")]
type ClientTransport = GrpcWebClient;

#[cfg(not(target_arch = "wasm32"))]
type ClientTransport = Channel;

struct IpcFileChunkUpload {
    pub index: usize,
    pub chunk_cid: String,
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
    ) -> Result<(), Box<dyn std::error::Error>> {
        if bucket_name.is_empty() {
            return Err("Empty bucket name")?;
        }

        let bucket = self
            .storage
            .get_bucket_by_name(bucket_name.to_string())
            .await?;

        self.create_file_upload(bucket.id.to_vec(), file_name)
            .await?;

        let info = vec![bucket_name, file_name].join("/");
        let encryption = Encryption::new(passwd.as_bytes(), info.as_bytes())?;

        // TODO: if erasure code this value is different
        let chunk_size = (BLOCK_SIZE * MAX_BLOCKS_IN_CHUNK) as u64;

        let chunker = Splitter::new(file, chunk_size, Some(encryption));

        if chunker.size() == 0 {
            return Err("Empty file".into());
        }

        let mut enum_blocks = chunker.into_iter().enumerate();

        let mut root_hasher = Sha256::new();
        let mut file_size: usize = 0;

        while let Some((idx, Ok(block))) = enum_blocks.next() {
            let (chunk_upload, receipt, proto_chunk) = self
                .create_chunk_upload(idx, block.to_vec(), bucket.id, file_name)
                .await?;

            root_hasher.update(chunk_upload.chunk_cid.clone());

            file_size += chunk_upload.actual_size;

            self.upload_chunk(
                chunk_upload,
                bucket.id.to_vec(),
                file_name.to_string(),
                proto_chunk,
            )
            .await?;
        }

        let root_cid = hex::encode(Sha256::digest(root_hasher.finalize()));

        self.storage
            .commit_file(
                bucket.id.to_vec(),
                file_name.to_string(),
                file_size as i64,
                hex::decode(root_cid)?,
            )
            .await?;

        todo!()
    }

    async fn create_chunk_upload(
        &mut self,
        index: usize,
        data: Vec<u8>,
        bucket_id: [u8; 32],
        file_name: &str,
    ) -> Result<(IpcFileChunkUpload, TransactionReceipt, IpcChunk), Box<dyn std::error::Error>>
    {
        let size = data.len();

        // TODO: if erasure code this value is different
        let block_size = BLOCK_SIZE;

        let mut dag = DagBuilder::new(data, block_size);

        let mut blocks = vec![];

        while let Some(block) = dag.next() {
            blocks.push(block);
        }

        let chunk_cid = dag.root_cid()?;

        let (cids, sizes, proto_chunk) =
            self.to_ipc_proto_chunks(chunk_cid.clone(), index, size, &blocks);

        let req = IpcFileUploadChunkCreateRequest {
            chunk: Some(proto_chunk.clone()),
            bucket_id: bucket_id.to_vec(),
            file_name: file_name.to_string(),
        };

        let res = self
            .client
            .file_upload_chunk_create(req)
            .await?
            .into_inner();

        let to_up_size = dag.count();
        let up_size = res.blocks.len();
        if up_size != to_up_size {
            return Err(format!(
                "received unexpected amount of blocks {}, expected {}",
                up_size, to_up_size
            ))?;
        }

        let mut res_iter = res.blocks.into_iter().enumerate();
        while let Some((idx, res_block)) = res_iter.next() {
            blocks[idx].node_address = res_block.node_address;
            blocks[idx].node_id = res_block.node_id;
            blocks[idx].permit = res_block.permit;
        }

        let tx = self
            .storage
            .add_file_chunk(
                hex::decode(chunk_cid.clone())?,
                bucket_id.to_vec(),
                file_name.to_string(),
                size.into(),
                cids.iter()
                    .map(|cid| hex::decode(cid).unwrap().as_slice().try_into())
                    .collect::<Result<_, _>>()?,
                sizes
                    .iter()
                    .map(|s| s.to_owned().into())
                    .collect::<Vec<_>>()
                    .clone(),
                index.into(),
            )
            .await?;

        Ok((
            IpcFileChunkUpload {
                index,
                chunk_cid,
                actual_size: size,
                raw_data_size: size,   // TODO: WRONG!!
                proto_node_size: size, // TODO: WRONG!!
                blocks,
                bucket_id,
                file_name: file_name.to_string(),
            },
            tx,
            proto_chunk,
        ))
    }

    fn to_ipc_proto_chunks(
        &self,
        chunk_cid: String,
        index: usize,
        size: usize,
        blocks: &Vec<FileBlockUpload>,
    ) -> (Vec<String>, Vec<i64>, IpcChunk) {
        let mut chunk_blocks = vec![];
        let mut cids = vec![];
        let mut sizes = vec![];
        blocks.iter().for_each(|block| {
            let size = block.data.len() as i64;
            let cid = block.cid.clone();
            chunk_blocks.push(Block {
                cid: cid.clone(),
                size,
            });
            cids.push(cid);
            sizes.push(size);
        });

        (
            cids,
            sizes,
            IpcChunk {
                cid: chunk_cid,
                index: index as i64,
                size: size as i64,
                blocks: chunk_blocks,
            },
        )
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
        let boxed_chunk = Box::new(proto_chunk);
        while let Some((idx, block)) = iter_blocks.next() {
            blocks_upload.push(IpcFileBlockData {
                bucket_id: bucket_id.clone(),
                data: block.data,
                cid: block.cid,
                chunk: Some(*boxed_chunk.to_owned()),
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

        /* while let Some(block) = blocks.next() {
            self.client.
        } */

        println!("before this?");

        Ok(())
    }

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
    use std::{env, fs::File, future::Future}; // crate for test-only use. Cannot be used in non-test code.

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

    async fn test_upload_file() {
        println!("Test 5: upload a file to {}", BUCKET_TO_TEST);
        let mut sdk = get_sdk().await.unwrap();
        let file = File::open("foo.txt").unwrap();
        let _ = sdk
            .upload_file(BUCKET_TO_TEST, "foo", file, "passwd")
            .await
            .unwrap();
    }

    async fn test_view_uploaded_file() {}

    #[tokio::test]
    async fn test_all() {
        // env::set_var("RUST_BACKTRACE", "1");
        // test_create_bucket().await;
        // test_list_buckets().await;
        // test_view_bucket().await;
        // test_delete_bucket().await;
        test_upload_file().await;
        // test_view_uploaded_file().await;
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
