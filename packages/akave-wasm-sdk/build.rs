use prost_wkt_build::*;
use std::{env, io, path::PathBuf};

fn main() -> io::Result<()> {
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let descriptor_file = out.join("descriptors.bin");
    let conf = tonic_build::configure()
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
        .extern_path(".google.protobuf.Any", "::prost_wkt_types::Any")
        .extern_path(".google.protobuf.Timestamp", "::prost_wkt_types::Timestamp")
        .extern_path(".google.protobuf.Value", "::prost_wkt_types::Value")
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
