mod sdk;
mod utils;

#[cfg(target_arch = "wasm32")]
mod wasm;

#[cfg(target_arch = "wasm32")]
mod panic_handler;

#[cfg(target_arch = "wasm32")]
mod blockchain;
