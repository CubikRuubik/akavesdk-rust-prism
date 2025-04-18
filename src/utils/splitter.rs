use derivative::Derivative;

use std::{
    cmp,
    io::{Read, Seek, SeekFrom},
};
use crate::sdk_types::AkaveError;
#[cfg(target_arch = "wasm32")]
use crate::utils::seekable_web_file::{SeekableWebFile as File, AsyncRead};

#[cfg(not(target_arch = "wasm32"))]
use super::file_size::FileSize;
#[cfg(not(target_arch = "wasm32"))]
use std::fs::File;

#[derive(Derivative)]
#[derivative(Debug)]
pub(crate) struct Splitter {
    #[derivative(Debug = "ignore")]
    file: File,
    chunk_size: u64,
    counter: u64,
}

impl Splitter {
    /// Create a new FileChunker
    pub fn new(file: File, chunk_size: u64) -> Self {
        return Self {
            file,
            chunk_size,
            counter: 0,
        };
    }

    pub fn size(&self) -> usize {
        return u64::div_ceil(self.file.size(), self.chunk_size) as usize;
    }
}

impl Splitter {

    fn count(self) -> usize
    where
        Self: Sized,
    {
        return u64::div_ceil(self.file.size(), self.chunk_size) as usize;
    }

    pub async fn next_chunk(&mut self) -> Option<Result<Box<[u8]>, AkaveError>> {
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

        #[cfg(target_arch = "wasm32")]
        {
            // Use the AsyncRead trait for WASM
            AsyncRead::read_async(&mut self.file, &mut chunk_data)
                .await
                .map_err(|e| {
                    let err_msg = format!("Failed to read file: {:?}", e);
                    // log::error!("{}", err_msg);
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
}
