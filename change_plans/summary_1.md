# Change Summary for change_plan_1

## CHANGE-1

Skipped — `src/blockchain/storage.json` was already identical to the new ABI extracted from the Go binding at `meta.source_commit` (101 entries, same content). No update required.

## CHANGE-2

Done — Updated `src/blockchain/access_manager.json` from 22 to 23 entries by adding the new `getValidateAccessToBucket` function. Added corresponding `get_validate_access_to_bucket` method to `src/blockchain/access_manager.rs` and the `GET_VALIDATE_ACCESS_TO_BUCKET` constant.

## CHANGE-3

Skipped — The Go change was a structural refactor (anonymous inline errors → named exported constants) with no selector changes. The Rust `ignore_offset_error` in `provider.rs` already uses `matches!(err, ProviderError::OffsetOutOfBounds)`, which is the idiomatic Rust equivalent of `errors.Is(err, ErrOffsetOutOfBounds)`. The Rust `contract_errors.rs` error list was verified against the Go dispatch table; no selector values changed and the set of errors matches the Storage ABI.

## CHANGE-4

Skipped — No `PolicyFactory`, `ListPolicyAbi`, `PolicyFactoryAbi`, or `PolicyFactoryContractAddress` fields exist in the Rust `AkaveSDK` struct or anywhere in the codebase. No equivalent to remove.

## CHANGE-5

Skipped — The Rust `AkaveSDK` struct has no legacy non-IPC `NodeAPIClient` field analogous to the Go `client` field that was removed. The Rust `client` field is the IPC gRPC client (`IpcNodeApiClient`), not a separate non-IPC client. No Rust equivalent to remove. Connection pooling is not implemented in the Rust SDK; no teardown change required.

## CHANGE-6

Skipped — The Rust `AkaveSDK` has no separate `conn` or `pool` field analogous to the Go `IPC` struct's refactored field. The Rust SDK uses the IPC gRPC client directly without a connection pool abstraction. No Rust equivalent to replace.

## TEST-1

Skipped — No `deploy_list_policy`, `TestDeployListPolicy`, or `PolicyFactory` deployment logic exists in the Rust integration tests. No Rust equivalent to update.

## TEST-2

Done — Added `src/utils/testrand.rs` with a `rand::block(size)` function that generates an IPFS block (random bytes + CIDv1 with DagProtobuf codec) mirroring the Go `testrand.Block` utility. Exposed the module from `src/utils/mod.rs`. Four unit tests verify size, codec, CID version, and uniqueness.

## TEST-3

Done — Updated `src/cli/mod.rs` line 1979: changed expected error output for "File info for non-existent file" test case from `"file not exists"` to `"BucketNotFound"`, reflecting improved error propagation that surfaces the raw contract error name instead of a wrapped gRPC message.
