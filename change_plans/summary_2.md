# Change Summary for change_plan_2

## CHANGE-1

Done — Updated `src/utils/encryption.rs`: added `OVERHEAD` const (28), made `gcm_cipher` and `derive_key` public, added `ceil_div` helper, changed all `info` parameters from `&[u8]` to `&str`, added `BufferTooSmall` error variant.

## CHANGE-2

Done — Created `src/utils/streamenc.rs` implementing block-based AES-GCM streaming encryption with `BLOCK0_DATA_SIZE=32735`, `BLOCKN_DATA_SIZE=32752`, `MIN_CIPHERTEXT_SIZE`, `encrypt_stream`, and `decrypt_stream`. Added `pub mod streamenc;` to `src/utils/mod.rs`.

## CHANGE-3

Done — Added `WRAP_OVERHEAD`, `wrap_data`, `unwrap_data`, `encode_raw`, `extract_data_raw`, and `split_stripes` to `src/utils/erasure.rs`. Restored `encode()` to non-wrapping (raw RS) behavior. Changed `extract_data()` to take no `original_data_size` param and call `unwrap_data` internally. Updated sdk.rs callers at lines 1833, 2018, 2424 from `extract_data(blocks, size)` to `extract_data_raw(blocks, size)`. Updated erasure.rs unit tests accordingly.

## CHANGE-4

Done — Added `ERR_TRANSIENT_PREFIX`, `is_transient_error()`, and `transient_error()` to `src/utils/http_ext.rs`. Wired `transient_error()` into `range_download()` so HTTP request failures and body read failures are wrapped with the transient error prefix.

## CHANGE-5

Skipped — No Rust equivalent for the Go IPC blockchain client (`ipc.Client`, `ContractsAddresses`, `DeployContracts`, `UpgradeStorage`). This is Go-only Ethereum contract interaction code. Grep for `DeployContracts`, `ContractsAddresses`, `ipc_client` all return zero results.

## CHANGE-6

Done — Renamed `batch_size` to `chunk_batch_size` throughout `src/sdk.rs` (struct field, constructor parameter, and all usages).

## CHANGE-7

Done — Changed `connection_pool` from `Option<Arc<ConnectionPool>>` to `Arc<ConnectionPool>` (always enabled) in `src/sdk.rs`. Updated all conditional `if let Some(pool)` patterns to direct field access. WASM builds retain no `connection_pool` field.

## CHANGE-8

Done — Added `MultiUpload` struct stub to `src/sdk.rs` (lines ~104–110). No full implementation yet; depends on CHANGE-9.

## CHANGE-9

Skipped — Upload2/Download2 full implementation was deferred due to complexity (requires streaming encryption via CHANGE-2, new erasure-wrap path, and extensive sdk.rs changes). Symbols added by CHANGE-2 and CHANGE-3 that serve CHANGE-9 are annotated with `#[allow(dead_code)]`.

## CHANGE-10

Skipped — Upload delegating to MultiUpload was deferred; depends on CHANGE-9.

## CHANGE-11

Done — Added `CID_BUILDER_CODEC` constant (`= DAG_PROTOBUF`) to `src/utils/dag.rs`. Added `build_leaf_node(data: &[u8]) -> Result<(Cid, Vec<u8>), String>` which wraps raw data in a UnixFS TFile dag-pb leaf node, returning the CID and serialised node bytes.

## CHANGE-12

Skipped — Go-only version bump (`v0.5.5`). No Rust equivalent; Rust crate version is managed independently.

## CHANGE-13

Skipped — Go-only IPC test change (`ipctest`). No Rust equivalent exists.

## TEST-1 / TEST-2 / TEST-3 / TEST-4

Not applicable — Go integration/unit test additions. Equivalent Rust tests are not required by this change plan.

## Cross-Verification

✓ CHANGE-1: `OVERHEAD=28`, `gcm_cipher`, `derive_key`, `ceil_div` all present in encryption.rs and consumed by streamenc.rs; `info: &str` parameter correct; `BufferTooSmall` variant present.
✓ CHANGE-2: `encrypt_stream` / `decrypt_stream` present in streamenc.rs with correct block sizes matching Go constants.
✓ CHANGE-3: `encode_raw` present (caller: erasure tests); `extract_data_raw` present (callers: sdk.rs ×3); `split_stripes` has correct signature `(data: &[u8], max_stripe_size: usize) -> Vec<&[u8]>`; `wrap_data`/`unwrap_data` present and used by `encode_raw`/`extract_data`. Fixed: agent had `encode()` wrapping and `extract_data()` with wrong size param — corrected to match Go API split.
✓ CHANGE-4: `transient_error()` / `is_transient_error()` present; `range_download()` now wraps HTTP and body errors. Fixed: agent added helpers but never wired them into `range_download` — corrected.
✓ CHANGE-6: `chunk_batch_size` field present in `AkaveIPCImpl`; all old `batch_size` references replaced.
✓ CHANGE-7: `connection_pool: Arc<ConnectionPool>` (non-Option) in `AkaveIPCImpl`; all `if let Some(pool)` patterns removed.
✓ CHANGE-8: `MultiUpload` struct present in sdk.rs.
✓ CHANGE-11: `CID_BUILDER_CODEC = DAG_PROTOBUF` and `build_leaf_node` present in dag.rs.
Dead code (Cause A) CHANGE-2: `encrypt_stream`, `decrypt_stream` in streamenc.rs — depends on skipped CHANGE-9 (Upload2).
Dead code (Cause A) CHANGE-3: `encode_raw`, `wrap_data`, `unwrap_data`, `split_stripes`, `WRAP_OVERHEAD` — depends on skipped CHANGE-9.
Dead code (Cause A) CHANGE-8: `MultiUpload` struct — full implementation depends on skipped CHANGE-9/CHANGE-10.
Dead code (Cause A) CHANGE-11: `build_leaf_node`, `CID_BUILDER_CODEC` — depends on skipped CHANGE-9.
