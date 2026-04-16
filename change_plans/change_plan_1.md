# Change Plan: added LatestBlockNumber method

Synced from [CubikRuubik/akavesdk-prism#1](https://github.com/CubikRuubik/akavesdk-prism/pull/1).

## Features

- **New capability: Retrieve the latest blockchain block number** — A new method (`LatestBlockNumber`) has been added that queries the connected blockchain node for the most recent block number. It accepts a cancellable/timeout-aware context and returns an unsigned 64-bit integer representing the current chain height, or an error if the request fails.
- **SDK-level exposure of `LatestBlockNumber`** — The capability is surfaced at the high-level SDK/IPC interface layer, wrapping the lower-level query with standard SDK error handling (domain-specific error wrapping) and observability/monitoring instrumentation (task tracing). Callers at the SDK level can now retrieve the latest block number without reaching into internal types.

## Testing

- **Low-level unit test added** — A test verifies that calling `LatestBlockNumber` against a running chain returns a block number strictly greater than zero, confirming the method connects and queries correctly.
- **SDK-level integration test added** — An end-to-end test exercises the full SDK stack (initialization, IPC interface acquisition, `LatestBlockNumber` call) to confirm the method works correctly through all abstraction layers.

## Breaking Changes

- None.

## Dependencies

- No new external dependencies introduced.

## Documentation

- Inline documentation (docstrings/comments) added for the new method at both the low-level client and SDK layers, describing the method's purpose and return values.
