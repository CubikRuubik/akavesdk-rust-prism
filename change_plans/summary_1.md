# Change Summary for change_plan_1

## CHANGE-1

Skipped — No `PolicyFactoryContractAddress` field exists in any Rust struct; this config field was never added to the Rust codebase.

## CHANGE-2

Skipped — No `PolicyFactory`, `ListPolicyAbi`, or `PolicyFactoryAbi` fields exist in any Rust struct or module; the PolicyFactory contract binding was never ported to Rust.

## CHANGE-3

Skipped — No `deploy_list_policy` method exists in the Rust SDK; the factory-mediated deployment flow was never implemented in Rust.

## CHANGE-4

Skipped — The Rust SDK does not use a per-operation gRPC connection pool; gRPC client channels are already struct-level fields on `AkaveSDK`, so no refactoring is required.

## CHANGE-5

Skipped — The `ignore_offset_error` function in `src/blockchain/provider.rs` already uses `matches!(err, ProviderError::OffsetOutOfBounds)`, which is a type-based identity comparison (not a string match). No change required.

## Contract Updates

### Storage

Skipped — The new Storage ABI from `private/ipc/contracts/storage.go` at `8b66e30` is identical to the current `src/blockchain/storage.json`; no update needed.

### AccessManager

Done — Updated `src/blockchain/access_manager.json` with the v0.4.4 ABI from `private/ipc/contracts/access_manager.go` at `8b66e30`. The new ABI adds the `getValidateAccessToBucket(bucketId, user, data) → bool` view function.

### PolicyFactory

Skipped — No `policy_factory` binding exists in the Rust repository; removal is already complete.
