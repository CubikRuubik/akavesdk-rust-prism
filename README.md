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

## Examples

See the `examples` directory for complete usage examples and ready to use base boilerplates:

- `native-demo/` - Native Rust e2e example
- `web-demo/` - Browser-based example application
- `web-demo-react/` - Real world browser-based typescript React example project:
  - TanStack Query for robust, declarative data fetching and state management.
  - RainbowKit for seamless wallet connection and a polished user experience.
  - Wagmi & Viem for type-safe, high-performance Ethereum (EVM) blockchain interactions, including integration with the Akave blockchain.
  - All components are written in TypeScript and designed for real-world scalability and maintainability.

## Installation

### Native Rust

Add the following to your `Cargo.toml`:

```toml
[dependencies]
akave-rs = "0.1.1"
```

### WebAssembly

Install the WASM package via npm:

```bash
npm install @akave/akave-rs
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
    let files = sdk.list_files(bucket_name).await?;

    // Download a file
    let output_file = File::create("/path/to/save/file.txt")?;
    sdk.download_file(bucket_name, "file.txt", output_file, None).await?;

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

  // Create bucket
  await sdk.createBucket("my-bucket");

  // List buckets
  const buckets = await sdk.listBuckets();

  // Upload file
  const fileInput = document.querySelector('input[type="file"]');
  const file = fileInput.files[0];
  const arrayBuffer = await file.arrayBuffer();
  await sdk.uploadFile("my-bucket", "file.txt", arrayBuffer);

  // List files
  const files = await sdk.listFiles("my-bucket");

  // Download file
  const result = await sdk.downloadFile("my-bucket", "file.txt", "download");
  if (result) {
    const blob = new Blob([result]);
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = "file.txt";
    a.click();
    URL.revokeObjectURL(url);
  }
}
```

## Build from source

```bash
git clone git@github.com:lightshiftdev/akave-rs.git
cd akave-rs
```

### Native build

```bash
cargo build
```

### Web wasm

```bash
wasm-pack build --target web
wasm-bindgen target/wasm32-unknown-unknown/release/akave_rs.wasm \
  --out-dir ./pkg \
  --target web
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
- `delete_bucket(name: &str) -> Result<()>` - Deletes the specified bucket.
- `list_buckets(&mut self) -> Result<BucketListResponse>` - Lists all buckets for the current user.
- `view_bucket(name: &str) -> Result<BucketViewResponse>` - Retrieves details of the specified bucket.

#### File Operations

- `upload_file<R: Read>(bucket_name: &str, file_name: &str, file: R, password: Option<&str>) -> Result<TransactionReceipt>` - Uploads a file to the specified bucket with optional encryption.
- `download_file<W: Write>(bucket_name: &str, file_name: &str, writer: W, password: Option<&str>) -> Result<()>` - Downloads the specified file from the bucket to the given writer with optional decryption.
- `list_files(bucket_name: &str) -> Result<FileListResponse>` - Lists all files within the specified bucket.
- `delete_file(bucket_name: &str, file_name: &str) -> Result<()>` - Deletes the specified file from the given bucket.

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

## Contributing

Contributions are welcome!

Thanks to the Lightshift team for their contributions to the SDK: @CondeGil, @essamhassan, @krzysztof-ls. 👏

## License

[GPL 3.0](https://github.com/akave-ai/akavesdk-rs?tab=GPL-3.0-1-ov-file)
