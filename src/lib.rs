mod allocator;
pub mod blockchain;
pub mod logger;
pub mod sdk;
pub mod types;
pub mod utils;

// Re-export main SDK types for convenience
pub use sdk::{AkaveSDK, AkaveSDKBuilder};

#[cfg(target_arch = "wasm32")]
pub mod wasm;

#[cfg(target_arch = "wasm32")]
pub use wasm::{AkaveWebSDK, AkaveWebSDKBuilder};

#[cfg(target_arch = "wasm32")]
mod panic_handler;

#[cfg(not(target_arch = "wasm32"))]
fn get_nonce() -> web3::types::U256 {
    use std::time::{SystemTime, UNIX_EPOCH};

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_micros();
    web3::types::U256::from(timestamp)
}

#[cfg(target_arch = "wasm32")]
fn get_nonce() -> web3::types::U256 {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = Date)]
        fn now() -> f64;
    }

    // Get timestamp in milliseconds from JavaScript's Date.now()
    // and convert to microseconds
    let timestamp = (now() * 1000.0) as u128;
    web3::types::U256::from(timestamp)
}
