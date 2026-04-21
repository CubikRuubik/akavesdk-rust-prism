# Change Plan: sync from akave repo from tag v0.4.4

**Source:** CubikRuubik/akavesdk-prism commit `8b66e30659c79ae8e763744bc50ae0a0540cf899`
**Tag:** v0.4.4

---

## Breaking Changes

- **Removal of gRPC/Protobuf communication layer**: The entire protobuf-defined node API — including all request/response message types, service definitions, and generated client/server stubs — has been removed. All communication that previously used gRPC over the node API must be migrated to the IPC-based transport.
- **Removal of policy factory contract**: The `PolicyFactory` smart contract interface and its associated types have been removed. Any code that creates or manages policies through the factory pattern must be updated to use the direct contract approach.
- **IPC client API refactoring**: The inter-process communication client has been significantly reworked. Method signatures and internal behavior have changed — all callers must review and update their usage accordingly.

## Features

- **Expanded storage contract interface**: The storage smart contract now exposes additional operations, providing more fine-grained control over storage interactions on-chain.
- **Enhanced error handling**: The IPC error layer has been substantially extended with many new named error types, providing richer context for failure scenarios and enabling more precise error handling at call sites.
- **New IPC error utility**: A new helper (`IgnoreOffsetError`) has been introduced to simplify handling of out-of-bounds offset errors, allowing callers to treat those as non-fatal.
- **Updated test utilities**: The test random-data helper has been refreshed to support new data shapes required by the updated SDK surface.

## SDK Surface Changes

- **Public SDK entry point updated**: The main SDK initialization has been modified — available options, method signatures, or wired-up behavior may have changed. Review the new interface and update any language-specific bindings accordingly.
- **IPC-backed SDK implementation refactored**: The internal wiring between the public SDK API and the IPC layer has been restructured. Update any low-level integration code that depends on this path.

## Dependencies

- **Major dependency update**: The module dependency manifest has been significantly updated with many additions and removals. Third-party libraries have been upgraded or replaced; dependent implementations should review whether any transitive dependencies they rely on have changed version or been removed.

## Documentation

- **README overhaul**: The project README has been substantially revised, removing references to the deprecated gRPC layer and reflecting the updated architecture.

## Build / Tooling

- **Makefile updates**: Build targets and commands have been adjusted to reflect the removal of the protobuf compilation step and other toolchain changes.
