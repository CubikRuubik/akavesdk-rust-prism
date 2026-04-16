# Change Plan: added LatestBlockNumber method

## Features

- **New `LatestBlockNumber` operation on IPC client**: A new method was introduced on the low-level IPC client that queries the blockchain node for the most recently finalized block number, returning it as an unsigned 64-bit integer. This provides callers with a simple, direct way to determine the current chain head.
- **SDK-level exposure of `LatestBlockNumber`**: The same operation was added to the higher-level SDK IPC interface. At this layer the method includes error wrapping (to produce consistent SDK-typed errors) and observability instrumentation (monitoring/tracing hooks around the call lifecycle).

## Tests

- **Integration test for IPC client layer**: A new integration test verifies that `LatestBlockNumber` returns a positive block number when called against a live chain endpoint. It sets up a funded account and a deployed contract environment before making the call, asserting the result is greater than zero.
- **Integration test for SDK layer**: A new integration test covers the SDK-level `LatestBlockNumber` method end-to-end: it initialises the SDK with a funded private key, obtains the IPC sub-client, calls `LatestBlockNumber`, and asserts the returned value is positive — confirming correct delegation and result propagation through the SDK abstraction.

## Breaking Changes

- None.

## Dependencies

- No new external dependencies introduced.

## Documentation

- Inline doc-comments were added to both the IPC client method and the SDK method describing their purpose.
