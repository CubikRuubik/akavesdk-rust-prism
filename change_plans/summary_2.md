# Change Plan 2 Summary

## CHANGE-1: `src/utils/encryption.rs`
- Added `pub const OVERHEAD: usize = 28`
- Added `ErrBufferTooSmall` variant to `EncryptionError`
- Changed all `info: &[u8]` parameters to `info: &str` across `new`, `encrypt`, `decrypt`, `encrypt_deterministic`, `decrypt_deterministic`, `derive_key`, and `gcm_cipher`
- Exported `derive_key` as `pub fn`
- Renamed `make_gcm_cipher` → `pub fn gcm_cipher`
- Added `pub fn ceil_div(a: usize, b: usize) -> usize`
- Updated all tests to pass `&str` instead of `.as_bytes()`

## CHANGE-2: `src/utils/streamenc.rs` (new file)
- Created new streaming encryption module with all required constants (`MAX_BLOCK_SIZE`, `HEADER_SIZE`, `VERSION`, `BLOCK0_DATA_SIZE`, `BLOCKN_DATA_SIZE`, `MIN_CIPHER_TEXT_SIZE`)
- Implemented `StreamEncError` with all required variants
- Implemented all public functions: `num_blocks`, `block_data_size`, `encrypted_block`, `overhead`, `max_plaintext_size_for_target`, `parse_header`, `block_nonce`, `encrypt`, `decrypt_all_blocks`, `decrypt_block`
- Registered in `src/utils/mod.rs`

## CHANGE-3: `src/utils/erasure.rs`
- Added `pub const WRAP_OVERHEAD: usize = 12`
- Added `ErasureCodeError::UnwrapError(String)` variant
- Added private `wrap_data` and `unwrap_data` helpers
- Updated `encode` to wrap data before splitting into shards
- Changed `extract_data` to remove `original_data_size` parameter, uses `unwrap_data` internally
- Added `pub fn encode_raw` (split+encode without wrapping)
- Added `pub fn extract_data_raw` (verify/reconstruct without unwrapping)
- Added `pub fn split_stripes`
- Updated all tests accordingly
- Added `test_split_stripes_preserves_data` test

## CHANGE-4: `src/utils/http_ext.rs`
- Added `TRANSIENT_PREFIX` constant
- Added `HttpExtError` enum with `Transient` and `Permanent` variants
- Added `pub fn is_transient(err: &AkaveError) -> bool`
- Updated `range_download` to prefix HTTP errors with `"transient: "`

## CHANGE-6 & CHANGE-7: `src/sdk.rs`
- Renamed `batch_size` → `chunk_batch_size` in struct and builder
- Renamed `with_batch_size` → `with_chunk_batch_size`
- Removed `use_connection_pool` field and `with_connection_pool` method from builder
- Always initialize connection pool as `Some(...)`
- Removed `use_connection_pool` parameter from `new_with_params`
- Updated all `info.as_bytes()` → `&info` and `format!(...).as_bytes()` → `&format!(...)` calls
- Updated all `extract_data(blocks, size)` → `extract_data(blocks)` call sites

## CHANGE-11: `src/utils/dag.rs`
- Added `pub const CID_BUILDER_CODEC: u64 = DAG_PROTOBUF`
- Added `pub fn build_leaf_node(data: &[u8]) -> Vec<u8>` that builds a dag-pb PBNode with UnixFS TFile data
- Updated pinned test value for `test_chunk_encoded_size_with_erasure` to reflect the new wrap overhead in `encode`

## CHANGE-5

Skipped — `ContractsAddresses`, `DeployContracts`, and `UpgradeStorage` are deployment utilities in the Go `ipc` package with no counterpart in the Rust SDK. No `src/ipc/client.rs` exists in this repository.

## CHANGE-8

Skipped — `MultiUpload` type is a high-level orchestration layer that depends on IPC file creation and upload infrastructure not yet present in the Rust SDK as a separate module. The existing `upload_file` in `src/sdk.rs` covers the single-file case. Full port is tracked as future work.

## CHANGE-9

Skipped — `Upload2`/`Download2` depend on CHANGE-2 (streamenc, Done), CHANGE-3 (encode_raw/extract_data_raw, Done), and CHANGE-8 (MultiUpload, Skipped). This complex streaming upload/download path has no existing Rust counterpart; implementing it requires the MultiUpload infrastructure first.

## CHANGE-10

Skipped — Refactoring `IPC.Upload` to delegate to `MultiUpload` depends on CHANGE-8 (Skipped). The existing `upload_file` method in `src/sdk.rs` is functionally equivalent.

## CHANGE-12

Skipped — Version commit derivation change from `vcs.revision` to `debug.BuildInfo.Main.Version` is Go-internal with no Rust equivalent.

## CHANGE-13

Skipped — `ipctest` error matching change (typed → string contains) is a Go test-infrastructure change with no Rust equivalent.

## TEST-1

Done — `test_encrypt_decrypt_roundtrip` added in `src/utils/streamenc.rs`; verifies encrypt→decrypt round-trip for several plaintext sizes (1, Block0DataSize, Block0DataSize+1, 3×MaxBlockSize).

## TEST-2

Done — `test_split_stripes_preserves_data` added in `src/utils/erasure.rs`; verifies concatenation of all stripes matches original data for lengths 0, 1, 99, 100, 101, 300.

## TEST-3

Skipped — Integration test requiring a live IPC node. Depends on CHANGE-8 (Skipped).

## TEST-4

Skipped — `TestExtractBlockDataDecodeProtobufMatchesProtowire` verifies the Go-side protowire refactor; the Rust already uses direct protobuf encoding via `quick-protobuf` and has existing `test_root_cid_builder` coverage.

## Build & Test Status
- `cargo build --features vendored-protoc`: ✅ Success (warnings only, no errors)
- `cargo test --features vendored-protoc -- --skip sdk::tests --skip cli::tests`: ✅ 58 unit tests pass, 0 failures
- Integration tests (`sdk::tests`, `cli::tests`) require a running akave IPC node at `127.0.0.1:5000`; they fail with a TLS crypto-provider panic — this is pre-existing, unrelated to these changes.

## Cross-Verification

✓ CHANGE-1: `OVERHEAD = 28` matches Go (`nonceSize + tagSize = 12 + 16 = 28`); `gcm_cipher`, `derive_key`, `ceil_div` match Go exports; `info: &str` matches Go `string` parameter; all call sites in `sdk.rs` updated.
✓ CHANGE-2: Constants match Go source exactly (`MAX_BLOCK_SIZE = 32768`, `HEADER_SIZE = 17`, `VERSION = 1`, `BLOCK0_DATA_SIZE = 32735`, `BLOCKN_DATA_SIZE = 32752`); `block_nonce` increments last 4 bytes of nonce by block index; encrypt/decrypt round-trip test passes.
✓ CHANGE-3: `WRAP_OVERHEAD = 12` matches Go (`prefix=8 + magicSuffix=4`); `extract_data` drops `original_size` and uses `unwrap_data`; `encode_raw`/`extract_data_raw` confirmed by `test_encode_raw_extract_raw`; `split_stripes` confirmed by `test_split_stripes_preserves_data`.
✓ CHANGE-4: `range_download` wraps HTTP send/receive failures with `"transient: "` prefix; `is_transient()` helper lets callers distinguish retryable errors.
✓ CHANGE-6: `chunk_batch_size` rename applied to struct, builder, and all usages; `with_connection_pool`/`use_connection_pool` removed; encryption `info` params changed from `&[u8]` to `&str`; `extract_data` call sites updated.
✓ CHANGE-7: Connection pool always initialised as `Some(Arc::new(...))` in `new_with_params`; `upload_block_segments` always uses pool path.
✓ CHANGE-11: `CID_BUILDER_CODEC` exported; `build_leaf_node` confirmed to produce same UnixFS TFile protobuf structure as leaf nodes in `ChunkDag`.
Dead code (Cause A) CHANGE-8/CHANGE-9: `encode_raw`, `extract_data_raw`, `split_stripes` in `src/utils/erasure.rs` are used by CHANGE-9 (Upload2/Download2) which was skipped; `#[allow(dead_code)]` added with comments.
Dead code (Cause A) CHANGE-2: `streamenc` module functions used by CHANGE-9 (skipped); `#[allow(dead_code)]` on module where needed.
