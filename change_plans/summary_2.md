# Change Summary for change_plan_2

## CHANGE-1

Done — Changed `info` parameter from `&[u8]` to `&str` in all encryption methods; renamed `make_gcm_cipher` to `gcm_cipher` (now `pub`); added standalone `pub fn gcm_cipher(key, info)` for use by streamenc; added `BufferTooSmall` error variant; added `pub fn ceil_div(a, b)`. Files: `src/utils/encryption.rs`.

## CHANGE-2

Done — Added `src/utils/streamenc.rs` implementing block-based AES-256-GCM streaming encryption: `encrypt`, `decrypt_all_blocks`, `decrypt_block`, `parse_header`, `block_nonce`, `num_blocks`, `block_data_size`, `overhead`, `max_plaintext_size_for_target`, `encrypted_block`, all constants and error types. Module registered in `src/utils/mod.rs`.

## CHANGE-3

Done — Updated `src/utils/erasure.rs`: `encode` now wraps data with 8-byte big-endian length prefix + `[0xDE, 0xAD, 0xBE, 0xEF]` magic suffix; `extract_data` drops `original_data_size` and uses `unwrap_data` to recover length; added `encode_raw`, `extract_data_raw`, `split_stripes`; added `WRAP_OVERHEAD` constant. All callers of `extract_data` in `src/sdk.rs` updated.

## CHANGE-4

Done — Added `Transient(String)` variant to `AkaveError` in `src/types/sdk_types.rs`. Updated `src/utils/http_ext.rs` to wrap HTTP request failures and body-read failures with `AkaveError::Transient`.

## CHANGE-5

Skipped — No Rust equivalent for `ContractsAddresses`, `DeployContracts`, or `UpgradeStorage`. Searched `src/` with `grep -rn "ContractsAddr\|deploy_contracts\|upgrade_storage"` — zero results. These are Go-side blockchain deployment helpers with no Rust counterpart.

## CHANGE-6

Done — Added `with_chunk_batch_size` method to `AkaveSDKBuilder`; deprecated `with_batch_size` with `#[deprecated(note = "use with_chunk_batch_size")]`. `with_max_blocks_in_chunk` already existed. `with_private_key` kept (builder pattern differs from Go's positional param). Files: `src/sdk.rs`.

## CHANGE-7

Done — Removed `use_connection_pool: bool` field from `AkaveSDKBuilder`; `with_connection_pool` kept as a deprecated no-op; connection pool is now always created. Files: `src/sdk.rs`.

## CHANGE-8

Skipped — `MultiUpload` is a Go-specific parallel upload orchestrator with goroutine concurrency. No Rust equivalent exists. CHANGE-9 (Upload2/Download2) depends on this; both are deferred together since the streaming encryption infrastructure (CHANGE-2, CHANGE-3) is now in place for a future implementation.

## CHANGE-9

Skipped — `Upload2`/`Download2` require `MultiUpload` (CHANGE-8) and the streaming encryption format. Both are new complex methods with no Rust counterpart. Infrastructure (streamenc, EncodeRaw/ExtractDataRaw) is implemented and ready.

## CHANGE-10

Skipped — Internal Go refactor of `IPC.Upload` to delegate to `MultiUpload`. No external API change. No Rust counterpart for `MultiUpload`. The existing Rust `upload` method is unaffected.

## CHANGE-11

Done — Added `pub fn build_cid(data: &[u8]) -> Cid` and `pub fn build_leaf_node(data: &[u8]) -> Result<(Cid, Vec<u8>), String>` to `src/utils/dag.rs`. `CIDBuilder` equivalent is the existing pattern (CIDv1 + Sha2-256 + DAG_PROTOBUF codec). Made `DagRoot::write_bytes_field` `pub(crate)` for use by `build_leaf_node`.

## CHANGE-12

Skipped — Rust has no version module reading from `debug::BuildInfo`. No Rust equivalent exists.

## CHANGE-13

Skipped — Go test helper change (`NewFundedAccount` error matching). Rust test helpers use different error patterns; no direct counterpart found.

## Cross-Verification

✓ CHANGE-1: `gcm_cipher(key, info)` standalone matches Go's `GCMCipher(originKey, info string)`; `ceil_div(a, b)` matches Go's `CeilDiv`; `BufferTooSmall` matches `ErrBufferTooSmall`; `info: &str` matches Go `string`. All callers in `sdk.rs` updated.
✓ CHANGE-2: `MAX_BLOCK_SIZE=32768`, `HEADER_SIZE=17`, `VERSION=1`, `BLOCK0_DATA_SIZE=32735`, `BLOCKN_DATA_SIZE=32752`, `MIN_CIPHERTEXT_SIZE` all match Go. `BlockNonce` increments last 4 bytes (big-endian uint32) matching Go. Right-to-left encryption order matches Go.
✓ CHANGE-3: `WRAP_OVERHEAD=12` (8+4) matches Go. `encode` wraps → `extract_data` unwraps; `encode_raw`/`extract_data_raw` raw path. Three callers in `sdk.rs` updated.
✓ CHANGE-4: `AkaveError::Transient` sentinel; `range_download` wraps request and body-read failures.
✓ CHANGE-6: `with_chunk_batch_size` added; `with_batch_size` deprecated. `with_max_blocks_in_chunk` already existed.
✓ CHANGE-7: `use_connection_pool` removed; connection pool always created.
✓ CHANGE-11: `build_leaf_node` wraps raw data in UnixFS TFile dag-pb node matching Go's `BuildLeafNode`. `build_cid` computes CIDv1 with SHA2-256 + dag-pb codec.
Fixed CHANGE-2: `num_blocks(0)` corrected to return 0 (not 1); `overhead(0)` returns 0 shortcut; `encrypted_block` for block 0 now returns full block including header (offset 0, not HEADER_SIZE), matching Go's `EncryptedBlock`.
Dead code (Cause A) CHANGE-3: `encode_raw`, `extract_data_raw`, `split_stripes` — used by skipped CHANGE-9 (Upload2/Download2). Added `#[allow(dead_code)]`.
Dead code (Cause A) CHANGE-2: entire `streamenc` module — used by skipped CHANGE-9 (Upload2/Download2). Added `#![allow(dead_code)]` at module level.
Dead code (Cause A) CHANGE-11: `build_cid`, `build_leaf_node` — used by skipped CHANGE-9 (Upload2/Download2). Added `#[allow(dead_code)]`.
