use std::{
    env,
    fs::create_dir_all,
    path::{Path, PathBuf},
};

use prost::Message;
use prost_types::FileDescriptorSet;

fn main() {
    #[cfg(feature = "vendored-protoc")]
    std::env::set_var("PROTOC", protobuf_src::protoc());

    // We don't have direct access (via #cfg[]) to target_arch
    // during build process, it always returns the host arch
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    let dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    build(&dir, "ipcnodeapi", target_arch);
}

// Define which messages are needed for WASM context
fn get_wasm_required_messages() -> Vec<&'static str> {
    vec![
        // Required by list_buckets
        "ipcnodeapi.IPCBucketListResponse",
        "ipcnodeapi.IpcBucketListResponse.IPCBucket",
        // Required by view_bucket
        "ipcnodeapi.IPCBucketViewResponse",
        // Required by view_file_info
        "ipcnodeapi.IPCFileViewResponse",
        // Required by list_files
        "ipcnodeapi.IPCFileListResponse",
        "ipcnodeapi.IpcFileListResponse.IPCFile",
        // Required by upload/download operations
        "ipcnodeapi.IPCChunk",
        "ipcnodeapi.IPCChunk.Block",
        "ipcnodeapi.IPCFileBlockData",
        "ipcnodeapi.IPCFileUploadChunkCreateResponse",
        "ipcnodeapi.IPCFileUploadChunkCreateResponse.BlockUpload",
        "ipcnodeapi.IPCFileDownloadCreateResponse",
        "ipcnodeapi.IPCFileDownloadCreateResponse.Chunk",
        "ipcnodeapi.IPCFileDownloadChunkCreateResponse",
        "ipcnodeapi.IPCFileDownloadChunkCreateResponse.BlockDownload",
        // Google standard types
        ".google.protobuf.Timestamp",
    ]
}

fn build(dir: &Path, proto: &str, target_arch: String) {
    let out = dir.join(proto);
    create_dir_all(&out).unwrap();
    let source = format!("proto/{proto}.proto");
    let descriptor_file = out.join("ipcnodeapi_descriptors.bin");

    println!("cargo:rerun-if-changed={}", source);
    println!("cargo:rerun-if-changed=build.rs");

    #[cfg(feature = "vendored-protox")]
    {
        let file_descriptors = protox::compile([source.clone()], ["proto/".to_string()]).unwrap();
        // Use protox's re-exported prost (0.12) Message trait via UFCS to avoid
        // version conflict with our build-dependency on prost 0.13.
        let bytes = protox::prost::Message::encode_to_vec(&file_descriptors);
        std::fs::write(&descriptor_file, bytes).unwrap();
    }

    // Determine if we're building for WASM
    let is_wasm = target_arch == "wasm32";

    // Get the list of required messages for WASM
    let wasm_messages = get_wasm_required_messages();

    // Configure protobuf code generation
    let mut conf = tonic_build::configure();

    // Common configuration for both WASM and non-WASM
    conf = conf
        .extern_path(".google.protobuf", "::prost_types")
        .file_descriptor_set_path(&descriptor_file)
        .compile_well_known_types(true)
        .build_server(false)
        .build_client(true)
        .client_mod_attribute("attrs", "#[cfg(feature = \"client\")]");

    // WASM-specific configurations
    if is_wasm {
        // Add serialization derivation for all messages
        conf = conf
            .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
            .field_attribute(
                "created_at",
                r#"#[serde(with = "crate::utils::timestamp::timestamp_serde")]"#,
            );

        // For each message that requires Tsify, add the specific attribute
        for msg in wasm_messages {
            conf = conf
                .type_attribute(msg, "#[derive(tsify_next::Tsify)]")
                .type_attribute(msg, "#[serde(rename_all = \"camelCase\")]")
                .type_attribute(msg, "#[tsify(into_wasm_abi, from_wasm_abi)]");
        }

        // Add specific field attributes as needed to optimize TS types. Below is an example of how to do this.
        // conf = conf
        //     .field_attribute("created_at", "#[tsify(type = \"String\")]")
        //     .field_attribute("encoded_size", "#[tsify(type = \"number\")]")
        //     .field_attribute("size", "#[tsify(type = \"number\")]")
        //     .field_attribute("index", "#[tsify(type = \"number\")]");

        // Only show optimization message if we're building for wasm32
        println!("cargo:warning=WASM_OPT: See WASM_OPTIMIZATION.md for optimization instructions");
    } else {
        // Non-WASM configuration
        conf = conf
            .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
            .field_attribute(
                "created_at",
                r#"#[serde(with = "crate::utils::timestamp::timestamp_serde")]"#,
            );
    }

    // Add any additional type-specific attributes
    conf = conf.type_attribute("routeguide.Point", "#[derive(Hash)]");

    #[cfg(feature = "vendored-protox")]
    {
        conf = conf.skip_protoc_run();
    }

    // Compile the protobuf definitions
    conf.compile_protos(&[source], &["proto/".to_string()])
        .unwrap();

    // Process the file descriptor for additional serde attributes
    let descriptor_bytes = std::fs::read(descriptor_file).unwrap();
    let descriptor = FileDescriptorSet::decode(&descriptor_bytes[..]).unwrap();
    prost_wkt_build::add_serde(out, descriptor);
}
