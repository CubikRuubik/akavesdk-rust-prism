# Change Summary for change_plan_2

## CHANGE-1

Done — Updated `src/utils/encryption.rs`: changed `info` parameter from `&[u8]` to `&str` in
`Encryption::new`, `derive_key`, `make_gcm_cipher`, `encrypt`, `decrypt`, `encrypt_deterministic`,
`decrypt_deterministic`. Added `pub const OVERHEAD: usize = 28`, `ErrBufferTooSmall` error variant,
standalone `pub fn gcm_cipher(origin_key: &[u8], info: &str)`, and `pub fn ceil_div(a, b)`.
Updated all call sites in `src/sdk.rs` accordingly.

## CHANGE-2

Done — Created new `src/utils/streamenc.rs` module for block-based streaming AES-GCM encryption.
Added `pub mod streamenc` to `src/utils/mod.rs`. Constants: `MAX_BLOCK_SIZE`, `HEADER_SIZE`,
`VERSION`, `BLOCK0_DATA_SIZE`, `BLOCKN_DATA_SIZE`, `MIN_CIPHERTEXT_SIZE`. Error type
`StreamEncError`. Functions: `parse_header`, `num_blocks`, `block_data_size`, `encrypted_block`,
`overhead`, `max_plaintext_size_for_target`, `block_nonce`, `encrypt` (native-only), `decrypt_block`,
`decrypt_all_blocks`.

## CHANGE-3

Done — Updated `src/utils/erasure.rs`: added `WRAP_OVERHEAD=12`, `PREFIX_SIZE`, `MAGIC_SUFFIX`
constants; `encode()` now wraps data with 8-byte size prefix + 4-byte magic suffix before encoding;
`extract_data()` signature changed to remove `original_data_size` parameter — now always unwraps
internally. Added `encode_raw()`, `extract_data_raw()`, `wrap_data()`, `unwrap_data()`,
`split_stripes()`. Added `UnwrapError` and `DataCorrupted` error variants. Fixed RSError variant
names (`EmptyShards` → `EmptyShard`, `ShardSizeMismatch` → `IncorrectShardSize`). Updated all
`extract_data` call sites in `src/sdk.rs` to remove the size argument. Updated
`test_chunk_encoded_size_with_erasure` expected value in `dag.rs` to reflect 12-byte wrap overhead.

## CHANGE-4

Done — Updated `src/utils/http_ext.rs` to return `AkaveError::TransientError(...)` for HTTP request
and body-read failures instead of `InternalError`. Added `pub const ERR_TRANSIENT: &str = "transient
error"`. Added `TransientError(String)` variant to `AkaveError` in `src/types/sdk_types.rs`.

## CHANGE-5

Skipped — Go-side ipc.Client additions (`ContractsAddresses`, `DeployContracts`,
`UpgradeStorage`). No direct Rust IPC deployment client equivalent exists in this repository.

## CHANGE-6

Done — Updated `src/sdk.rs`: renamed `batch_size` field → `chunk_batch_size` in `AkaveSDK` and
`AkaveSDKBuilder`; renamed `with_batch_size` → `with_chunk_batch_size`; removed
`use_connection_pool: bool` from builder and `new_with_params`; connection pool is now always
created on native builds (always `Some(Arc::new(...))`).

## CHANGE-7

Done — Part of CHANGE-6: removed `with_connection_pool()` method and `use_connection_pool`
parameter. Connection pool is always initialized. No optional flag remains.

## CHANGE-8

Skipped — Go's `MultiUpload` type is a new Go-only orchestration struct with no existing Rust
equivalent. The corresponding Rust upload path uses a different code structure. Full equivalence
would require a larger architectural addition beyond this sync scope.

## CHANGE-9

Skipped — `Upload2`/`Download2` are large new Go functions that depend on `streamenc` (CHANGE-2)
and `encode_raw` (CHANGE-3), both of which are now implemented in Rust. The Rust SDK does not yet
have direct counterparts for these functions. `streamenc` and `encode_raw` are marked with
`#[allow(dead_code)]` comments noting this dependency.

## CHANGE-10

Skipped — `IPC.Upload` refactor to delegate to `MultiUpload` (CHANGE-8). Skipped because CHANGE-8
was skipped.

## CHANGE-11

Done — Added `pub(crate) fn write_bytes_field_pub` helper and `pub fn build_leaf_node(data: &[u8]) -> Result<(Cid, Vec<u8>), String>` to `src/utils/dag.rs`. `build_leaf_node` creates a dag-pb ProtoNode with UnixFS TFile data, mirroring Go's `BuildLeafNode`.

## CHANGE-12

Skipped — Go module version-commit logic (`Version` derived from `runtime/debug.ReadBuildInfo`).
Go-only internal change; Rust has a static `VERSION` constant, no equivalent runtime build info
needed.

## CHANGE-13

Skipped — `ipctest` typed error (`ErrNodeNotAvailable`) changed to string matching in test code.
Go test-only change with no Rust equivalent.

## Cross-Verification

✓ CHANGE-1: `info: &str` matches Go `string` type; standalone `gcm_cipher` matches Go's package-level `GCMCipher`; `OVERHEAD=28` matches Go's `Overhead` constant (16 GCM tag + 12 nonce). `ceil_div` matches Go's `CeilDiv`. All sdk.rs call sites updated.
✓ CHANGE-2: `MAX_BLOCK_SIZE=32768`, `HEADER_SIZE=17`, `VERSION=1`, `BLOCK0_DATA_SIZE=32751`, `BLOCKN_DATA_SIZE=32752` match Go constants. `parse_header`, `num_blocks`, `block_data_size`, `encrypted_block`, `overhead` match Go function signatures. `encrypt`/`decrypt_block`/`decrypt_all_blocks` match Go function semantics.
✓ CHANGE-3: `WRAP_OVERHEAD=12` (8+4) matches Go `WrapOverhead`. `encode()` wraps before RS encode, `extract_data()` unwraps after RS decode. `encode_raw`/`extract_data_raw` match Go `EncodeRaw`/`ExtractDataRaw`. `split_stripes` returns borrowed slices matching Go sub-slice semantics.
✓ CHANGE-4: `TransientError` variant added; `range_download` returns `TransientError` on reqwest/body failures matching Go's `errors.Join(ErrTransient, ...)`.
✓ CHANGE-6: `chunk_batch_size` field and `with_chunk_batch_size` method match Go `WithChunkBatchSize`; `use_connection_pool` removed matching Go removal of optional pool flag; pool always created matching Go's unconditional pool.
✓ CHANGE-11: `build_leaf_node` serializes UnixFS TFile PbData then wraps in dag-pb PBNode, matching Go's `BuildLeafNode`.
Dead code (Cause A) CHANGE-2: `streamenc::encrypt`, `streamenc::decrypt_all_blocks`, `streamenc::decrypt_block` — depend on skipped CHANGE-9 (Upload2/Download2). Added `#[allow(dead_code)]` attributes.
Dead code (Cause A) CHANGE-3: `encode_raw`, `extract_data_raw`, `split_stripes` — depend on skipped CHANGE-9 (Upload2/Download2). Added `#[allow(dead_code)]` attributes.
Dead code (Cause A) CHANGE-11: `build_leaf_node` — new public function, no existing call site yet (would be used by Upload2 in CHANGE-9). Added `#[allow(dead_code)]` attribute.
