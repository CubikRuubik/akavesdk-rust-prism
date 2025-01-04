pub mod ipcnodeapi {
    tonic::include_proto!("ipcnodeapi");
}

use ipcnodeapi::{
    ipc_node_api_client::IpcNodeApiClient,
    IpcBucketListRequest,
    IpcBucketViewRequest,
    IpcFileListRequest,
    IpcFileViewRequest,
};
use ipcnodeapi::{IpcFileListResponse, IpcBucketListResponse};


/// Conditionally use grpc-web is target arch is wasm32.
#[cfg(target_arch = "wasm32")]
use tonic_web_wasm_client::Client as GrpcWebClient;
/// Otherwise default to grpc.
#[cfg(not(target_arch = "wasm32"))]
use tonic::transport::{Channel, ClientTlsConfig};

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
    pub async fn list_buckets(&mut self, address: &str) -> Result<IpcBucketListResponse, Box<dyn std::error::Error>> {
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
    ) -> Result<ipcnodeapi::IpcBucketViewResponse, Box<dyn std::error::Error>> {
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
    ) -> Result<ipcnodeapi::IpcFileViewResponse, Box<dyn std::error::Error>> {
        let request = IpcFileViewRequest {
            bucket_name: bucket_name.to_string(),
            file_name: file_name.to_string(),
            address: address.to_string(),
        };
        Ok(self.client.file_view(request).await?.into_inner())
    }
}
