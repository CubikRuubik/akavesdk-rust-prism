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

fn build(dir: &Path, proto: &str, target_arch: String) {
    let out = dir.join(proto);
    create_dir_all(&out).unwrap();
    let source = format!("proto/{proto}.proto");
    let descriptor_file = out.join("ipcnodeapi_descriptors.bin");

    #[cfg(feature = "vendored-protox")]
    {
        let file_descriptors = protox::compile([source.clone()], ["proto/".to_string()]).unwrap();
        std::fs::write(&descriptor_file, file_descriptors.encode_to_vec()).unwrap();
        prost_build.skip_protoc_run();
    }

    let conf = match target_arch == "wasm32" {
        true => tonic_build::configure()
            .type_attribute(
                ".",
                "#[derive(tsify_next::Tsify, serde::Serialize, serde::Deserialize)]",
            )
            .type_attribute(".", "#[serde(rename_all = \"camelCase\")]")
            .type_attribute(".", "#[tsify(into_wasm_abi, from_wasm_abi)]")
            .field_attribute("created_at", "#[tsify(type = \"String\")]")
            .type_attribute("routeguide.Point", "#[derive(Hash)]"),
        false => tonic_build::configure()
            .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
            .type_attribute("routeguide.Point", "#[derive(Hash)]"),
    };

    conf.extern_path(".google.protobuf", "::prost_wkt_types")
        .file_descriptor_set_path(&descriptor_file)
        .compile_well_known_types(true)
        .build_server(false)
        .build_client(true)
        .client_mod_attribute("attrs", "#[cfg(feature = \"client\")]")
        .compile_protos(&[source], &["proto/".to_string()])
        .unwrap();

    let descriptor_bytes = std::fs::read(descriptor_file).unwrap();
    let descriptor = FileDescriptorSet::decode(&descriptor_bytes[..]).unwrap();

    prost_wkt_build::add_serde(out, descriptor);
}
