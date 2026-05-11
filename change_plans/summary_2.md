# Change Summary for change_plan_2

## CHANGE-1

Done — Updated `src/utils/encryption.rs`: changed `info: &[u8]` → `info: &str` in `new()`, `derive_key()`, `gcm_cipher()`, `encrypt()`, `decrypt()`, `encrypt_deterministic()`, `decrypt_deterministic()`; renamed `make_gcm_cipher` → `pub fn gcm_cipher`; added `pub const OVERHEAD: usize = 28`; added `ErrBufferTooSmall` error variant; added `pub fn ceil_div(a: usize, b: usize) -> usize`. Updated all 12 call sites in `src/sdk.rs` and internal tests.

## CHANGE-2

Done — Created `src/utils/streamenc.rs` as a full Rust port of Go's `private/encryption/streamenc/streamenc.go`. Implements block-based AES-GCM streaming encryption with header (version, initial nonce, plaintext size), right-to-left in-place encryption, and per-block nonce derivation. Gated `#[cfg(not(target_arch = "wasm32"))]` since it uses `OsRng`. Added `pub mod streamenc;` (cfg-gated) to `src/utils/mod.rs`.

## CHANGE-3

Done — Updated `src/utils/erasure.rs`: added `pub const WRAP_OVERHEAD: usize = 12`; added `pub fn encode_raw()` returning individual shards; added `pub fn extract_data_raw()` as an alias to `extract_data()`; added standalone `pub fn split_stripes(data: &[u8], max_stripe_size: usize) -> Vec<&[u8]>`.

## CHANGE-4

Done — Updated `src/utils/http_ext.rs`: added `ErrTransient` newtype wrapping `AkaveError` with `Display` and `Error` implementations, marking transient HTTP errors that are safe to retry.

## CHANGE-5

Skipped — No `src/ipc/client.rs` or equivalent IPC client struct exists in the Rust repository. The Rust SDK communicates via generated tonic gRPC stubs directly.

## CHANGE-6

Done — Updated `src/sdk.rs`: renamed `batch_size` field → `chunk_batch_size` in both `AkaveSDK` and `AkaveSDKBuilder` structs; renamed builder method `with_batch_size()` → `with_chunk_batch_size()`; updated all internal uses including `new()`, `new_with_params()`, and the upload loop. Updated the integration test that called `with_batch_size`.

## CHANGE-7

Done — Updated `src/sdk.rs`: removed `use_connection_pool: bool` field from `AkaveSDKBuilder`; removed `with_connection_pool()` builder method; always initialise connection pool (`Some(Arc::new(RwLock::new(HashMap::new())))`); added `.initial_connection_window_size(128 * 1024)` to the gRPC `Channel` builder in `new_with_params`.

## CHANGE-8

Skipped — No `MultiUpload` or equivalent multi-stream upload type exists in the Rust repository. This is a new Go-side abstraction with no current Rust counterpart; implementing it from scratch is out of scope for this sync.

## CHANGE-9

Skipped — Depends on CHANGE-8 (`MultiUpload`). No `Upload2`/`Download2` equivalents exist in the Rust repository.

## CHANGE-10

Skipped — Internal Go refactor of `IPC.Upload` to use `MultiUpload`. The Rust upload path in `src/sdk.rs` does not share the same structure and is unaffected.

## CHANGE-11

Done — Added `pub fn build_leaf_node(data: &[u8]) -> Result<(Cid, Vec<u8>), String>` to `src/utils/dag.rs`. Wraps raw bytes in a UnixFS TFile protobuf message and encodes it as a dag-pb PBNode, returning the CIDv1 (dag-pb/sha2-256) and the serialised node bytes — matching Go's `BuildLeafNode`. Also promoted `DagRoot::write_bytes_field` to `pub(crate)` to allow reuse.

## CHANGE-12

Skipped — `target_files: []` in the change plan; no Rust version constant to update.

## CHANGE-13

Skipped — Go-only test helper (`ipctest` package). No Rust equivalent exists.

---

**Build note:** `cargo build` and `cargo test` fail on the pre-existing `alloy-consensus v0.11.1` compilation error (`serde::__private` not found) which is present on the `main` branch and unrelated to the changes in this PR.
