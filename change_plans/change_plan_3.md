# Change Plan 3 — sync from akave repo for tag v0.3.0

Source commit: `429a32ed8907e80ceb9d8aadfe40150f78ec2333`
Origin: akave repo tag v0.3.0 (PR #51)

---

## Features

- **Unary block upload endpoint**: A new non-streaming (unary) RPC method was added for uploading a single file block. Clients can now choose between a streaming and a non-streaming approach when uploading individual blocks.
- **Range-based file download**: A new RPC endpoint allows initiating a file download over a specific byte range (defined by a start index and end index), enabling partial or chunked retrieval of large files.
- **Connection parameters extended**: The `ConnectionParams` response now includes an `access_address` field, exposing the on-chain access manager contract address to clients alongside the existing storage address and dial URI.
- **Batch transaction receipt fetching**: A new method allows clients to query multiple blockchain transaction receipts in a single batched request, reducing round-trips when confirming multiple on-chain operations.
- **Wallet balance query**: A new CLI subcommand (`wallet balance`) retrieves and displays the AKVT token balance for a specified wallet address by querying the node's connection parameters and the underlying chain.
- **Wallet import**: A new CLI subcommand (`wallet import`) allows importing an existing wallet via a raw private key, deriving the address, and persisting it to the local keystore.
- **Upgradeable storage contract (ERC-1967 proxy)**: The storage contract is now deployed behind an upgradeable proxy following the ERC-1967 standard. The proxy is initialized with the token address, enabling future in-place upgrades without redeployment.
- **Policy factory & list policy deployment**: Contract deployment now includes deploying a base list policy implementation and a policy factory, which together manage per-user access control policies on-chain.
- **Encryption support in DAG construction**: File chunking and DAG building now optionally accept an encryption key; when provided, data is split and encrypted at the block level before being hashed into the DAG.

## Breaking Changes

- **Storage contract interface updated**: The `Storage` smart contract ABI has changed significantly (new functions, modified function signatures, removed functions). Any SDK binding that calls storage contract methods directly must be regenerated or updated.
- **Access manager contract interface updated**: The `AccessManager` contract ABI has changed; callers interfacing with access management on-chain must update their bindings accordingly.
- **Policy factory contract updated**: The policy factory contract interface has changed; clients using policy management need updated bindings.
- **`ConnectionParamsResponse` has a new required field**: The response from `ConnectionParams` now includes `access_address`. Clients that deserialize this response must handle the new field.
- **`DeployContracts` sequence changed**: The deployment flow now includes an ERC-1967 proxy and grants minter roles automatically. Any tooling or test harness that mirrors the deployment sequence must be updated.
- **`BuildDAG` function signature changed**: The DAG construction function now accepts an encryption key parameter. Callers that do not use encryption must pass an empty/nil value.

## Fixes

- **Improved error messages**: The error-code-to-human-readable-message mapping was significantly expanded. New error codes cover cases such as `OffsetOutOfBounds`, `NonceAlreadyUsed`, `NotSignedByBucketOwner`, `InvalidBlocksAmount`, `InvalidBlockIndex`, `LastChunkDuplicate`, `FileNotExists`, and several ECDSA signature validation errors.
- **`IgnoreOffsetError` helper**: A new utility function is provided to suppress `OffsetOutOfBounds` errors, simplifying pagination logic in callers.

## Dependencies

- Dependency tree updated to align with akave v0.3.0 (additions and removals in the module dependency list). Clients building against this SDK should regenerate their lock files.

## Testing

- New comprehensive test suites added for IPC client operations, streaming operations, and common CLI helpers. Tests cover both happy paths and error scenarios.

## Documentation / Tooling

- Line-length linting configuration added (`.linelint.yml`).
- Makefile updated with new build/test targets.
- README updated.
