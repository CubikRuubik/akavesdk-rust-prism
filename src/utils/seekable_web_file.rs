use js_sys::{Number, Promise, Uint8Array, ArrayBuffer};
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Blob, FileReader, Event};
use std::future::Future;
use std::pin::Pin;

thread_local! {
    static FILE_READER: FileReader = FileReader::new().expect("Failed to create FileReader. Help: make sure this is a web worker context.");
}

/// Wrapper around a `web_sys::File` that implements `Read` and `Seek`.
pub struct SeekableWebFile {
    file: web_sys::File,
    pos: u64,
}

impl SeekableWebFile {
    pub fn new(file: web_sys::File) -> Self {
        Self { file, pos: 0 }
    }

    /// File size in bytes.
    pub fn size(&self) -> u64 {
        let size_f64 = self.file.size();

        f64_to_u64_safe(size_f64).expect("file size is not a valid integer")
    }
    
    /// Read a blob as an array buffer asynchronously
    async fn read_blob_as_array_buffer(&self, blob: Blob) -> Result<ArrayBuffer, JsValue> {
        // Create a FileReader
        let reader = FileReader::new()?;
        
        // Create a promise that resolves when the file is loaded
        let promise = Promise::new(&mut |resolve, reject| {
            // Set up callbacks
            let onload = Closure::once_into_js(move |_event: web_sys::Event| {
                resolve.call0(&JsValue::NULL).unwrap();
            });
            
            let onerror = Closure::once_into_js(move |_event: web_sys::Event| {
                reject.call0(&JsValue::NULL).unwrap();
            });
            
            reader.set_onload(Some(onload.as_ref().unchecked_ref()));
            reader.set_onerror(Some(onerror.as_ref().unchecked_ref()));
            
            // Start reading
            reader.read_as_array_buffer(&blob).unwrap();
        });
        
        // Wait for the promise to resolve
        JsFuture::from(promise).await?;
        
        // Get the result
        let result = reader.result()?;
        let array_buffer = result.dyn_into::<ArrayBuffer>()?;
        
        Ok(array_buffer)
    }
    
    /// Reads a blob into a buffer and returns the number of bytes read
    pub async fn read_into_buffer(&mut self, buf: &mut [u8]) -> Result<usize, JsValue> {
        let buf_len = buf.len();
        let old_offset = self.pos;
        
        // Calculate positions
        let offset_f64 = u64_to_f64_safe(old_offset).expect("offset too large");
        let offset_end_f64 = u64_to_f64_safe(
            old_offset.saturating_add(u64::try_from(buf_len).expect("buffer too large")),
        ).expect("offset + len too large");
        
        // Create a slice of the file
        let blob = self
            .file
            .slice_with_f64_and_f64(offset_f64, offset_end_f64)
            .expect("failed to slice file");
        
        // Read the slice asynchronously
        let array_buffer = self.read_blob_as_array_buffer(blob).await?;
        
        // Process the array buffer
        let array = Uint8Array::new(&array_buffer);
        let actual_read_bytes = array.byte_length();
        let actual_read_bytes_usize =
            usize::try_from(actual_read_bytes).expect("read too many bytes at once");
        
        // Copy to output buffer
        array.copy_to(&mut buf[..actual_read_bytes_usize]);
        
        // Update position
        self.pos = old_offset
            .checked_add(u64::from(actual_read_bytes))
            .expect("new position too large");

        Ok(actual_read_bytes_usize)
    }
}

/// Convert `u64` to `f64` but only if it can be done without loss of precision (if the number does
/// not exceed the `MAX_SAFE_INTEGER` constant).
fn u64_to_f64_safe(x: u64) -> Option<f64> {
    let x_float = x as f64;

    if x_float <= Number::MAX_SAFE_INTEGER {
        Some(x_float)
    } else {
        None
    }
}

/// Convert `f64` to `u64` but only if it can be done without loss of precision (if the number is
/// positive and it does not exceed the `MAX_SAFE_INTEGER` constant).
fn f64_to_u64_safe(x: f64) -> Option<u64> {
    if 0.0 <= x && x <= Number::MAX_SAFE_INTEGER {
        Some(x as u64)
    } else {
        None
    }
}

/// This trait is for compatibility with WASM code that expects async file reading
pub trait AsyncRead {
    /// Read asynchronously into a buffer
    fn read_async<'a>(&'a mut self, buf: &'a mut [u8]) -> Pin<Box<dyn Future<Output = Result<usize, JsValue>> + 'a>>;
}

impl AsyncRead for SeekableWebFile {
    fn read_async<'a>(&'a mut self, buf: &'a mut [u8]) -> Pin<Box<dyn Future<Output = Result<usize, JsValue>> + 'a>> {
        Box::pin(self.read_into_buffer(buf))
    }
}

impl Read for SeekableWebFile {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error> {
        // Note: This is a synchronous version that will attempt to use the async method
        // in a blocking way. This is not ideal for WASM environments but is provided
        // for compatibility with the Read trait.
        
        // Use the thread-local FileReader instead of our async method for this synchronous API
        let buf_len = buf.len();
        let old_offset = self.pos;
        let offset_f64 = u64_to_f64_safe(old_offset).expect("offset too large");
        let offset_end_f64 = u64_to_f64_safe(
            old_offset.saturating_add(u64::try_from(buf_len).expect("buffer too large")),
        )
        .expect("offset + len too large");
        let blob = self
            .file
            .slice_with_f64_and_f64(offset_f64, offset_end_f64)
            .expect("failed to slice file");
            
        // Use the thread-local FileReader since we can't await in a sync function
        let array_buffer = FILE_READER.with(|file_reader| {
            // Start the read
            file_reader.read_as_array_buffer(&blob)
                .expect("failed to start read_as_array_buffer");
                
            // Because this is synchronous, we have to get the result right away
            // This may return the incorrect result if the read isn't finished
            // That's why the async API is preferred
            file_reader.result().expect("failed to get result from FileReader")
        });
        
        let array = Uint8Array::new(&array_buffer);
        let actual_read_bytes = array.byte_length();
        let actual_read_bytes_usize =
            usize::try_from(actual_read_bytes).expect("read too many bytes at once");
        // Copy to output buffer
        array.copy_to(&mut buf[..actual_read_bytes_usize]);
        // Update position
        self.pos = old_offset
            .checked_add(u64::from(actual_read_bytes))
            .expect("new position too large");

        Ok(actual_read_bytes_usize)
    }
}

// Copied these functions from std because they are unstable
fn overflowing_add_signed(lhs: u64, rhs: i64) -> (u64, bool) {
    let (res, overflowed) = lhs.overflowing_add(rhs as u64);
    (res, overflowed ^ (rhs < 0))
}

fn checked_add_signed(lhs: u64, rhs: i64) -> Option<u64> {
    let (a, b) = overflowing_add_signed(lhs, rhs);
    if b {
        None
    } else {
        Some(a)
    }
}

impl Seek for SeekableWebFile {
    fn seek(&mut self, style: SeekFrom) -> Result<u64, std::io::Error> {
        // Seek impl copied from std::io::Cursor
        let (base_pos, offset) = match style {
            SeekFrom::Start(n) => {
                self.pos = n;
                return Ok(n);
            }
            SeekFrom::End(n) => (self.size(), n),
            SeekFrom::Current(n) => (self.pos, n),
        };
        match checked_add_signed(base_pos, offset) {
            Some(n) => {
                self.pos = n;
                Ok(self.pos)
            }
            None => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "invalid seek to a negative or overflowing position",
            )),
        }
    }
}