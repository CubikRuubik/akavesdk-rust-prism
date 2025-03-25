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
akave-sdk = "1.0.0"
```

### WebAssembly

Install the WASM package via npm:

```bash
npm install @akave/akave-web-sdk
```

## Usage

### Native Rust

```rust
use akave_sdk::AkaveIpcSDK;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize SDK
    let mut sdk = AkaveIpcSDK::new("http://connect.akave.ai:5500").await?;
    
    // Create a bucket
    let bucket_name = "my-bucket";
    let bucket = sdk.create_bucket(bucket_name).await?;
    
    // Upload a file
    let file = File::open("path/to/file")?;
    sdk.upload_file(bucket_name, "file.txt", file, None).await?;
    
    // List files
    let files = sdk.list_files("your-address", bucket_name).await?;
    
    // Download a file
    sdk.download_file("your-address", bucket_name, "file.txt", None, "/path/to/save").await?;
    
    Ok(())
}
```

### WebAssembly (Browser)

```javascript
import init, { AkaveWebSDK } from '@akave/akave-web-sdk';

async function initialize() {
    // Initialize WASM
    await init();
    
    // Create SDK instance
    const sdk = await AkaveWebSDK.new();
    
    // Connect wallet (requires MetaMask)
    const address = await sdk.connectWallet();
    
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
    await sdk.downloadFile(address, "my-bucket", "file.txt", "/path/to/save");
}
```

## API Reference

### Common Methods

#### Bucket Operations
- `create_bucket(name: &str) -> Result<BucketResponse>`
- `delete_bucket(address: &str, name: &str) -> Result<()>`
- `list_buckets(address: &str) -> Result<IpcBucketListResponse>`
- `view_bucket(address: &str, name: &str) -> Result<IpcBucketViewResponse>`

#### File Operations
- `upload_file(bucket_name: &str, file_name: &str, file: File, password: Option<&str>) -> Result<TransactionReceipt>`
- `download_file(address: &str, bucket_name: &str, file_name: &str, password: Option<&str>, destination: &str) -> Result<()>`
- `list_files(address: &str, bucket_name: &str) -> Result<IpcFileList>`
- `delete_file(address: &str, bucket_name: &str, file_name: &str) -> Result<()>`

### WASM-Specific Methods
- `connectWallet() -> Promise<string>` - Connects to MetaMask and returns the wallet address
- `new_with_endpoint(endpoint: string) -> Promise<AkaveWebSDK>` - Creates SDK instance with custom endpoint

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
- Server endpoint (defaults to production endpoint)
- Browser with WebAssembly support

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
- Native Rust examples (coming soon)

## Contributing

Contributions are welcome! Please read our contributing guidelines before submitting pull requests.

## License

This project is licensed under the MIT License - see the LICENSE file for details. 