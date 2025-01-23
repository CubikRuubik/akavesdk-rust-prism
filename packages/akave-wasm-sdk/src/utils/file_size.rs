#[cfg(not(target_arch = "wasm32"))]
use std::fs::File;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_file_reader::WebSysFile as File;

pub trait FileSize {
    fn size(&self) -> u64;
}
#[cfg(not(target_arch = "wasm32"))]
impl FileSize for File {
    fn size(&self) -> u64 {
        self.metadata().unwrap().len() as u64
    }
}

#[cfg(target_arch = "wasm32")]
impl FileSize for File {
    fn size(&self) -> u64 {
        self.size()
    }
}
