use prost_wkt_build::*;
use std::{env, io, path::PathBuf};

fn main() -> io::Result<()> {
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let descriptor_file = out.join("descriptors.bin");
    let conf = tonic_build::configure()
        .type_attribute(
            ".",
            "#[derive(tsify_next::Tsify, serde::Serialize, serde::Deserialize)]",
        )
        .type_attribute(".", "#[serde(rename_all = \"camelCase\")]")
        .type_attribute(".", "#[tsify(into_wasm_abi, from_wasm_abi)]")
        .field_attribute("created_at", "#[tsify(type = \"String\")]")
        .extern_path(".google.protobuf", "::prost_wkt_types")
        .file_descriptor_set_path(&descriptor_file)
        .compile_well_known_types(true)
        .build_server(false)
        .build_client(true)
        .compile_protos(&["ipcnodeapi.proto"], &["./protos"]);

    let descriptor_bytes = std::fs::read(descriptor_file).unwrap();

    let descriptor = FileDescriptorSet::decode(&descriptor_bytes[..]).unwrap();

    prost_wkt_build::add_serde(out, descriptor);
    conf
}
