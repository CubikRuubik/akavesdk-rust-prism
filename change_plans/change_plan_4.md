# Change Plan: sync from akave repo for tag v0.4.0

## Summary

This change plan describes the updates introduced in the akave SDK reference implementation for the v0.4.0 tag. The changes refactor and extend the core SDK internals, remove deprecated subsystems, and introduce new utilities and client abstractions.

---

## Features

- **Batch client**: A new batch client has been introduced for the IPC layer, enabling multiple operations to be bundled and sent together, reducing round-trips and improving throughput.
- **Block parser**: A new block parser has been added to handle parsing of raw block data received from IPC nodes, enabling structured access to block content.
- **Transaction data parser**: A new parser has been added to extract structured data from transaction payloads in the IPC layer.
- **HTTP extension utilities**: A new set of HTTP helper utilities has been added to simplify and standardize HTTP interactions within the SDK.
- **CID utilities**: A new module for Content Identifier (CID) operations has been introduced, providing helpers to work with content-addressable identifiers.
- **Retry utility**: A general-purpose retry mechanism has been added, supporting configurable retry logic with backoff for use across the SDK.
- **PDP test helpers**: New test utilities have been added to support Proof of Data Possession (PDP) testing scenarios.

---

## Breaking Changes

- **Streaming subsystem removed**: The streaming command layer and its associated tests have been entirely removed. Any functionality relying on streaming commands must now be re-implemented using the new storage-oriented interface.
- **SP client removed**: The storage-provider client package has been removed. Dependent code that used this client must be updated to use the new IPC-based approach.
- **Crypto utilities removed**: The `cryptoutils` package (providing crypto source and shuffle utilities) has been removed. Any dependent logic must be migrated to alternative implementations.
- **Encryption splitter removed**: The encryption splitter utility has been removed. Dependent encryption flows must be updated accordingly.

---

## Refactoring

- **CLI restructured around storage**: The CLI command layer has been refactored, consolidating functionality under a storage-focused model. The previous IPC test file has been replaced with a new, comprehensive storage test file.
- **EIP-712 signing updated**: The EIP-712 message signing logic has been revised. Domain parameters and message structure have changed — implementations that perform EIP-712 signing must be updated to reflect the new structure.
- **IPC client simplified**: The IPC client has been simplified and aligned with the updated node API. The interface and expected behavior have changed.
- **IPC contracts updated**: The storage contract interface has been significantly updated with new method signatures and behaviors. All contract interactions must be reviewed.

---

## API / Protocol

- **Protobuf definitions updated**: Both the node API and IPC node API protocol buffer schemas have been updated with new RPC methods and message types. Clients that use the gRPC API must regenerate their stubs and update their call sites accordingly.
- **Connection handling updated**: The SDK's connection abstraction has been slightly modified to align with the updated API.

---

## Dependencies

- **Dependency versions updated**: Module dependencies have been bumped to align with the v0.4.0 release. Any dependent project should verify compatibility with the updated dependency versions.

---

## Configuration

- **Linter rules updated**: The static analysis configuration has been updated with new or modified rules.
- **Build targets updated**: The build automation file has been updated with new or modified targets.

---

## Documentation

- **README updated**: The project README has been updated to reflect the current usage instructions and architecture for v0.4.0.
