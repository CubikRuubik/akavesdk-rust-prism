# Change Summary for change_plan_1

## CHANGE-1

Skipped — No `PolicyFactoryContractAddress` field exists in the Rust `AkaveSDK` struct or any config struct; there is no Rust equivalent to remove.

## CHANGE-2

Skipped — The Rust `AkaveSDK` struct has no `PolicyFactory`, `ListPolicyAbi`, or `PolicyFactoryAbi` fields; these concepts were never introduced in the Rust codebase.

## CHANGE-3

Skipped — No `deploy_list_policy` function exists in the Rust SDK; there is no Rust equivalent of the renamed `TestDeployListPolicy` method.

## CHANGE-4

Skipped — The Rust SDK has no per-operation connection pool structure to migrate. Connections to storage nodes are created on demand via `get_client_for_node_address`; the main gRPC client (`self.client`) is already a persistent struct field. No equivalent pool abstraction existed to refactor.

## CHANGE-5

Done — Updated `src/blockchain/contract_errors.rs` to add the six AccessManager v0.4.4 error types (`BucketNotFound`, `FileDoesNotExist`, `InvalidAddress`, `NoDelegatedAccess`, `NoPolicy`, `NotBucketOwner`) to the `sol!` macro and the `match_selectors!` dispatch list, enabling `decode_revert_reason` to decode these errors by type-based selector matching (identity comparison, not string match). The `OffsetOutOfBounds` check already used selector comparison.

## Contract Updates

- **Storage ABI**: No change required — `src/blockchain/storage.json` was already up to date with the v0.4.4 ABI (101 items, identical content).
- **AccessManager ABI**: Done — Updated `src/blockchain/access_manager.json` with the v0.4.4 ABI (adds `getValidateAccessToBucket` function; 23 items total).
- **PolicyFactory contract**: Skipped — No equivalent file exists in the Rust repository; removal confirmed intentional.

## Test Updates

- **TEST-1, TEST-2, TEST-3**: Skipped — The Rust test suite does not have CLI integration tests that assert specific error output strings. The new AccessManager error types added under CHANGE-5 ensure `BucketNotFound` is decoded correctly when the contract reverts.
- **TEST-4**: Skipped — No equivalent list-policy contract lifecycle test exists in the Rust codebase.
