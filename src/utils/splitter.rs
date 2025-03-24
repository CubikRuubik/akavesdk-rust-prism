use derivative::Derivative;

use std::{
    cmp,
    io::{Read, Seek, SeekFrom},
};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_file_reader::WebSysFile as File;

use crate::log_info;

use super::encryption::Encryption;

#[cfg(not(target_arch = "wasm32"))]
use std::fs::File;
#[cfg(not(target_arch = "wasm32"))]
use super::file_size::FileSize;

#[derive(Derivative)]
#[derivative(Debug)]
pub(crate) struct Splitter {
    #[derivative(Debug = "ignore")]
    file: File,
    chunk_size: u64,
    counter: u64,
    encryption: Option<Encryption>,
}

impl Splitter {
    /// Create a new FileChunker
    pub fn new(file: File, chunk_size: u64, encryption: Option<Encryption>) -> Self {
        return Self {
            file,
            chunk_size,
            counter: 0,
            encryption,
        };
    }

    pub fn size(&self) -> usize {
        return u64::div_ceil(self.file.size(), self.chunk_size) as usize;
    }
}

impl Iterator for Splitter {
    type Item = Result<Box<[u8]>, Box<dyn std::error::Error>>;

    fn count(self) -> usize
    where
        Self: Sized,
    {
        return u64::div_ceil(self.file.size(), self.chunk_size) as usize;
    }

    fn next(&mut self) -> Option<Self::Item> {
        let file_size = self.file.size();
        if self.counter >= file_size {
            return None;
        }

        self.file
            .seek(SeekFrom::Start(self.counter))
            .expect("failed to seek to offset");

        let buf_size = cmp::min(self.chunk_size, file_size - self.counter);

        let array: Vec<u8> = vec![0; buf_size.try_into().unwrap()];
        let mut chunk_data = array.into_boxed_slice();
        self.file
            .read(&mut chunk_data)
            .expect("Failed to read the file");

        let encrypted_data = match &self.encryption {
            Some(some_encryption) => {
                let info = format!("block_{}", self.counter);
                Some(some_encryption.encrypt(&chunk_data, info.as_bytes()))
            }
            None => Some(Ok(chunk_data)),
        };
        self.counter += buf_size;
        encrypted_data
    }
}
