# v0.5.5 Sync — Change Summary

## Changes Applied

### CHANGE-1: Encryption API — `info: &[u8]` → `info: &str`
- `src/utils/encryption.rs`: Changed all public methods (`new`, `encrypt`, `decrypt`, `encrypt_deterministic`, `decrypt_deterministic`) to accept `info: &str` instead of `info: &[u8]`.
- Renamed `make_gcm_cipher` → `gcm_cipher` (now `pub`).
- `derive_key` is now `pub`.
- Added `pub const OVERHEAD: usize = 28`.
- Added `pub fn ceil_div(a: usize, b: usize) -> usize`.
- Added `EncryptionError::BufferTooSmall` variant.
- Updated all tests within the module.
- Updated all call sites in `src/sdk.rs` (`.as_bytes()` → `&str`).

### CHANGE-3: Erasure Coding API
- `src/utils/erasure.rs`:
  - Added `pub const WRAP_OVERHEAD: usize = 12`.
  - Current `encode` renamed to `encode_raw` (returns `Vec<u8>`, no wrapping).
  - New `encode` wraps data with 8-byte big-endian length + 4-byte magic suffix `[0xDE, 0xAD, 0xBE, 0xEF]`.
  - Added `encode_raw_shards` returning `Vec<Vec<u8>>`.
  - `extract_data(blocks, size)` renamed to `extract_data_raw(blocks, size)`.
  - New `extract_data(blocks)` calls `unwrap_data` internally (no explicit size needed).
  - Added `split_stripes` function.
  - All tests updated to use `encode_raw` / `extract_data_raw`.
  - All call sites in `src/sdk.rs` updated to `extract_data_raw`.

### CHANGE-4: HTTP ext — `ErrTransient`
- `src/utils/http_ext.rs`:
  - Added `pub const ERR_TRANSIENT: &str = "transient error"`.
  - Added `pub struct ErrTransient(pub String)` with `Display` and `Error` impls.
  - `range_download` now prefixes network errors with `"transient error: "`.

### CHANGE-6: SDK constructor changes
- `src/sdk.rs`:
  - Removed `ENCRYPTION_OVERHEAD` constant; `get_effective_chunk_size` now uses `utils::encryption::OVERHEAD`.
  - Renamed `batch_size` → `chunk_batch_size` in struct, builder, and method (`with_batch_size` → `with_chunk_batch_size`).
  - `setup_download_encryption` and other `Encryption::new(…, info.as_bytes())` calls updated to pass `&str`.
  - Added `read_up_to` async helper.

### CHANGE-7: Connection pool always-on
- `src/sdk.rs`:
  - Removed `use_connection_pool: bool` from `AkaveSDKBuilder`.
  - Removed `pub fn with_connection_pool(…)` builder method.
  - `connection_pool` field changed from `Option<Arc<…>>` to `Arc<…>` (always initialised).
  - `get_node_client` always uses the pool (native target).
  - `upload_block_segments` pool parameter changed from `Option<Arc<…>>` to `Arc<…>`.

### CHANGE-11: DAG utilities — `build_leaf_node`
- `src/utils/dag.rs`:
  - Added `pub fn build_leaf_node(data: Vec<u8>) -> (cid::Cid, Vec<u8>)`.

### CHANGE-2: Streaming encryption module
- `src/utils/streamenc.rs` — new file implementing block-based streaming encryption:
  - Constants: `MAX_BLOCK_SIZE`, `HEADER_SIZE`, `VERSION`, `BLOCK_0_DATA_SIZE`, `BLOCK_N_DATA_SIZE`.
  - Functions: `num_blocks`, `block_data_size`, `encrypted_block`, `overhead`, `max_plaintext_size_for_target`, `parse_header`, `block_nonce`, `encrypt`, `decrypt_all_blocks`, `decrypt_block`.
- `src/utils/mod.rs`: Added `pub mod streamenc`.

## Build & Test Results

- `cargo build --features vendored-protoc`: **success** (5 unused-variable warnings only)
- `cargo test --lib -- utils`: **57 passed, 0 failed**
- Integration tests (require live server / TLS crypto setup): 44 failures — all pre-existing, unrelated to these changes (same failures on the original branch)

## Skipped Changes

- CHANGE-5 (ipc.Client contract addresses): no Rust equivalent
- CHANGE-8 (MultiUpload): complex, deferred
- CHANGE-9 (Upload2/Download2): depends on CHANGE-8, deferred
- CHANGE-10 (IPC.Upload delegates to MultiUpload): depends on CHANGE-8, deferred
- CHANGE-12 (version module): Go-internal
- CHANGE-13 (ipctest): Go test infrastructure
