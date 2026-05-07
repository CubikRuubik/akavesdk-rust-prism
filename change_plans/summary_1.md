# Change Summary for change_plan_1

## CHANGE-1: Update Storage contract ABI

Skipped — `src/blockchain/storage.json` is already identical to the ABI embedded in
`private/ipc/contracts/storage.go` at `8b66e30`. The Go binding had code additions
(+23 lines) but the embedded ABI JSON content itself did not change.

## CHANGE-2: Update AccessManager contract ABI

Done — Replaced `src/blockchain/access_manager.json` with the new ABI from
`private/ipc/contracts/access_manager.go` at `8b66e30`. The only addition is the
`getValidateAccessToBucket` view function (22 → 23 entries).

## CHANGE-3: Synchronise error dispatch table

Done — Read `private/ipc/errors.go` at `8b66e30` and verified that no selector values
changed. Added 14 error types missing from the Go dispatch table to the `sol!` block and
`match_selectors!` call in `src/blockchain/contract_errors.rs`:
`BucketNotFound`, `FileDoesNotExist`, `InvalidAddress`, `NoPolicy`, `NotBucketOwner`
(from AccessManager ABI), `AlreadyWhitelisted`, `NotWhitelisted`,
`NotSignedByBucketOwner`, `NonceAlreadyUsed` (from list-policy / storage-signing
contracts), and `FileNonexists`, `NotThePolicyOwner`, `CloneArgumentsTooLong`,
`Create2EmptyBytecode`, `MathOverflowedMulDiv` (legacy / cross-contract entries still
present in the Go dispatch table).

## CHANGE-4: Remove PolicyFactory from IPC client

Skipped — No PolicyFactory contract, `PolicyFactoryContractAddress` config field, or
related client fields (`PolicyFactory`, `ListPolicyAbi`, `PolicyFactoryAbi`) exist in
the Rust codebase. No Rust equivalent was found.

## CHANGE-5: Refactor SDK struct — remove NodeAPIClient, add connection pool

Skipped — The Rust `AkaveSDK` struct's `client` field is an `IpcNodeApiClient` (the IPC
gRPC transport), not a legacy non-IPC HTTP-based NodeAPIClient like the Go `client`
field that was removed. No direct Rust equivalent of the removed field exists. The Rust
SDK does not maintain a separate connection pool structure analogous to the Go `pool`
field.

## CHANGE-6: Refactor IPC struct — replace direct connection with connection pool

Skipped — The Rust SDK does not have a separate `IPC` struct with a `conn` field.
Connection management is handled differently in Rust; there is no direct equivalent of
the Go `conn → pool` replacement.

## TEST-1: Update integration test for contract lifecycle

Skipped — No Rust integration test for the full contract lifecycle (bucket/file lifecycle
including policy deployment) was found in the Rust test suite. No Rust equivalent exists.

## TEST-2: Port new Block test-utility helper function

Done — Created `src/utils/testrand.rs` (gated by `#[cfg(all(test, not(target_arch = "wasm32")))]`)
with a `random_block(size: usize) -> (Vec<u8>, Cid)` function. It fills a buffer with
cryptographically random bytes (via `OsRng`), hashes with SHA2-256, and constructs a
CIDv1 with DagProtobuf codec — mirroring `testrand.Block` from the Go source. Exposed
from `src/utils/mod.rs`.

## TEST-3: Update CLI-level integration test error-message expectations

Skipped — No Rust CLI-level integration test checking bucket-not-found error message
strings was found in the test suite. The only integration test (`tests/exit_code.rs`)
checks exit codes, not error message text. No Rust equivalent exists.
