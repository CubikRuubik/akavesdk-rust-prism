
#[cfg(not(target_arch = "wasm32"))]
mod native_imports {
    pub use std::fs::{File, OpenOptions};
    pub use std::io::{self, Write};
}

#[cfg(not(target_arch = "wasm32"))]
use native_imports::*;



#[cfg(target_arch = "wasm32")]
mod wasm_imports {
    pub use wasm_bindgen::prelude::*;
    pub use web_sys::{Blob, Url};
    pub use js_sys::Uint8Array;
}

#[cfg(target_arch = "wasm32")]
use wasm_imports::*;


#[cfg(not(target_arch = "wasm32"))]
pub struct Destination {
    file: File,
}

#[cfg(target_arch = "wasm32")]
pub struct Destination {
    buffer: Vec<u8>,
    path: String,
}

impl Destination {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new(destination_path: &str, file_name: &str) -> io::Result<Self> {
        let full_path = format!("{}{}", destination_path, file_name);
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .open(full_path)?;
        Ok(Destination {
            file: file,
        })
    }

    #[cfg(target_arch = "wasm32")]
    pub fn new(destination_path: &str, file_name: &str) -> Self {
        let full_path = format!("{}{}", destination_path, file_name);
        Destination {
            buffer: Vec::new(),
            path: full_path,
        }
    }

    pub fn write(&mut self, data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.file.write_all(data)?;
        }

        #[cfg(target_arch = "wasm32")]
        {
            self.buffer.extend_from_slice(data);
        }
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn flush(&mut self) -> io::Result<()> {
        self.file.flush()?;
        Ok(())
    }

    #[cfg(target_arch = "wasm32")]
    pub fn flush(&mut self) -> Result<(), JsValue> {
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn finalize(&mut self) -> io::Result<()> {
        self.file.flush()?;
        Ok(())
    }

    #[cfg(target_arch = "wasm32")]
    pub fn finalize(&mut self) -> Result<(), JsValue> {
        // Convert Vec<u8> to a Uint8Array
        let uint8_array = Uint8Array::new_with_length(self.buffer.len() as u32);
        uint8_array.copy_from(&self.buffer);

        // Create a Blob from the Uint8Array
        let blob = Blob::new_with_u8_array_sequence(&JsValue::from(&uint8_array))?;

        // Create a Blob from the Uint8Array
        let blob = Blob::new_with_u8_array_sequence(&JsValue::from(&uint8_array))?;
        let url = Url::create_object_url_with_blob(&blob)?;

        // Optionally, you can create a download link to trigger the browser's download dialog
        let link = web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .create_element("a")?;
        link.set_attribute("href", &url)?;
        link.set_attribute("download", &self.path)?;
        link.dyn_into::<web_sys::HtmlElement>()?.click();

        // Revoke the URL after the download
        Url::revoke_object_url(&url)?;

        Ok(())
    }
}
