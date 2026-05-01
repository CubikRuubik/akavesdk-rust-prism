# Change Summary for change_plan_1

## CHANGE-1
Skipped — The AccessManager contract interface (functions, events, errors) is unchanged per
the change plan notes ("no type changes observed"). The ABI JSON stores only function/event/error
signatures, not bytecode, so a bytecode-only recompilation does not require updating
`src/blockchain/access_manager.json`. The Go source file was not available in this repository
to extract an updated ABI string.

## CHANGE-2
Skipped — The Storage contract ABI JSON (`src/blockchain/storage.json`) requires the updated ABI
string from the Go `StorageMetaData.ABI` field, which is not available in this repository. The
newly added contract error types (the substantive ABI-visible change) are handled by CHANGE-3.
This file should be regenerated from the Go source when the Go contract files are available.

## CHANGE-3
Done — Added 13 missing contract error types to `src/blockchain/contract_errors.rs`:
`NoPolicy`, `NotBucketOwner`, `BucketNotFound`, `FileDoesNotExist`, `NotThePolicyOwner`,
`CloneArgumentsTooLong`, `Create2EmptyBytecode`, `AlreadyWhitelisted`, `InvalidAddress`,
`NotWhitelisted`, `MathOverflowedMulDiv`, `NotSignedByBucketOwner`, `NonceAlreadyUsed`.
Each was added to the `sol!` block and to the `match_selectors!` call in `decode_revert_reason`.

## CHANGE-4
Skipped — The Rust SDK has no `PolicyFactory` references (confirmed: zero results for
`policy_factory` / `PolicyFactory` in `src/`). Already aligned.

## CHANGE-5
Done — Introduced `ConnectionPool` struct in `src/sdk.rs` that caches
`IpcNodeApiClient<ClientTransport>` connections keyed by node address. Added
`pool: Arc<ConnectionPool>` field to `AkaveSDK` (initialized in both WASM and native
constructors). Updated `upload_block_segments` to accept `pool: &ConnectionPool` and use
`pool.get_or_create()` instead of creating a new client per call. Updated both download
`async move` block sites to clone the pool before the loop and pass it into the closure.
Removed the now-superseded `get_client_for_node_address` associated function. Added
`AkaveSDK::close()` to release all pooled connections.

## CHANGE-6
Skipped — The Rust SDK has no `DeployListPolicy` or equivalent method (confirmed: zero results
in `src/` and `tests/`). Already aligned.

## CHANGE-7
Skipped — The Rust repo already uses only `proto/ipcnodeapi.proto`; no `nodeapi.proto` was
present. Already aligned.

## CHANGE-8
Skipped — The `Block(t, size)` test helper is optional and no existing Rust tests require it.
No CIDv1/DagPb block generation utility exists in the current test helpers to base it on.

## CHANGE-9
Skipped — No Rust integration tests contain assertions on the error strings `"not found"`,
`"bucket not found"`, or `"file not exists"` (confirmed: zero matches in `tests/`).

## CHANGE-10
Skipped — Makefile and README changes are Go-repo-specific documentation/build changes with no
direct impact on the Rust SDK.

## CHANGE-11
Skipped — `go.mod` dependency changes do not translate directly to `Cargo.toml`. No new
Ethereum/gRPC crates were identified as required equivalents.
