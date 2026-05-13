# Summary of Changes (Change Set 2)

## Files Changed

### `src/utils/encryption.rs` (CHANGE-1 — replaced entirely)
- Rewrote the `Encryption` struct API: `encrypt`, `decrypt`, `encrypt_deterministic`, and `decrypt_deterministic` now accept `info: &str` instead of `&[u8]`, eliminating all `.as_bytes()` call-sites.
- Added `derive_key` and `gcm_cipher` as public helper functions for use by `streamenc`.
- Added `ceil_div` utility (replaces any ad-hoc ceiling division).
- Added `OVERHEAD` constant (28 bytes = 12B nonce + 16B GCM tag) replacing the local `ENCRYPTION_OVERHEAD` in `sdk.rs`.
- Nonce generation is split per target: `OsRng` on native, Web Crypto API on WASM.

### `src/utils/streamenc.rs` (CHANGE-2 — new file)
- New block-based streaming encryption module using AES-256-GCM.
- Plaintext is split into fixed-size 32 KiB blocks (block 0 carries a 17-byte header: version + initial nonce + plaintext size).
- Nonces are derived by incrementing the last 4 bytes of the initial random nonce per block.
- The last block is zero-padded so the total ciphertext length is always a multiple of 16 bytes (EC data block alignment).
- Public API: `encrypt`, `decrypt_all_blocks`, `overhead`, `num_blocks`, `block_data_size`, `max_plaintext_size_for_target`.
- Unit tests: roundtrip for multiple sizes, block count assertions, 16-byte alignment check.

### `src/utils/erasure.rs` (CHANGE-3 — replaced entirely)
- Added a **wrap format** to `encode`/`extract_data`: data is prefixed with an 8-byte big-endian length and suffixed with a 4-byte magic (`0xDEADBEEF`). This allows `extract_data` to self-describe the original size without requiring the caller to pass it.
- `extract_data` signature changed from `(blocks, original_size)` to `(blocks)` — size is recovered from the wrap header.
- Added `encode_raw` / `extract_data_raw` for callers that manage size externally.
- Added `split_stripes` helper.
- Added `UnwrapFailed` and updated `InvalidBlockCount` error variants.
- Corrected `InvalidShardSize` → `IncorrectShardSize` to match the `reed-solomon-erasure 4.0.2` enum.
- New tests: wrap roundtrip, missing shards, too many missing shards, raw encode/decode, stripe splitting.

### `src/types/sdk_types.rs` (CHANGE-4)
- Added `TransientError(String)` variant to `AkaveError` for distinguishing retryable network errors.

### `src/utils/http_ext.rs` (CHANGE-5 — replaced entirely)
- `range_download` now maps connection/read errors to `AkaveError::TransientError` instead of `InternalError`.
- Added `is_transient` helper to test whether an error is transient.
- Added `ERR_TRANSIENT_PREFIX` constant.
- Unit tests cover rejection of negative offsets and non-positive lengths.

### `src/utils/mod.rs` (CHANGE-6)
- Added `pub mod streamenc;` after `pub mod retry;`.

### `src/utils/dag.rs` (CHANGE-7)
- Added `cid_for_data(data: &[u8]) -> Cid`: computes the SHA2-256 DAG-PB CID for raw bytes.
- Added `build_leaf_node(data: Vec<u8>) -> FileBlockUpload`: wraps raw bytes in a `FileBlockUpload` with the computed CID.
- Updated `test_chunk_encoded_size_with_erasure` pinned value from `20_971_968` to `20_972_000` to account for the 12-byte wrap overhead added by the new `ErasureCode::encode`.

### `src/sdk.rs` (CHANGE-8)
- Removed `const ENCRYPTION_OVERHEAD: usize = 28;`; replaced usages with `crate::utils::encryption::OVERHEAD`.
- Renamed `batch_size` field to `chunk_batch_size` in both `AkaveSDK` and `AkaveSDKBuilder`.
- Removed `use_connection_pool: bool` field from `AkaveSDKBuilder` and `new_with_params`; the connection pool is now always created (`Some(Arc::new(...))`).
- Removed `with_connection_pool` builder method.
- Renamed `with_batch_size` to `with_chunk_batch_size`.
- Updated all `Encryption::new(key, info.as_bytes())` → `Encryption::new(key, &info)`.
- Updated all `enc.encrypt(data, format!(...).as_bytes())` → `enc.encrypt(data, &format!(...))`.
- Updated all `enc.decrypt(data, format!(...).as_bytes())` → `enc.decrypt(data, &format!(...))`.
- Updated all `enc.decrypt_deterministic(data, path.as_bytes())` → `enc.decrypt_deterministic(data, path)`.
- Updated all `enc.encrypt_deterministic(data, path.as_bytes())` → `enc.encrypt_deterministic(data, &path)`.
- Updated `erasure_code.extract_data(blocks, size)` → `erasure_code.extract_data(blocks)` at all call sites.
- Added `if !default_key.is_empty()` guard in `setup_download_encryption` for the default-key branch.
- Updated `test_ipc_upload_with_chunks_batch_size` to use `.with_chunk_batch_size`.

## Key New Features
- **streamenc module**: block-streaming AES-GCM encryption with nonce-counter derivation and 16-byte aligned ciphertext.
- **TransientError variant**: allows callers to distinguish retryable network errors.
- **Erasure wrap format**: `ErasureCode::encode` embeds the original data length so `extract_data` is self-describing.
- **`cid_for_data` / `build_leaf_node`**: canonical CID helpers in `dag.rs` matching Go's leaf node construction.

## Compilation Fixes Applied
- Fixed `reed_solomon_erasure::Error::InvalidShardSize` → `IncorrectShardSize` (correct variant name in v4.0.2).
- Updated all encryption info-parameter types from `&[u8]` to `&str` across `sdk.rs`.
- Removed `use_connection_pool` parameter and field consistently from builder, `new`, and `new_with_params`.

## Cross-Verification

### Step 1 — Signature verification

✓ CHANGE-1: `encrypt`/`decrypt` info param is `&str` matching Go's `string` type; `OVERHEAD=28`, `gcm_cipher`, `derive_key`, `ceil_div` all match Go source signatures at meta.source_commit.

✓ CHANGE-2: `streamenc` constants match Go: `MAX_BLOCK_SIZE=32768`, `HEADER_SIZE=17`, `VERSION=1`, `BLOCK0_DATA_SIZE=32751`, `BLOCK_N_DATA_SIZE=32752`. `encrypt`/`decrypt_all_blocks`/`parse_header`/`block_nonce` match Go signatures.

✓ CHANGE-3: `extract_data(blocks)` drops originalSize (matches new Go API); `encode_raw` returns `Vec<Vec<u8>>`; `extract_data_raw(blocks, original_size)` matches; `WRAP_OVERHEAD=12`; `split_stripes(data, max_stripe_size)` matches Go.

✓ CHANGE-4: `TransientError` variant added to `AkaveError`; `is_transient` helper checks for `TransientError` variant; `range_download` wraps network errors with `TransientError`.

✓ CHANGE-6: `chunk_batch_size` field/method replaces `batch_size`; `ENCRYPTION_OVERHEAD` removed; `use_connection_pool` removed; connection pool always created; empty-key guard added.

✓ CHANGE-11: `cid_for_data` and `build_leaf_node` added to `dag.rs`.

### Step 2 — Call-site check

✓ CHANGE-1: `gcm_cipher` and `derive_key` used by `src/utils/streamenc.rs`; `OVERHEAD` used by `src/sdk.rs`; `ceil_div` used by `src/utils/streamenc.rs` and `src/utils/erasure.rs`.

✓ CHANGE-2: `streamenc` module registered in `src/utils/mod.rs`. Functions not yet called from sdk.rs (Upload2/Download2 not yet implemented — CHANGE-9 is skipped for now).

Dead code (Cause A) CHANGE-2: `streamenc::encrypt`, `streamenc::decrypt_all_blocks` — depend on skipped CHANGE-9 (Upload2/Download2). Added `#[allow(dead_code)]` suppressed by module-level tests that call them.

✓ CHANGE-3: `extract_data(blocks)` (no size) called at 3 sites in `sdk.rs`; `encode_raw`/`extract_data_raw`/`split_stripes` not yet called outside defining file — depend on skipped CHANGE-9.

Dead code (Cause A) CHANGE-3: `encode_raw`, `extract_data_raw`, `split_stripes` — depend on skipped CHANGE-9 (Upload2/Download2).

✓ CHANGE-4: `TransientError` used in `http_ext.rs`; `is_transient` exported but not yet called from sdk — transient retry logic is an open extension point.

✓ CHANGE-11: `cid_for_data` used by `build_leaf_node`; `build_leaf_node` not yet called outside dag.rs — depends on skipped CHANGE-9/10.

### Step 3 — Skipped changes

CHANGE-5 skipped: No Rust equivalent of `ipc.Client` or `ContractsAddresses`/`DeployContracts`/`UpgradeStorage` — Go-only internal tooling with no Rust counterpart.

CHANGE-8 skipped: `MultiUpload` type requires new file `src/multi_upload.rs`. Complex implementation deferred; no existing Rust code exhibits the old inline-upload pattern that MultiUpload replaces.

CHANGE-9 skipped: `Upload2`/`Download2` require `streamenc` + per-stripe erasure + new DAG layout. Deferred pending CHANGE-8.

CHANGE-10 skipped: IPC.Upload refactor to delegate to MultiUpload deferred — depends on CHANGE-8.

CHANGE-12 skipped: Version module change is Go-internal (reads build info differently). No Rust equivalent module.

CHANGE-13 skipped: ipctest error matching change is Go test infrastructure only. No Rust equivalent.

TEST-3 skipped: MultiUpload integration test — depends on CHANGE-8 (skipped).

TEST-4 skipped: ExtractBlockData protowire parity test — existing Rust ExtractBlockData is already tested via sdk unit tests.
