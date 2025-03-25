#[macro_use(shards)]
// #[cfg(not(target_arch = "wasm32"))]
extern crate reed_solomon_erasure;

mod allocator;
mod blockchain;
mod sdk;
mod utils;

#[cfg(target_arch = "wasm32")]
mod wasm;

#[cfg(target_arch = "wasm32")]
mod panic_handler;


mod sdk_types;

pub mod logger;
