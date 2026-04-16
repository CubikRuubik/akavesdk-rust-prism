# Change Plan: added LatestBlockNumber method

_Synced from [CubikRuubik/akavesdk-prism PR #1](https://github.com/CubikRuubik/akavesdk-prism/pull/1)_

## Summary

A new `LatestBlockNumber` method has been added to the SDK's IPC (on-chain) layer. It queries the underlying blockchain node for the most recently mined block number and returns it as an unsigned 64-bit integer.

---

## Features

- **New method: `LatestBlockNumber`**
  - Added at the low-level IPC client layer: delegates to the Ethereum JSON-RPC `eth_blockNumber` call, returning the latest block number on the chain.
  - Exposed at the public SDK layer (IPC interface): wraps the low-level call with error propagation and observability/monitoring instrumentation.
  - Accepts a context parameter for deadline/cancellation control.
  - Returns the latest block number as an unsigned 64-bit integer, or an error if the RPC call fails.

---

## Tests

- Integration test added at the IPC client level verifying that calling `LatestBlockNumber` after contract deployment returns a non-zero block number.
- Integration test added at the SDK level verifying end-to-end that `LatestBlockNumber` returns a positive block number when called through the public API.

---

## Breaking Changes

None. This is a purely additive change.

---

## Dependencies

No new dependencies introduced.

---

## Required Changes in This Repository

Implement a `latest_block_number` method (or equivalent, following this language's naming conventions) on the IPC/on-chain client interface that:

1. Issues a `eth_blockNumber` (or equivalent) RPC query to the connected blockchain node.
2. Returns the result as an unsigned 64-bit integer (or the idiomatic equivalent).
3. Accepts a context/cancellation token.
4. Propagates errors appropriately using this SDK's error handling conventions.
5. Include appropriate tests verifying that a non-zero block number is returned after node connection.
