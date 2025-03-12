// Proto module definition
pub mod ipcnodeapi {
    tonic::include_proto!("ipcnodeapi");
}

// Standard library imports
use std::{borrow::Cow, io::Write};

// External crate imports (general)
use alloy::hex;
use bytesize::{ByteSize, MB};
use cid::{
    multihash::{Code, MultihashDigest},
    Cid,
};
use prost_wkt_types::Timestamp;
use quick_protobuf::BytesReader;
use web3::types::{TransactionReceipt, U256};

// Proto-related imports
use ipcnodeapi::{
    ipc_chunk::Block, ipc_file_download_create_response::Chunk, 
    ipc_node_api_client::IpcNodeApiClient, ConnectionParamsRequest, 
    IpcBucketListRequest, IpcBucketListResponse, IpcBucketViewRequest, 
    IpcBucketViewResponse, IpcChunk, IpcFileBlockData, IpcFileDeleteRequest, 
    IpcFileDeleteResponse, IpcFileDownloadBlockRequest, IpcFileDownloadChunkCreateRequest, 
    IpcFileDownloadCreateRequest, IpcFileDownloadCreateResponse, IpcFileListRequest,
    IpcFileUploadChunkCreateRequest, IpcFileViewRequest, IpcFileViewResponse,
};

// Internal crate imports
use crate::blockchain::provider::BlockchainProvider;
use crate::blockchain::response_types::BucketResponse;
use crate::utils::dag::{ChunkDag, FileBlockUpload, RAW, DAG_PROTOBUF};
use crate::utils::encryption::Encryption;
use crate::utils::pb_data::PbData;
use crate::utils::splitter::Splitter;

// Target-specific imports and types
#[cfg(target_arch = "wasm32")]
mod wasm_imports {
    pub use wasm_bindgen_file_reader::WebSysFile as File;
    pub use tonic_web_wasm_client::Client as GrpcWebClient;
}

#[cfg(target_arch = "wasm32")]
use wasm_imports::*;

#[cfg(not(target_arch = "wasm32"))]
mod native_imports {
    pub use std::fs::File;
    pub use std::fs::OpenOptions;
    pub use tonic::transport::{Channel, ClientTlsConfig};
}

#[cfg(not(target_arch = "wasm32"))]
use native_imports::*;

// Target-specific type definitions
#[cfg(target_arch = "wasm32")]
type ClientTransport = GrpcWebClient;

#[cfg(not(target_arch = "wasm32"))]
type ClientTransport = Channel;

// Constants
const ENCRYPTION_OVERHEAD: usize = 32;
const BLOCK_SIZE: usize = MB as usize;
const MIN_BUCKET_NAME_LENGTH: usize = 3;
const MIN_FILE_SIZE: usize = 127;
const MAX_BLOCKS_IN_CHUNK: usize = 32;
const BLOCK_PART_SIZE: usize = ByteSize::kib(128).as_u64() as usize;

/// Represents the Akave SDK client
/// Akave Rust SDK should support both WASM (gRPC-Web) and native gRPC
pub struct IpcFileListItem {
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

struct AkaveBlockData {
    permit: String,
    node_address: String,
    node_id: String,
}
struct FileBlockDownload {
    cid: String,
    data: Vec<u8>,
    akave: AkaveBlockData,
}

struct FileChunkDownload {
    cid: String,
    index: i64,
    encoded_size: i64,
    size: i64,
    blocks: Vec<FileBlockDownload>,
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

            let mut client = IpcNodeApiClient::new(channel)
                .max_decoding_message_size(usize::MAX)
                .max_encoding_message_size(usize::MAX);
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
        println!("request:  {:?}", request);
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
        let bucket_idx = self.storage.get_bucket_index_by_name(bucket_name.to_string()).await?;

        self.storage
            .delete_bucket(bucket_id, bucket_name.into(), bucket_idx)
            .await?;
        Ok(())
    }

    // Delete an existing file
    // TODO: fixme
    pub async fn delete_file(
        &mut self,
        address: &str,
        bucket_name: &str,
        file_name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // let file = self.view_file_info(address, bucket_name, file_name).await?;
        // let fileinfo = self.storage.get_file_by_name(bucket_id, file_name).await?;
        let bucket = self.view_bucket(address, bucket_name).await?;
        let bucket_id = hex::decode(bucket.id.clone())?;
        self.storage.delete_file(file_name.to_string(), bucket_id).await?;

        Ok(())
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
        passwd: Option<&str>,
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

        let encryption = match passwd {
            Some(key) => Some(Encryption::new(key.as_bytes(), info.as_bytes())?),
            None => None,
        };

        let chunk_size = (BLOCK_SIZE * MAX_BLOCKS_IN_CHUNK) as u64;
        let chunker = Splitter::new(file, chunk_size, encryption);
        if chunker.size() == 0 {
            return Err("Empty file".into());
        }
        // ITERATE OVER 32MB CHUNKS
        let mut enum_blocks = chunker.into_iter().enumerate();

        let root_hasher = Code::Sha2_256;
        let mut root_hash = None;
        let mut file_size: usize = 0;

        while let Some((idx, Ok(block_32m))) = enum_blocks.next() {
            // CREATE CHUNK UPLOAD
            let (chunk, _, ipc_chunk) = self
                .create_chunk_upload(idx, block_32m.to_vec(), bucket.id, file_name)
                .await?;
            // INCREMENT FILE SIZE
            file_size += chunk.actual_size;
            // ADD CHUNK TO DAG ROOT
            root_hash = Some(root_hasher.digest(&chunk.chunk_cid.to_bytes()));
            // UPLOAD CHUNK

            let mut chunks_iter = chunk.blocks.iter().enumerate();
            while let Some((index, block_1mb)) = chunks_iter.next() {
                self.upload_chunk(IpcFileBlockData {
                    data: block_1mb.data.to_owned(),
                    cid: block_1mb.cid.to_string(),
                    index: index as i64,
                    chunk: Some(ipc_chunk.to_owned()),
                    bucket_id: bucket.id.to_vec(),
                    file_name: file_name.to_string(),
                })
                .await?;
            }

            /* self.upload_chunk(chunk, bucket.id.to_vec(), file_name.to_string(), ipc_chunk)
            .await?; */
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

    async fn create_chunk_upload(
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
        // let mut dag = DagBuilder::new(data, block_size);

        let chunk_dag = ChunkDag::new(block_size, data);
        let mut dag = chunk_dag.blocks.iter();

        // GET CIDS AND SIZES FROM to_ipc_proto_chunk

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
            cids.push(block_cid);
            sizes.push(U256::from(size));
        }

        let chunk_cid = chunk_dag.cid;

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
        let chunk_create_response = self
            .client
            .file_upload_chunk_create(chunk_create_request)
            .await?
            .into_inner();
        // UPDATE DAG INFO WITH RESPONSE FROM GRPC
        let mut blocks = chunk_dag.blocks;
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
        chunk: IpcFileBlockData,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let data = chunk.data;
        let data_len = data.len();
        if data_len == 0 {
            return Ok(());
        }

        let mut total = 0;

        println!("{:#?}", chunk.cid);

        let mut blocks_upload = vec![];

        while total < data_len {
            let mut end = total + BLOCK_PART_SIZE;
            if end > data_len {
                end = data_len;
            }

            let new_bock_part = data[total..end].to_vec();

            blocks_upload.push(IpcFileBlockData {
                bucket_id: chunk.bucket_id.clone(),
                data: new_bock_part,
                cid: if total == 0 {
                    chunk.cid.to_string()
                } else {
                    String::from("")
                },
                chunk: if total == 0 {
                    chunk.chunk.clone()
                } else {
                    None
                },
                file_name: chunk.file_name.clone(),
                index: chunk.index as i64,
            });

            total += BLOCK_PART_SIZE;
        }

        let block_stream = futures::stream::iter(blocks_upload);

        self.client
            .file_upload_block(block_stream)
            .await?
            .into_inner();
        Ok(())
    }

    async fn create_file_download(
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

    pub async fn download_file(
        &mut self,
        address: &str,
        bucket_name: &str,
        file_name: &str,
        passwd: Option<&str>,
        destination_path: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let info = vec![bucket_name, file_name].join("/");

        let option_encryption = match passwd {
            Some(key) => Some(Encryption::new(key.as_bytes(), info.as_bytes())?),
            None => None,
        };

        let file_download = self
            .create_file_download(address, bucket_name, file_name)
            .await?;

        let mut destination_file = OpenOptions::new()
            .create(true)
            .write(true)
            .open(format!("{}{}", destination_path, file_name))?;

        let codec = Cid::try_from(file_download.chunks[0].cid.clone())?.codec();

        let mut chunk_index = 0;
        for chunk in file_download.chunks {
            let chunk_cid = chunk.cid.clone();
            let chunk_download = self
                .create_chunk_download(bucket_name, file_name, address, chunk, chunk_index)
                .await?;

            let mut block_index = 0;
            for block in chunk_download.blocks {
                let mut chunk_data = vec![];
                // let mut retrieved_blocks = vec![];
                let req = IpcFileDownloadBlockRequest {
                    address: address.to_string(),
                    chunk_cid: chunk_cid.clone(),
                    chunk_index,
                    block_cid: block.cid.clone(),
                    block_index,
                    bucket_name: bucket_name.to_string(),
                    file_name: file_name.to_string(),
                };
                let mut stream = self.client.file_download_block(req).await?.into_inner();

                while let Some(mut message) = stream.message().await? {
                    chunk_data.append(message.data.as_mut());
                }

                let final_data = match codec {
                    RAW => chunk_data,
                    DAG_PROTOBUF => {
                        let mut reader = BytesReader::from_bytes(&chunk_data);

                        let mut msg = PbData::default();
                        while !reader.is_eof() {
                            match reader.next_tag(&chunk_data) {
                                Ok(18) => {
                                    msg.data =
                                        Some(reader.read_bytes(&chunk_data).map(Cow::Borrowed)?)
                                }
                                Ok(_) => {}
                                Err(_) => return Err("error decoding message")?,
                            }
                        }

                        msg.data.unwrap().into_owned()
                    }
                    _default => Err("Unknown codec for decoding message")?,
                };

                let decrypted_data = match option_encryption {
                    Some(ref encryption) => encryption
                        .decrypt(&final_data, format!("block_{}", block_index).as_bytes())?,
                    None => final_data,
                };

                destination_file.write_all(&decrypted_data)?;
                destination_file.flush()?;
                block_index += 1;
            }
            chunk_index += 1;
        }

        Ok(())
    }

    async fn create_chunk_download(
        &mut self,
        bucket_name: &str,
        file_name: &str,
        address: &str,
        chunk: Chunk,
        index: i64,
    ) -> Result<FileChunkDownload, Box<dyn std::error::Error>> {
        let request = IpcFileDownloadChunkCreateRequest {
            bucket_name: bucket_name.to_string(),
            file_name: file_name.to_string(),
            chunk_cid: chunk.cid.clone(),
            address: address.to_string(),
        };

        let resp = self
            .client
            .file_download_chunk_create(request)
            .await?
            .into_inner();
        let mut blocks = vec![];
        for block in resp.blocks {
            blocks.push(FileBlockDownload {
                cid: block.cid,
                data: [].to_vec(),
                akave: AkaveBlockData {
                    node_id: block.node_id,
                    permit: block.permit,
                    node_address: block.node_address,
                },
            });
        }

        Ok(FileChunkDownload {
            cid: chunk.cid,
            index,
            encoded_size: chunk.encoded_size,
            size: chunk.size,
            blocks,
        })
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use crate::sdk::AkaveIpcSDK;
    use pretty_assertions::{assert_eq, assert_ne};
    use std::fs::{self, File};
    use std::path::Path;
    use uuid::Uuid;

    const ADDRESS: &str = "0x7975eD6b732D1A4748516F66216EE703f4856759";
    const FILE_NAME_TO_TEST: &str = "5MB.txt";
    const DOWNLOAD_DESTINATION: &str = "/tmp/akave-tests/";

    async fn get_sdk() -> Result<AkaveIpcSDK, Box<(dyn std::error::Error + 'static)>> {
        AkaveIpcSDK::new("http://connect.akave.ai:5500").await
    }

    // Helper to create a unique bucket name for each test
    fn generate_test_bucket_name() -> String {
        format!("TEST_BUCKET_{}", Uuid::new_v4().to_string().split('-').next().unwrap())
    }

    // Helper to clean up downloaded files
    fn cleanup_download(file_path: &str) {
        if Path::new(file_path).exists() {
            let _ = fs::remove_file(file_path);
        }
    }

    // Helper to ensure download directory exists
    fn ensure_download_dir() {
        let _ = fs::create_dir_all(DOWNLOAD_DESTINATION);
    }

    #[tokio::test]
    async fn test_create_bucket() {
        let bucket_name = generate_test_bucket_name();
        println!("Testing create bucket: {}", bucket_name);
        
        // Test
        let mut sdk = get_sdk().await.unwrap();
        let bucket_resp = sdk.create_bucket(&bucket_name).await.unwrap();
        assert_eq!(bucket_resp.name, bucket_name);
        
        // Cleanup
        let _ = sdk.delete_bucket(ADDRESS, &bucket_name).await;
    }

    #[tokio::test]
    async fn test_list_buckets() {
        let bucket_name = generate_test_bucket_name();
        println!("Testing list buckets");
        
        // Setup
        let mut sdk = get_sdk().await.unwrap();
        let _ = sdk.create_bucket(&bucket_name).await.unwrap();
        
        // Test
        let buckets = sdk.list_buckets(ADDRESS).await.unwrap();
        let len = buckets.buckets.len();
        println!("Found {} buckets", len);
        assert_ne!(len, 0, "there should be buckets in this account");
        
        // Cleanup
        let _ = sdk.delete_bucket(ADDRESS, &bucket_name).await;
    }

    #[tokio::test]
    async fn test_view_bucket() {
        let bucket_name = generate_test_bucket_name();
        println!("Testing view bucket: {}", bucket_name);
        
        // Setup
        let mut sdk = get_sdk().await.unwrap();
        let _ = sdk.create_bucket(&bucket_name).await.unwrap();
        
        // Test
        let bucket = sdk.view_bucket(ADDRESS, &bucket_name).await.unwrap();
        assert_eq!(bucket.name, bucket_name);
        
        // Cleanup
        let _ = sdk.delete_bucket(ADDRESS, &bucket_name).await;
    }

    #[tokio::test]
    async fn test_delete_bucket() {
        let bucket_name = generate_test_bucket_name();
        println!("Testing delete bucket: {}", bucket_name);
        
        // Setup
        let mut sdk = get_sdk().await.unwrap();
        let _ = sdk.create_bucket(&bucket_name).await.unwrap();
        
        // Test delete
        let result = sdk.delete_bucket(ADDRESS, &bucket_name).await;
        assert!(result.is_ok(), "Failed to delete bucket: {:?}", result.err());
        
        // Verify deletion - this might need adjustment based on expected behavior
        // If view_bucket is expected to return an error for non-existent buckets:
        let view_result = sdk.view_bucket(ADDRESS, &bucket_name).await;
        assert!(view_result.is_err() || view_result.unwrap().name != bucket_name,
                "Bucket should not exist after deletion");
    }

    #[tokio::test]
    async fn test_upload_and_list_files() {
        let bucket_name = generate_test_bucket_name();
        println!("Testing upload file to bucket: {}", bucket_name);
        
        // Setup
        let mut sdk = get_sdk().await.unwrap();
        let _ = sdk.create_bucket(&bucket_name).await.unwrap();
        
        // Test upload
        let file = File::open(format!("test_files/{}", FILE_NAME_TO_TEST)).unwrap();
        let upload_result = sdk
            .upload_file(&bucket_name, FILE_NAME_TO_TEST, file, None)
            .await;
        assert!(upload_result.is_ok(), "Failed to upload file: {:?}", upload_result.err());
        
        // Test list files
        let files = sdk.list_files(ADDRESS, &bucket_name).await.unwrap();
        assert_ne!(files.len(), 0, "there should be files in this bucket");
        let has_test_file = files.iter().any(|file| file.name == FILE_NAME_TO_TEST);
        assert!(has_test_file, "Uploaded file not found in bucket");

        // Test delete files and list
        for file in files {
            let _ = sdk.delete_file(ADDRESS, &bucket_name, &file.name).await.expect("failed to delete file");
        }
        let files = sdk.list_files(ADDRESS, &bucket_name).await.unwrap();
        assert_eq!(files.len(), 0, "there should be no files in this bucket");
        
        
        // Cleanup
        let _ = sdk.delete_bucket(ADDRESS, &bucket_name).await;
    }

    #[tokio::test]
    async fn test_download_file() {
        let bucket_name = generate_test_bucket_name();
        println!("Testing download file from bucket: {}", bucket_name);
        
        // Setup
        ensure_download_dir();
        let download_path = format!("{}{}", DOWNLOAD_DESTINATION, FILE_NAME_TO_TEST);
        let mut sdk = get_sdk().await.unwrap();
        let _ = sdk.create_bucket(&bucket_name).await.unwrap();
        
        let file = File::open(format!("test_files/{}", FILE_NAME_TO_TEST)).unwrap();
        let _ = sdk
            .upload_file(&bucket_name, FILE_NAME_TO_TEST, file, None)
            .await
            .unwrap();
        
        // Clean up any previously downloaded file
        cleanup_download(&download_path);
        
        // Test download
        let download_result = sdk.download_file(
            ADDRESS,
            &bucket_name,
            FILE_NAME_TO_TEST,
            None,
            DOWNLOAD_DESTINATION,
        ).await;
        
        assert!(download_result.is_ok(), "Failed to download file: {:?}", download_result.err());
        assert!(Path::new(&download_path).exists(), "Downloaded file not found");
        
        // Cleanup
        cleanup_download(&download_path);
        let _ = sdk.delete_bucket(ADDRESS, &bucket_name).await;
    }

    #[tokio::test]
    async fn test_full_lifecycle() {
        let bucket_name = generate_test_bucket_name();
        println!("Testing full lifecycle with bucket: {}", bucket_name);
        
        // Setup
        ensure_download_dir();
        let download_path = format!("{}{}", DOWNLOAD_DESTINATION, FILE_NAME_TO_TEST);
        let mut sdk = get_sdk().await.unwrap();
        
        // Create bucket
        let bucket_resp = sdk.create_bucket(&bucket_name).await.unwrap();
        assert_eq!(bucket_resp.name, bucket_name);
        
        // Upload file
        let file = File::open(format!("test_files/{}", FILE_NAME_TO_TEST)).unwrap();
        let _ = sdk
            .upload_file(&bucket_name, FILE_NAME_TO_TEST, file, None)
            .await
            .unwrap();
        
        // List files
        let files = sdk.list_files(ADDRESS, &bucket_name).await.unwrap();
        let has_test_file = files.iter().any(|file| file.name == FILE_NAME_TO_TEST);
        assert!(has_test_file, "Uploaded file not found in bucket");
        
        // Download file
        cleanup_download(&download_path); // Clean up any previously downloaded file
        let download_result = sdk.download_file(
            ADDRESS,
            &bucket_name,
            FILE_NAME_TO_TEST,
            None,
            DOWNLOAD_DESTINATION,
        ).await;
        assert!(download_result.is_ok());
        assert!(Path::new(&download_path).exists());
        
        // Cleanup
        cleanup_download(&download_path);
        let _ = sdk.delete_bucket(ADDRESS, &bucket_name).await;
    }

    // helper cleanup function
    async fn cleanup_all_test_resources() {
        println!("Starting cleanup of all test resources...");
        
        let mut sdk = get_sdk().await.unwrap();
        
        // 1. List all buckets
        println!("Listing all buckets to find test buckets...");
        let buckets = match sdk.list_buckets(ADDRESS).await {
            Ok(bucket_list) => bucket_list.buckets,
            Err(e) => {
                println!("Error listing buckets: {:?}", e);
                return;
            }
        };
        
        let test_buckets: Vec<_> = buckets
            .iter()
            .filter(|bucket| bucket.name.starts_with("TEST_BUCKET_"))
            .collect();
        
        println!("Found {} test buckets to clean up", test_buckets.len());
        
        // 2. Clean up any downloaded test files
        println!("Cleaning up downloaded test files...");
        if let Ok(entries) = fs::read_dir(DOWNLOAD_DESTINATION) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_file() {
                        if let Ok(file_name) = entry.file_name().into_string() {
                            println!("Removing downloaded file: {}", file_name);
                            let _ = fs::remove_file(entry.path());
                        }
                    }
                }
            }
        }
        
        // 3. Empty each test bucket then delete.
        for bucket in test_buckets {
            println!("Deleting test bucket: {}", bucket.name);
            // TODO: fixme
            // let files = sdk.list_files(ADDRESS, &bucket.name).await.expect("Failed to list files for specified (address, bucket_name)");
            
            // for file in files {
            //     println!("Deleting test file: {}, from bucket: {}", file.name, bucket.name);
            //     match sdk.delete_file(ADDRESS, bucket.name.as_str(), file.name.as_str()).await {
            //         Ok(_) => println!("Successfully deleted file: {}", file.name),
            //         Err(e) => println!("Error deleting bucket {}: {:?}", bucket.name, e),
            //     }
            // }
            match sdk.delete_bucket(ADDRESS, &bucket.name).await {
                Ok(_) => println!("Successfully deleted bucket: {}", bucket.name),
                Err(e) => println!("Error deleting bucket {}: {:?}", bucket.name, e),
            }
        }
        
        println!("Cleanup complete!");
    }
    
    #[tokio::test]
    #[ignore]
    async fn test_cleanup_manual() {
        // This test is ignored by default and must be run manually with:
        // cargo test --package akave-wasm-sdk --lib -- tests::test_cleanup_manual --ignored --nocapture
        cleanup_all_test_resources().await;
    }
}