# Change Summary for change_plan_1

## CHANGE-1

Skipped — `src/blockchain/storage.json` already contains the up-to-date ABI (101 entries, diff confirmed to be zero between the extracted Go ABI at `8b66e30` and the current Rust file). No changes needed.

## CHANGE-2

Done — Added the new `getValidateAccessToBucket` view function entry to `src/blockchain/access_manager.json` (+1 entry, from 22 to 23 entries). Added the corresponding `get_validate_access_to_bucket` method and `GET_VALIDATE_ACCESS_TO_BUCKET` constant to `src/blockchain/access_manager.rs`.

## CHANGE-3

Skipped — The Go change was a pure structural refactor (anonymous inline error values promoted to named exported package-level variables). The set of error selectors and error names did not change. The Rust `ignore_offset_error` function in `src/blockchain/provider.rs` already uses an enum-variant `matches!` check (`ProviderError::OffsetOutOfBounds`), which is equivalent to Go's new `errors.Is()` identity comparison. No code changes required.

## CHANGE-4

Skipped — No PolicyFactory equivalent was found in the Rust codebase. Searched for `policy_factory`, `PolicyFactory`, `deploy_list_policy`, and `DeployListPolicy` across all Rust source files; all searches returned zero results.

## CHANGE-5

Skipped — The Go change removed a legacy non-IPC `NodeAPIClient` field from the Go SDK struct. The Rust `AkaveSDK` struct does not contain a legacy non-IPC client field; its `client: IpcNodeApiClient<ClientTransport>` field is the active IPC gRPC client and is not the pattern being removed. The Rust SDK already holds a `connection_pool` field, so no structural change is needed.

## CHANGE-6

Skipped — The Go IPC struct replaced a direct `conn grpc.ClientConn` field with a `pool connectionPool` field. The Rust `AkaveSDK` struct already uses `connection_pool: Option<Arc<tokio::sync::RwLock<HashMap<String, Channel>>>>` rather than a bare connection. No structural change is needed.

## TEST-1

Skipped — No Rust integration test for contract lifecycle with PolicyFactory deployment was found. Searches for `deploy_list_policy`, `PolicyFactory`, and `TestContracts` in the `tests/` directory returned zero results, consistent with CHANGE-4 (no PolicyFactory existed in Rust).

## TEST-2

Done — Created `tests/testrand.rs` with a `testrand::block(size: usize) -> (Vec<u8>, Cid)` helper. The function fills a buffer with cryptographically random bytes via `getrandom`, hashes them with SHA2-256 using `cid::multihash::Code::Sha2_256.digest`, and wraps the digest in a CIDv1 with the DagProtobuf codec (`0x70`) — a direct Rust port of Go's `private/testrand/testrand.go:Block`. Includes three inline unit tests.

## TEST-3

Skipped — No Rust CLI-level integration test that checks error output strings was found in the `tests/` directory (only `tests/exit_code.rs` exists, which does not test error message content). No equivalent of Go's `TestExternalViewBucketCommand` exists to update.
