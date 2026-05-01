# Change Summary for change_plan_1

## CHANGE-1
Done — Added `getValidateAccessToBucket` function to `src/blockchain/access_manager.json` (updated ABI from 22 to 23 entries); added `GET_VALIDATE_ACCESS_TO_BUCKET` constant and `get_validate_access_to_bucket` method to `src/blockchain/access_manager.rs`.

## CHANGE-2
Skipped — Verified `src/blockchain/storage.json` ABI is identical to the Go source at tag v0.4.4; no change required.

## CHANGE-3
Done — Added 13 new Solidity custom error types (`NoPolicy`, `NotBucketOwner`, `BucketNotFound`, `FileDoesNotExist`, `NotThePolicyOwner`, `CloneArgumentsTooLong`, `Create2EmptyBytecode`, `AlreadyWhitelisted`, `InvalidAddress`, `NotWhitelisted`, `MathOverflowedMulDiv`, `NotSignedByBucketOwner`, `NonceAlreadyUsed`) to the `sol!` block and `match_selectors!` macro in `src/blockchain/contract_errors.rs`.

## CHANGE-4
Skipped — `PolicyFactory` contract has no Rust equivalent in this repository.

## CHANGE-5
Done — Added `NodeConnectionPool` struct to the `native_support` module in `src/sdk.rs` with `tokio::sync::RwLock<HashMap<String, IpcNodeApiClient<ClientTransport>>>` for connection reuse. Added `node_pool: Arc<NodeConnectionPool>` field to `AkaveSDK` (non-WASM only), initialized in `new_with_params`, plumbed as a `#[cfg(not(target_arch = "wasm32"))]` parameter to `upload_block_segments`, and used in the non-WASM paths of `download_file` and `download_file_range`.

## CHANGE-6
Skipped — `DeployListPolicy` has no Rust equivalent in this repository.

## CHANGE-7
Skipped — Rust SDK already uses IPC NodeAPI exclusively; no change required.

## CHANGE-8
Skipped — Optional block test helper; no existing test suite requires it in Rust.

## CHANGE-9
Skipped — `tests/exit_code.rs` contains no error string assertions that reference changed error messages.

## CHANGE-10
Skipped — Documentation and build-tooling only change with no Rust equivalent.

## CHANGE-11
Skipped — Go dependency update (go.mod/go.sum); informational only, no Rust action required.

## Additional Fixes
- Fixed pre-existing `alloy-consensus v0.11.1` incompatibility with `serde v1.0.228`: vendored a patched copy of `alloy-consensus` in `patches/alloy-consensus/` replacing the hand-written `serde::__private` usage with `#[serde(untagged)]`, and added a `[patch.crates-io]` entry to `Cargo.toml`.
- Fixed pre-existing bugs in the `vendored-protox` feature of `build.rs`: corrected `serde::__private` prost version mismatch and moved `conf.skip_protoc_run()` to after `conf` is initialized.
