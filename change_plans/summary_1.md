# Change Summary for change_plan_1

## CHANGE-1

Skipped — Storage ABI artifact already up to date in this repository. The `+23 lines` in the Go diff were in the binding wrapper file (`storage.go`), not in the ABI JSON itself. The ABI content in `src/blockchain/storage.json` is bit-for-bit identical to the Go source at `meta.source_commit`.

## CHANGE-2

Done — Updated `src/blockchain/access_manager.json` from 22 to 23 ABI entries (added `getValidateAccessToBucket` view function). Added `GET_VALIDATE_ACCESS_TO_BUCKET` selector constant and `get_validate_access_to_bucket` method to `src/blockchain/access_manager.rs`, following the same pattern as the existing `get_validate_access` method.

## CHANGE-3

Skipped — The Go change renamed error variables to named constants for readability (e.g. `ErrOffsetOutOfBounds`). The Rust codebase uses `matches!(err, ProviderError::OffsetOutOfBounds)` enum-variant matching which does not depend on these named constants. No code change needed.

## CHANGE-4

Skipped — No PolicyFactory contract exists in the Rust repository. No `policy_factory.rs`, `policy_factory.json`, or equivalent Rust code was found. No action needed.

## CHANGE-5

Skipped — The Go change extracted `NodeAPIClient` into a struct wrapping gRPC client management. The Rust `AkaveSDK` struct holds `client: IpcNodeApiClient<ClientTransport>` directly with no separate legacy connection management layer to replace. The Go structural refactor has no applicable counterpart in Rust.

## CHANGE-6

Skipped — The Go change removed a `conn` field from the `IPC` struct in favor of the gRPC client. The Rust `IPC` struct (in `src/blockchain/`) holds `provider` and `signer` fields with no separate `conn` field. No equivalent old pattern found in Rust.

## TEST-1

Skipped — The Go change updates `TestContracts` to remove PolicyFactory setup. No equivalent `TestContracts` function or test exists in the Rust repository.

## TEST-2

Done — Created `tests/testrand.rs` with a `block(size: usize) -> (Vec<u8>, Cid)` helper function that generates a random byte slice and computes its CIDv1 (DAG-PB / SHA2-256), matching the Go `testrand.Block` helper. Includes two unit tests: `test_block_generates_correct_size` and `test_block_generates_unique_data`.

## TEST-3

Skipped — The Go change updates CLI error message tests. No equivalent CLI error message test exists in the Rust repository.

## Build notes

Pre-existing build issue fixed: `alloy = "0.11.0"` was resolving `alloy-consensus` to 0.11.1, which is incompatible with serde ≥ 1.0.220 due to the `serde_core` subcrate split. Fixed by pinning `alloy = "=0.11.0"` (exact) and `serde = { version = ">=1.0, <1.0.220" }`. Build verified with `cargo build --features vendored-protoc`. The `user.akvf.key` test file is intentionally absent from the repository (private key); `cargo test --test testrand` passes cleanly.
