use std::{
    env,
    fs::{self, create_dir_all},
    path::{Path, PathBuf},
    process::Command,
};

use prost::Message;
use prost_types::FileDescriptorSet;

fn main() {
    #[cfg(feature = "vendored-protoc")]
    std::env::set_var("PROTOC", protobuf_src::protoc());

    // We don't have direct access (via #cfg[]) to target_arch
    // during build process, it always returns the host arch
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    let target_triple = env::var("TARGET").unwrap_or_else(|_| "unknown".to_string());
    let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());

    let dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    build(&dir, "ipcnodeapi", target_arch, &target_triple, &profile);
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

fn build(dir: &Path, proto: &str, target_arch: String, target_triple: &str, profile: &str) {
    let out = dir.join(proto);
    create_dir_all(&out).unwrap();
    let source = format!("proto/{proto}.proto");
    let descriptor_file = out.join("ipcnodeapi_descriptors.bin");

    println!("cargo:rerun-if-changed={}", source);
    println!("cargo:rerun-if-changed=build.rs");

    #[cfg(feature = "vendored-protox")]
    {
        let file_descriptors = protox::compile([source.clone()], ["proto/".to_string()]).unwrap();
        std::fs::write(&descriptor_file, file_descriptors.encode_to_vec()).unwrap();
        conf.skip_protoc_run();
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
        conf = conf.type_attribute(
            ".",
            "#[derive(serde::Serialize, serde::Deserialize)]",
        ).field_attribute(   "created_at", 
        r#"#[serde(with = "crate::utils::timestamp::timestamp_serde")]"#);
        
        // For each message that requires Tsify, add the specific attribute
        for msg in wasm_messages {
            conf = conf
                .type_attribute(
                    msg,
                    "#[derive(tsify_next::Tsify)]",
                )
                .type_attribute(msg, "#[serde(rename_all = \"camelCase\")]")
                .type_attribute(msg, "#[tsify(into_wasm_abi, from_wasm_abi)]");
        }
        
        // Add specific field attributes
        // conf = conf
        //     .field_attribute("created_at", "#[tsify(type = \"String\")]")
        //     .field_attribute("encoded_size", "#[tsify(type = \"number\")]")
        //     .field_attribute("size", "#[tsify(type = \"number\")]")
        //     .field_attribute("index", "#[tsify(type = \"number\")]");
            
    } else {
        // Non-WASM configuration
        conf = conf.type_attribute(
            ".",
            "#[derive(serde::Serialize, serde::Deserialize)]",
        ).field_attribute(   "created_at", 
        r#"#[serde(with = "crate::utils::timestamp::timestamp_serde")]"#);
    }

    // Add any additional type-specific attributes 
    conf = conf.type_attribute("routeguide.Point", "#[derive(Hash)]");

    // Compile the protobuf definitions
    conf.compile_protos(&[source], &["proto/".to_string()]).unwrap();

    // Process the file descriptor for additional serde attributes
    let descriptor_bytes = std::fs::read(descriptor_file).unwrap();
    let descriptor = FileDescriptorSet::decode(&descriptor_bytes[..]).unwrap();
    prost_wkt_build::add_serde(out, descriptor);

    // Post-processing steps for WASM target in release mode
    // if is_wasm && profile == "release" {
    //     let _ = optimize_wasm(target_triple);
    // }
}

fn optimize_wasm(target_triple: &str) -> Result<(), String> {
    let out_dir = PathBuf::from("target")
        .join(target_triple)
        .join("release");
    
    // Find the wasm file
    let wasm_files: Vec<_> = fs::read_dir(&out_dir)
        .map_err(|e| format!("Failed to read output directory: {}", e))?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension()? == "wasm" {
                Some(path)
            } else {
                None
            }
        })
        .collect();

    let wasm_file = match wasm_files.first() {
        Some(file) => file,
        None => {
            println!("cargo:warning=WASM_OPT: No WASM file found in {}", out_dir.display());
            return Err(format!("No WASM file found in {}", out_dir.display()));
        }
    };

    println!("cargo:warning=WASM_OPT: Starting WASM optimization process: {}", wasm_file.display());
    
    // Get original size
    let original_size = fs::metadata(wasm_file)
        .map_err(|e| format!("Failed to get WASM file metadata: {}", e))?
        .len();
    println!("cargo:warning=WASM_OPT: Original size: {:.2} KB", original_size as f64 / 1024.0);

    // Create a backup
    let optimized_file = wasm_file.with_extension("wasm.opt");
    fs::copy(wasm_file, &optimized_file)
        .map_err(|e| format!("Failed to create backup of WASM file: {}", e))?;

    let mut current_size = original_size;
    // let mut optimization_failed = false;

    // 0. Run wasm-gc
    // let gc_path = wasm_file.with_extension("wasm.gc");
    match Command::new("wasm-gc")
        .arg(&optimized_file)
        .arg(&optimized_file)
        .output() {
        Ok(output) if output.status.success() => {
            // fs::rename(&gc_path, wasm_file)
            //     .map_err(|e| format!("Failed to replace with gc'd version: {}", e))?;
            let new_size = fs::metadata(&optimized_file)
            .map_err(|e| format!("Failed to get gc'd WASM file metadata: {}", e))?
            .len();
            println!(
                "cargo:warning=WASM_OPT: Size after wasm-gc: {}",
            format_size_change(current_size, new_size)
        );
            current_size = new_size;
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            println!("cargo:warning=WASM_OPT: wasm-gc failed: {}", stderr);
        }
        Err(e) => {
            println!("cargo:warning=WASM_OPT: wasm-gc not available or failed: {}", e);
        }
    }

    // 1. Run wasm-snip
    // let snip_path = wasm_file.with_extension("wasm.snip");
    match Command::new("wasm-snip")
        .arg("--snip-rust-fmt-code")
        .arg("--snip-rust-panicking-code")
        .arg("-o")
        .arg(&optimized_file)
        .arg(&optimized_file)
        .output() {
        Ok(output) if output.status.success() => {
            // fs::rename(&snip_path, wasm_file)
            //     .map_err(|e| format!("Failed to replace with snipped version: {}", e))?;
            let new_size = fs::metadata(&optimized_file)
            .map_err(|e| format!("Failed to get snipped WASM file metadata: {}", e))?
            .len();
            println!(
                "cargo:warning=WASM_OPT: Size after wasm-snip: {}",
                format_size_change(current_size, new_size)
            );
            current_size = new_size;
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            println!("cargo:warning=WASM_OPT: wasm-snip failed: {}", stderr);
            // optimization_failed = true;
        }
        Err(e) => {
            println!("cargo:warning=WASM_OPT: wasm-snip not available or failed: {}", e);
        }
    }

    // Only continue if previous step didn't fail
    // if !optimization_failed {
        // 2. Run wasm-strip
        // let strip_path = wasm_file.with_extension("wasm.strip");
    match Command::new("wasm-strip")
        .arg("-o")
        .arg(&optimized_file)
        .arg(&optimized_file)
        .output() {
        Ok(output) if output.status.success() => {
            // fs::rename(&strip_path, wasm_file)
            //     .map_err(|e| format!("Failed to replace with stripped version: {}", e))?;
            let new_size = fs::metadata(&optimized_file)
            .map_err(|e| format!("Failed to get stripped WASM file metadata: {}", e))?
            .len();
            println!(
                "cargo:warning=WASM_OPT: Size after wasm-strip: {}",
                format_size_change(current_size, new_size)
            );
            current_size = new_size;
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            println!("cargo:warning=WASM_OPT: wasm-strip failed: {}", stderr);
        }
        Err(e) => {
            println!("cargo:warning=WASM_OPT: wasm-strip not available or failed: {}", e);
        }
    }

    // 4. Run wasm-opt with additional flags (first pass)
    // let opt_path = wasm_file.with_extension("wasm.opt");
    match Command::new("wasm-opt")
        .arg("-O4")
        .arg("--enable-bulk-memory")
        .arg("--enable-threads")
        .arg("--enable-reference-types")
        .arg("--enable-simd")
        .arg("--enable-tail-call")
        .arg("--dce")                   // Dead code elimination
        .arg("--low-memory-unused")     // Free memory as early as possible
        .arg("--shrink-level=2")        // Aggressive name minification
        .arg("-o")
        .arg(&optimized_file)
        .arg(&optimized_file)
        .output() {
        Ok(output) if output.status.success() => {
            let new_size = fs::metadata(&optimized_file)
                .map_err(|e| format!("Failed to get optimized WASM file metadata: {}", e))?
                .len();
            println!(
                "cargo:warning=WASM_OPT: Size after wasm-opt -O4: {}",
                format_size_change(current_size, new_size)
            );
            current_size = new_size;

            // Second pass: size optimization
            // let oz_path = wasm_file.with_extension("wasm.oz");
            match Command::new("wasm-opt")
                .arg("-Oz")
                .arg("--enable-bulk-memory")
                .arg("--enable-threads")
                .arg("--enable-reference-types")
                .arg("--enable-simd")
                .arg("--enable-tail-call")
                .arg("--dce")                   // Dead code elimination
                .arg("--low-memory-unused")     // Free memory as early as possible
                .arg("--shrink-level=2")        // Aggressive name minification
                .arg("-o")
                .arg(&optimized_file)
                .arg(&optimized_file)
                .output() {
                Ok(output) if output.status.success() => {
                    let final_size = fs::metadata(&optimized_file)
                        .map_err(|e| format!("Failed to get size-optimized WASM file metadata: {}", e))?
                        .len();
                    println!(
                        "cargo:warning=WASM_OPT: Size after wasm-opt -Oz: {}",
                        format_size_change(current_size, final_size)
                    );
                    
                    // // Replace original with most optimized version
                    // fs::rename(&oz_path, wasm_file)
                    //     .map_err(|e| format!("Failed to replace with optimized version: {}", e))?;
                    
                    println!(
                        "cargo:warning=WASM_OPT: Total size reduction: {}",
                        format_size_change(original_size, final_size)
                    );
                }
                Ok(output) => {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    println!("cargo:warning=WASM_OPT: Second optimization pass failed: {}", stderr);
                    // optimization_failed = true;
                }
                Err(e) => {
                    println!("cargo:warning=WASM_OPT: wasm-opt (second pass) failed: {}", e);
                    // optimization_failed = true;
                }
            }
            
            // // Clean up intermediate files
            // fs::remove_file(&opt_path).ok();
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            println!("cargo:warning=WASM_OPT: First optimization pass failed: {}", stderr);
            // optimization_failed = true;
        }
        Err(e) => {
            println!("cargo:warning=WASM_OPT: wasm-opt not available or failed: {}", e);
            // optimization_failed = true;
        }
        // }
    }

    // if optimization_failed {
    //     println!("cargo:warning=WASM_OPT: Some optimization steps failed, restoring from backup");
    //     fs::rename(&backup_path, wasm_file)
    //         .map_err(|e| format!("Failed to restore WASM file from backup: {}", e))?;
    // } else {
    //     // Remove backup if everything succeeded
    //     fs::remove_file(&backup_path).ok();
    // }

    println!("cargo:warning=WASM_OPT: WASM optimization completed");
    Ok(())
}

fn format_size_change(original_size: u64, new_size: u64) -> String {
    let kb_original = original_size as f64 / 1024.0;
    let kb_new = new_size as f64 / 1024.0;
    let reduction = ((original_size - new_size) as f64 / original_size as f64 * 100.0).round();
    
    if original_size >= new_size {
        format!("{:.2} KB -> {:.2} KB ({:.1}% reduction)", kb_original, kb_new, reduction)
    } else {
        // Handle the unexpected case where size increases
        let increase = ((new_size - original_size) as f64 / original_size as f64 * 100.0).round();
        format!("{:.2} KB -> {:.2} KB ({:.1}% increase)", kb_original, kb_new, increase)
    }
}
