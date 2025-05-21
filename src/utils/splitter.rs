use derivative::Derivative;

use crate::sdk_types::AkaveError;
#[cfg(target_arch = "wasm32")]
use crate::utils::seekable_web_file::{AsyncRead, SeekableWebFile as File};
use std::{
    cmp,
    io::{Read, Seek, SeekFrom},
};

use super::chunkable::Chunkable;
#[cfg(not(target_arch = "wasm32"))]
use super::file_size::FileSize;
#[cfg(not(target_arch = "wasm32"))]
use std::fs::File;

#[derive(Derivative)]
#[derivative(Debug)]
pub(crate) struct Splitter {
    #[derivative(Debug = "ignore")]
    file: File,
    counter: usize,
}

impl Chunkable for Splitter {
    async fn next_chunk(&mut self, chunk_size: usize) -> Option<Result<Box<[u8]>, AkaveError>> {
        let file_size = self.file.size() as usize;
        if self.counter >= file_size {
            return None;
        }

        self.file
            .seek(SeekFrom::Start(self.counter as u64))
            .expect("failed to seek to offset");

        let buf_size = cmp::min(chunk_size, file_size - self.counter);

        let array: Vec<u8> = vec![0; buf_size.try_into().unwrap()];
        let mut chunk_data = array.into_boxed_slice();

        #[cfg(target_arch = "wasm32")]
        {
            // Use the AsyncRead trait for WASM
            AsyncRead::read_async(&mut self.file, &mut chunk_data)
                .await
                .map_err(|e| {
                    let err_msg = format!("Failed to read file: {:?}", e);
                    std::io::Error::new(std::io::ErrorKind::Other, err_msg)
                })
                .expect("Failed to read the file");
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.file
                .read(&mut chunk_data)
                .expect("Failed to read the file");
        }
        self.counter += buf_size;
        Some(Ok(chunk_data))
    }

    fn data_size(&mut self) -> usize {
        self.file.size() as usize
    }
}

impl Splitter {
    /// Create a new FileChunker
    pub fn new(file: File) -> Self {
        return Self { file, counter: 0 };
    }
}
