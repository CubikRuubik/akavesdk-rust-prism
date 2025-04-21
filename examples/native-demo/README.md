# Akave SDK Native Demo

This is a demonstration of the Akave SDK's native Rust implementation. The demo shows basic functionality for bucket and file operations.

## Features

- SDK initialization with builder pattern
- Bucket creation and deletion
- File upload and download
- File listing and information retrieval
- Erasure coding and encryption support

## Prerequisites

- Rust toolchain (2021 edition or later)
- Cargo package manager
- Local test files (located in the `test_files` directory)

## Running the Demo

1. Build the project:
```bash
cargo build
```

2. Run the demo:
```bash
cargo run
```

## Demo Overview

The demo performs the following operations in sequence:

1. **Initializes the SDK** using the builder pattern with erasure coding and encryption
2. **Creates a bucket** with a unique timestamp-based name
3. **Views bucket details** after creation
4. **Uploads a test file** (2MB.txt) to the bucket
5. **Lists files** in the bucket
6. **Views file information** of the uploaded file
7. **Downloads the file** to a local directory
8. **Deletes the file** from the bucket
9. **Cleans up** by deleting the bucket

## Code Explanation

The demo uses the `AkaveSDKBuilder` to configure the SDK with:
- Server endpoint: `http://23.227.172.82:5001`
- Default encryption with a test password
- Erasure coding with 4 data blocks and 2 parity blocks

```rust
let mut sdk = AkaveSDKBuilder::new("http://23.227.172.82:5001")
    .with_default_encryption(TEST_PASSWORD)
    .with_erasure_coding(4, 2)
    .build()
    .await?;
```

## Error Handling

The demo implements proper error handling with the Rust Result type and propagates errors up to the main function.

## Notes

- The demo uses a test Ethereum address for demonstration purposes
- Test files should be placed in the `test_files` directory before running
- Downloaded files will be saved to the `test_files/downloads` directory
