use tonic::transport::Channel;
use tonic::client::Grpc as GrpcClient; // gRPC client
use tonic_web_wasm_client::Client as GrpcWebClient; // gRPC-Web client
use std::sync::Arc;

enum TransportClient {
    Grpc(Arc<GrpcClient<Channel>>),
    GrpcWeb(Arc<GrpcWebClient>),
}

impl TransportClient {
    /// Create a new TransportClient instance
    pub async fn new(grpc_url: &str, use_grpc_web: bool) -> Result<Self, Box<dyn std::error::Error>> {
        if use_grpc_web {
            let grpc_web_client = GrpcWebClient::new(grpc_url.into());
            Ok(TransportClient::GrpcWeb(Arc::new(grpc_web_client)))
        } else {
            let channel = Channel::from_shared(grpc_url.to_string())?
                .connect()
                .await?;
            let grpc_client = GrpcClient::new(channel);
            Ok(TransportClient::Grpc(Arc::new(grpc_client)))
        }
    }

    // Make a dynamic gRPC call
    // pub async fn make_call<F, Fut>(&self, call: F) -> Result<(), Box<dyn std::error::Error>>
    // where
    //     F: FnOnce(&Self) -> Fut,
    //     Fut: std::future::Future<Output = Result<(), tonic::Status>>,
    // {
    //     call(self).await?;
    //     Ok(())
    // }
}
