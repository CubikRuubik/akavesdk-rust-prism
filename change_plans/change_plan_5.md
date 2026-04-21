# Change Plan: sync from akave repo from tag v0.4.4

**Source:** CubikRuubik/akavesdk-prism commit `8b66e30659c79ae8e763744bc50ae0a0540cf899`
**Tag:** v0.4.4

---

## Breaking Changes

- **Removal of gRPC/Protobuf communication layer**: The entire protobuf-defined node API (request/response message types, service definitions, and generated client/server stubs) has been removed. All communication that previously used gRPC over the node API must be migrated to the new IPC-based transport.
- **Removal of policy factory contract**: The `PolicyFactory` smart contract interface and its associated types have been removed. Any code that creates or manages policies through the factory must be updated to use the new approach.
- **IPC client API changes**: The inter-process communication client has been significantly refactored. Method signatures and behavior may have changed — callers must review and update their usage.

## Features

- **Expanded storage contract interface**: The storage contract now exposes additional operations (23 new additions), enabling more fine-grained control over storage interactions.
- **Enhanced error handling**: The IPC error layer has been substantially extended (139 new lines). More error types are now defined, providing richer context for failure scenarios and making error handling more precise.
- **Updated test utilities**: The test random-data helper has been refreshed to support new data shapes used in tests for the updated SDK surface.

## Dependencies

- **Major dependency update**: The module dependency manifest has been significantly updated (301 additions, 127 removals). Third-party libraries have been upgraded or replaced; dependent implementations should review whether any transitive dependencies they rely on have changed version or been removed.

## Documentation

- **README overhaul**: The project README has been substantially trimmed and revised (127 lines removed, 2 added), reflecting the new architecture and removing references to the deprecated gRPC layer.

## SDK Surface Changes

- **`sdk.go`**: Public SDK entry point updated — method signatures or available operations may have changed. Review the new interface and update any language-specific bindings accordingly.
- **`sdk_ipc.go`**: The IPC-backed SDK implementation has been refactored (10 additions, 37 removals). The internal wiring between the public API and the IPC layer is different; update any low-level integration code that depends on this path.

## Build / Tooling

- **Makefile updates**: Build targets and commands have been adjusted (9 additions, 13 deletions) to reflect the removal of the protobuf compilation step and other toolchain changes.

