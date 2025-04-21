pub mod ipc_types;
pub mod provider;
pub mod eip712_types;
pub mod eip712_utils;

#[cfg(not(target_arch = "wasm32"))]
pub mod eip712;