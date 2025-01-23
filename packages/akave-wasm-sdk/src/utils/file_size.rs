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
        if self.sync_all().is_err() {
            return 0;
        }
        let meta = self.metadata();
        match meta {
            Ok(meta) => meta.len() as u64,
            Err(_) => 0,
        }
    }
}

#[cfg(target_arch = "wasm32")]
impl FileSize for File {
    fn size(&self) -> u64 {
        self.size()
    }
}
