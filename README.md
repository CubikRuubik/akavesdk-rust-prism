# Akave SDK

The Akave SDK is a Rust-based software development kit that provides a unified interface for interacting with the Akave blockchain storage system. It supports both native Rust applications and WebAssembly (WASM) environments, making it versatile for various use cases.

## Features

- **Dual Platform Support**
  - Native Rust applications
  - WebAssembly (WASM) for browser-based applications
- **Bucket Management**
  - Create and delete buckets
  - List buckets
  - View bucket details
- **File Operations**
  - Upload files with optional encryption
  - Download files
  - List files within buckets
  - Delete files
- **Blockchain Integration**
  - Seamless interaction with Akave blockchain
  - Transaction management
  - Wallet integration (MetaMask for WASM)

## Installation

### Native Rust

Add the following to your `Cargo.toml`:

```toml
[dependencies]
akave-rs = "1.0.0"
```

### WebAssembly

Install the WASM package via npm:

```bash
npm install @akave/akave-web-sdk
```

## Usage

### Native Rust

```rust
use akave_rs::AkaveSDK;
use std::fs::File;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize SDK
    let mut sdk = AkaveSDK::new("http://23.227.172.82:7001/grpc").await?;

    // Create a bucket
    let bucket_name = "my-bucket";
    let bucket = sdk.create_bucket(bucket_name).await?;

    // Upload a file
    let file = File::open("path/to/file")?;
    sdk.upload_file(bucket_name, "file.txt", file, None).await?;

    // List files
    let files = sdk.list_files("your-address", "my-bucket").await?;

    // Download a file
    sdk.download_file("your-address", bucket_name, "file.txt", None, "/path/to/save/").await?;

    Ok(())
}
```

### WebAssembly (Browser)

```javascript
import init, { AkaveWebSDKBuilder } from "@akave/akave-web-sdk";

async function initialize() {
  // Initialize WASM
  await init();

  // Create SDK instance with builder pattern
  const sdk = await new AkaveWebSDKBuilder("http://23.227.172.82:7001/grpc")
    .withDefaultEncryption("encryption-key")
    .withErasureCoding(4, 2)
    .build();

  // Get wallet address from MetaMask (requires MetaMask to be connected separately)
  const address = (
    await window.ethereum.request({ method: "eth_requestAccounts" })
  )[0];

  // Create bucket
  await sdk.createBucket("my-bucket");

  // List buckets
  const buckets = await sdk.listBuckets(address);

  // Upload file
  const fileInput = document.querySelector('input[type="file"]');
  const file = fileInput.files[0];
  await sdk.uploadFile("my-bucket", "file.txt", file);

  // List files
  const files = await sdk.listFiles(address, "my-bucket");

  // Download file
  await sdk.downloadFile(address, "my-bucket", "file.txt", "download");
}
```

## Build from source

```
git clone git@github.com:lightshiftdev/akave-rs.git
cd akave-rs
```

### Native build

```
cargo build
```

### Web wasm

```
wasm-pack build --target web
```

<!--
wasm-pack --verbose build --target web
cargo build --target wasm32-unknown-unknown --release
wasm-bindgen target/wasm32-unknown-unknown/release/akave_rs.wasm \
  --out-dir ./pkg \
  --target web
-->

## API Reference

### Common Methods

#### Bucket Operations

- `create_bucket(name: &str) -> Result<BucketResponse>` - Creates a new bucket with the specified name.
- `delete_bucket(address: &str, name: &str) -> Result<()>` - Deletes the specified bucket for the given address.
- `list_buckets(address: &str) -> Result<BucketListResponse>` - Lists all buckets associated with the specified address.
- `view_bucket(address: &str, name: &str) -> Result<BucketViewResponse>` - Retrieves details of the specified bucket for the given address.

#### File Operations

- `upload_file(bucket_name: &str, file_name: &str, file: File, password: Option<&str>) -> Result<TransactionReceipt>` - Uploads a file to the specified bucket with optional encryption.
- `download_file(address: &str, bucket_name: &str, file_name: &str, password: Option<&str>, destination: &str) -> Result<()>` - Downloads the specified file from the bucket to the given destination path with optional decryption.
- `list_files(address: &str, bucket_name: &str) -> Result<FileListResponse>` - Lists all files within the specified bucket for the given address.
- `delete_file(address: &str, bucket_name: &str, file_name: &str) -> Result<()>` - Deletes the specified file from the given bucket for the address.

### WASM-Specific Methods

- `build() -> Promise<AkaveWebSDK>` - Builds the SDK with the configured options.
- `new_with_endpoint(endpoint: string) -> Promise<AkaveWebSDK>` - Creates an SDK instance with a custom endpoint.

## Configuration

### Native Configuration

The native SDK can be configured with:

- Server endpoint
- Poll interval for transaction confirmation
- Number of confirmations required
- Private key for signing transactions

### WASM Configuration

The WASM SDK requires:

- MetaMask or compatible Web3 wallet
- Server endpoint
- Browser with WebAssembly support

Additional configuration options via the builder pattern:

- `withErasureCoding(dataBlocks, parityBlocks)` - Configure erasure coding parameters
- `withDefaultEncryption(key)` - Set default encryption key
- `withBlockSize(size)` - Set block size
- `withMinBucketLength(length)` - Set minimum bucket name length
- `withMaxBlocksInChunk(blocks)` - Set maximum blocks in chunk
- `withBlockPartSize(size)` - Set block part size

## Error Handling

The SDK provides comprehensive error handling for both platforms:

- Network errors
- Transaction failures
- File operation errors
- Wallet connection issues
- Invalid input validation

## Security

- File encryption support with password protection
- Secure wallet integration
- TLS encryption for all network communications
- Transaction signing and verification

## Examples

See the `examples` directory for complete usage examples:

- `web-demo/` - Browser-based example application
- `native-demo/` Native Rust e2e example

## Contributing

Contributions are welcome! Please read our contributing guidelines before submitting pull requests.

## License

TODO
