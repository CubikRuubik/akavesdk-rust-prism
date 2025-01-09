use async_trait::async_trait;
use js_sys::Uint8Array;
use std::{error::Error, fmt::format};
use std::io::Read;
use wasm_bindgen::{prelude::wasm_bindgen, JsValue};
use wasm_bindgen_futures::JsFuture;

#[async_trait(?Send)]
pub trait FileReader: Send + Sync {
    async fn read_file(&self, path: &str) -> Result<Vec<u8>, Box<dyn Error>>;
}

#[cfg(target_arch = "wasm32")]
pub struct WasmFileReader;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = fs)]
    fn read_file_js(path: &str) -> js_sys::Promise;
}

#[cfg(target_arch = "wasm32")]
#[async_trait(?Send)] // ?Send is the magic word
impl FileReader for WasmFileReader {
    async fn read_file(&self, path: &str) -> Result<Vec<u8>, Box<dyn Error>> {
        // Check for global 'fs' support
        let global = js_sys::global();
        let has_fs = js_sys::Reflect::has(&global, &JsValue::from_str("fs"))
            .map_err(|_| "Error checking for 'fs' in the JavaScript environment. Ensure Node.js is being used.")?;

        if !has_fs {
            return Err("'fs' package is not available in the JavaScript environment".into());
        }

        // Call JavaScript's `fs.read_file`
        let promise = read_file_js(path);

        let js_value = JsFuture::from(promise)
            .await
            .map_err(|err| format!("JavaScript error: {:?}", err.as_string().unwrap_or("Unknown error".to_string())))?;

        let uint8_array = Uint8Array::new(&js_value);
        let mut data = vec![0; uint8_array.length() as usize];
        uint8_array.copy_to(&mut data[..]);

        Ok(data)
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub struct NativeFileReader;

#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
impl FileReader for NativeFileReader {
    async fn read_file(&self, path: &str) -> Result<Vec<u8>, Box<dyn Error>> {
        let mut file = std::fs::File::open(path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        Ok(buffer)
    }
}

pub fn create_reader() -> impl FileReader + Send + Sync {
    #[cfg(target_arch = "wasm32")]
    {
        WasmFileReader
    }
    
    #[cfg(not(target_arch = "wasm32"))]
    {
        NativeFileReader
    }
}
