# Change Summary for change_plan_1

## CHANGE-1

Skipped — Storage ABI verified identical. The Go diff added helper methods in Go binding code, but the ABI JSON content in `src/blockchain/storage.json` already matched the authoritative Go source at `meta.source_commit`. No update required.

## CHANGE-2

Done — Updated `src/blockchain/access_manager.json` with the new `getValidateAccessToBucket(bytes32 bucketId, address user, bytes data) → bool` function entry extracted from the Go source `private/ipc/contracts/access_manager.go` at `meta.source_commit`. Item count increased from 22 to 23.

## CHANGE-3

Skipped — The Rust `ignore_offset_error` in `src/blockchain/provider.rs` already uses `matches!(err, ProviderError::OffsetOutOfBounds)` which is semantically equivalent to Go's `errors.Is(err, ErrOffsetOutOfBounds)`. No code change required.

## CHANGE-4

Skipped — No `PolicyFactory` or equivalent contract deployment logic exists in the Rust codebase. Searched with `grep -r "PolicyFactory\|deploy_list_policy\|policy_factory"` — zero results. This is a Go-side test-only concern involving a factory contract not mirrored in the Rust SDK.

## CHANGE-5

Skipped — The Rust `AkaveSDK` struct in `src/sdk.rs` uses `IpcNodeApiClient<ClientTransport>` directly (IPC only). There is no non-IPC `NodeAPIClient` field to remove. Searched with `grep -r "NodeAPIClient\|node_api_client"` — zero non-IPC results. No equivalent structural change applies.

## CHANGE-6

Skipped — The Rust IPC struct does not hold a raw `grpc.ClientConn` field; it uses `IpcNodeApiClient<ClientTransport>` which already manages the connection internally. Searched with `grep -r "\.conn\b\|grpc.*conn\|ClientConn"` — zero results in `src/`. The Go change (replacing `conn` with `pool`) has no Rust analog to implement.

## TEST-1

Skipped — No contract lifecycle test involving `PolicyFactory` deployment or `deploy_list_policy` pattern was found in the Rust test suite. Searched with `grep -r "deploy_list_policy\|PolicyFactory\|lifecycle"` — zero results. This test exists only in the Go SDK.

## TEST-2

Done — Implemented `random_block(size: usize) -> Result<(Vec<u8>, Cid), getrandom::Error>` in `src/utils/io.rs` as the Rust equivalent of Go's `testrand.Block` helper. Uses `getrandom` for CSPRNG data and `cid::multihash::Code::Sha2_256` with CIDv1 dag-protobuf (0x70) encoding. Added `test_random_block` test. Both function and test are gated with `#[cfg(not(target_arch = "wasm32"))]` to match the non-WASM dependency constraints.

## TEST-3

Skipped — No CLI-layer or integration test asserting specific error message strings (e.g. `"BucketNotFound"`) was found in `tests/` or `src/`. Searched with `grep -r "BucketNotFound\|bucket.*not.*found\|error.*message" tests/ src/` — zero matching test assertions. This test exists only in the Go CLI tests.

## Build Fix (pre-existing)

Done — Updated `Cargo.toml` from `alloy = "0.11.0"` to `alloy = "2.0.0"` to resolve a pre-existing build failure. `alloy-consensus 0.11.x` through `0.15.x` use `serde::__private` (an unstable internal API) directly, which was renamed to `serde::__private{patch}` in serde ≥ 1.0.214. Since the transitive dependency chain requires serde ≥ 1.0.221 (`jiff` → `env_logger` → `serde >= 1.0.221`), downgrading serde was not possible. `alloy 2.0.x` no longer uses `serde::__private` directly and compiles cleanly. The three alloy API call sites (`alloy::sol`, `alloy::sol_types::SolError`, `alloy::hex`) are unchanged and compatible with alloy 2.x.
