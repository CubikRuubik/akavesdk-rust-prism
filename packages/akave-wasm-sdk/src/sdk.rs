use tonic::transport::{Channel, ClientTlsConfig};
use tonic_web_wasm_client::Client as GrpcWebClient;

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
use ipcnodeapi::IpcFileListResponse;
use ipcnodeapi::IpcBucketListResponse;

/// Represents the Akave SDK client
pub struct AkaveSDK {
    client: Transport,
}

enum Transport {
    Grpc(IpcNodeApiClient<Channel>),
    GrpcWeb(IpcNodeApiClient<GrpcWebClient>),
}

impl AkaveSDK {
    /// Creates a new AkaveSDK instance
    pub async fn new(server_address: &str, web_compat: bool) -> Result<Self, Box<dyn std::error::Error>> {
        let client = if web_compat {
            let grpc_web_client = GrpcWebClient::new(server_address.into());
            Transport::GrpcWeb(IpcNodeApiClient::new(grpc_web_client))
        } else {
            let channel = Channel::from_shared(server_address.to_string())?
                .tls_config(ClientTlsConfig::new())?
                .connect()
                .await?;
            Transport::Grpc(IpcNodeApiClient::new(channel))
        };
        Ok(Self { client })
    }

    /// List all buckets
    pub async fn list_buckets(&mut self, address: &str) -> Result<IpcBucketListResponse, Box<dyn std::error::Error>> {
        let request = IpcBucketListRequest {
            address: address.to_string(),
        };

        match &mut self.client {
            Transport::Grpc(client) => Ok(client.bucket_list(request).await?.into_inner()),
            Transport::GrpcWeb(client) => Ok(client.bucket_list(request).await?.into_inner()),
        }
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

        match &mut self.client {
            Transport::Grpc(client) => Ok(client.bucket_view(request).await?.into_inner()),
            Transport::GrpcWeb(client) => Ok(client.bucket_view(request).await?.into_inner()),
        }
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

        match &mut self.client {
            Transport::Grpc(client) => Ok(client.file_list(request).await?.into_inner()),
            Transport::GrpcWeb(client) => Ok(client.file_list(request).await?.into_inner()),
        }
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

        match &mut self.client {
            Transport::Grpc(client) => Ok(client.file_view(request).await?.into_inner()),
            Transport::GrpcWeb(client) => Ok(client.file_view(request).await?.into_inner()),
        }
    }
}
