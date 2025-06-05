pub mod eip712_types;
pub mod eip712_utils;
pub mod ipc_types;
pub mod provider;

#[cfg(not(target_arch = "wasm32"))]
pub mod eip712;
