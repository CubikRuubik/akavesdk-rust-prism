use akave_rs::sdk::{AkaveSDK, AkaveSDKBuilder};
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the SDK
    let mut sdk = AkaveSDKBuilder::new("http://connect.akave.ai:5500")
        .with_default_encryption("testkey123")
        .with_erasure_coding(4, 2)
        .build()
        .await?;

    // Test address (replace with your test address)
    let test_address = "0x7975eD6b732D1A4748516F66216EE703f4856759";
    let bucket_name = "demo_bucket";
    let file_name = "test.txt";

    println!("Starting Akave SDK demo...");

    // Create a bucket
    println!("Creating bucket: {}", bucket_name);
    sdk.create_bucket(bucket_name).await?;

    // View bucket details
    println!("Viewing bucket details...");
    let bucket_view = sdk.view_bucket(test_address, bucket_name).await?;
    println!("Bucket name: {}", bucket_view.name);

    // Read the test file
    println!("Reading test file...");
    let mut file = File::open("./src/test.txt")?;

    // Upload the file
    println!("Uploading file to bucket...");
    sdk.upload_file(bucket_name, file_name, file, None).await?;

    // List files in the bucket
    println!("Listing files in bucket...");
    let file_list = sdk.list_files(test_address, bucket_name).await?;
    println!("Files in bucket:");
    for file in file_list.files {
        println!("- {}", file.name);
    }

    // View file info
    println!("Viewing file info...");
    let file_info = sdk.view_file_info(test_address, bucket_name, file_name).await?;
    println!("File info: {:?}", file_info);

    // Download the file
    println!("Downloading file...");
    sdk.download_file(test_address, bucket_name, file_name, None, "downloads").await?;
    println!("File downloaded to 'downloads' directory");

    // Clean up
    println!("Cleaning up...");
    sdk.delete_bucket(test_address, bucket_name).await?;
    println!("Demo completed successfully!");

    Ok(())
}