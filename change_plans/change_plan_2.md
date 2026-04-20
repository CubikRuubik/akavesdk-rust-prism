# Change Plan: Modifications (PR #2)

## Summary

The `LatestBlockNumber` function has been enhanced to return richer block information instead of only the block number.

## Breaking Changes

- **`LatestBlockNumber` return type changed**: Previously returned a single integer (block number). Now returns a structured object containing:
  - `Number`: the block number (unsigned 64-bit integer)
  - `Time`: the block timestamp (date/time value)
  - `Hash`: the block hash (hex-encoded string)
- Any callers of `LatestBlockNumber` must be updated to use the new return type.

## Features

- **New `BlockInfo` data structure**: Introduced a new structured type to hold block metadata:
  - Block number
  - Block timestamp (resolved from Unix time)
  - Block hash (as a hex string)
- **Richer block query**: The function now fetches the full block header from the chain, enabling access to timestamp and hash in addition to the block number.

## Internal / Implementation Notes

- The implementation now makes two chain calls: first to get the latest block number, then to retrieve the full block header by that number.
- The hash is serialized as a hex string in the public API (SDK layer), while the internal layer retains the native hash type.

## Tests

- Tests updated to assert all three fields (`Number`, `Time`, `Hash`) are valid and non-zero.
- Log output updated to display all three fields for diagnostics.
