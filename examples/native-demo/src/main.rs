use akave_rs::sdk::AkaveSDKBuilder;
use env_logger::Builder;
use log::LevelFilter;
use std::fs::File;
use std::path::Path;

const TEST_PASSWORD: &str = "testkey123";
const FILE_NAME_TO_TEST: &str = "2MB.txt";
const TEST_ADDRESS: &str = "0x7975eD6b732D1A4748516F66216EE703f4856759";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    Builder::new()
        .filter_level(LevelFilter::Debug) // Set to Debug to see all logs
        .format_timestamp(None)
        .init();

    // Initialize the SDK
    let sdk = AkaveSDKBuilder::new("http://23.227.172.82:5001")
        .with_default_encryption(TEST_PASSWORD)
        .with_erasure_coding(4, 2)
        .build()
        .await?;
    println!("Starting Akave SDK demo...");
    let bucket_name = format!(
        "demo_bucket_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    );
    // Create a bucket
    println!("Creating bucket: {}", bucket_name);
    sdk.create_bucket(&bucket_name).await?;

    // View bucket details
    println!("Viewing bucket details...");
    let bucket_view = sdk.view_bucket(TEST_ADDRESS, &bucket_name).await?;
    println!("Bucket name: {}", bucket_view.name);

    // Read the test file
    println!("Reading test file...");
    let test_file_path = format!("test_files/{}", FILE_NAME_TO_TEST);
    if !Path::new(&test_file_path).exists() {
        return Err(format!("Test file not found at: {}", test_file_path).into());
    }
    let mut upload_file = File::open(&test_file_path)?;

    // Upload the file
    println!("Uploading file to bucket...");
    sdk.upload_file(&bucket_name, FILE_NAME_TO_TEST, &mut upload_file, None)
        .await?;

    // List files in the bucket
    println!("Listing files in bucket...");
    let file_list = sdk.list_files(TEST_ADDRESS, &bucket_name).await?;
    println!("Files in bucket:");
    for file in file_list.files {
        println!("- {}", file.name);
    }

    // View file info
    println!("Viewing file info...");
    let file_info = sdk
        .view_file_info(TEST_ADDRESS, &bucket_name, FILE_NAME_TO_TEST)
        .await?;
    println!("File info: {:?}", file_info);

    // Create downloads directory if it doesn't exist
    std::fs::create_dir_all("test_files/downloads")?;

    let download_file = File::create(Path::new("test_files/downloads").join(FILE_NAME_TO_TEST))
        .map_err(|e| format!("Failed to open test file: {}", e))?;

    // sleep(Duration::from_secs(5));
    // Download the file
    println!("Downloading file...");
    sdk.download_file(
        TEST_ADDRESS,
        &bucket_name,
        FILE_NAME_TO_TEST,
        None,
        download_file,
    )
    .await?;
    println!("File downloaded successfully!");

    // Delete the file
    println!("Deleting file...");
    sdk.delete_file(TEST_ADDRESS, &bucket_name, FILE_NAME_TO_TEST)
        .await?;
    println!("File deleted successfully!");

    // Clean up
    println!("Cleaning up...");
    sdk.delete_bucket(TEST_ADDRESS, &bucket_name).await?;
    println!("Demo completed successfully!");

    Ok(())
}
