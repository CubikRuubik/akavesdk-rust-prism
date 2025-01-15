use derivative::Derivative;
#[cfg(not(target_arch = "wasm32"))]
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_file_reader::WebSysFile as File;

const BLOCK_SIZE: u64 = 1024 * 1024; // 1MB blocks

#[derive(Derivative)]
#[derivative(Debug)]
pub struct FileChunker {
    #[derivative(Debug = "ignore")]
    file: File,
    chunk_size: u64,
    last_pos: u64,
}

impl FileChunker {
    /// Create a new FileChunker
    pub fn new(file: File, chunk_size: Option<u64>) -> Self {
        if chunk_size.is_some() {
            return Self {
                file,
                chunk_size: chunk_size.unwrap(),
                last_pos: 0,
            };
        } else {
            return Self {
                file,
                chunk_size: BLOCK_SIZE,
                last_pos: 0,
            };
        }
    }
}

impl Iterator for FileChunker {
    type Item = Box<[u8]>;

    fn count(self) -> usize
    where
        Self: Sized,
    {
        return u64::div_ceil(self.file.size(), self.chunk_size) as usize;
    }

    fn next(&mut self) -> Option<Self::Item> {
        let file_size = self.file.size();
        if self.last_pos >= file_size {
            return None;
        }

        self.file
            .seek(SeekFrom::Start(self.last_pos))
            .expect("failed to seek to offset");
        self.last_pos += self.chunk_size;

        let buf_size = if self.last_pos > file_size {
            self.last_pos - file_size
        } else {
            self.chunk_size
        };

        let array: Vec<u8> = vec![0; buf_size.try_into().unwrap()];
        let mut chunk_data = array.into_boxed_slice();
        self.file
            .read(&mut chunk_data)
            .expect("Failed to read the file");
        Some(chunk_data)
    }
}
