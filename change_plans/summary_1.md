# Change Summary for change_plan_1

## CHANGE-1

Skipped — This change added a `PolicyFactory` contract to the Go SDK. No `PolicyFactory` references exist in the Rust SDK; the Rust SDK does not implement this Go-only contract helper.

## CHANGE-2

Skipped — The Go side renamed its proto package import path. The Rust SDK already uses the correct `ipcnodeapi` naming throughout `src/` and `proto/ipcnodeapi.proto`.

## CHANGE-3

Done — Added `ConnectionPool` struct (backed by `Arc<TokioMutex<HashMap<String, IpcNodeApiClient<ClientTransport>>>>`) to `src/sdk.rs`. Added `connection_pool: ConnectionPool` field to `AkaveSDK` and initialised it in both WASM and non-WASM `new_with_params` paths. Renamed the static `get_client_for_node_address` helper to `create_node_client` (used exclusively by `ConnectionPool::get_or_connect`). Updated `upload_block_segments` to accept a `pool: ConnectionPool` parameter and wired all four former `get_client_for_node_address` call-sites in `upload_block_segments`, `download_file_stream`, and `download_file_range` to use `pool.get_or_connect()` instead.

## CHANGE-4

Done — Added 14 missing Solidity error types (`NoPolicy`, `NotBucketOwner`, `BucketNotFound`, `FileDoesNotExist`, `FileNonexists`, `NotThePolicyOwner`, `CloneArgumentsTooLong`, `Create2EmptyBytecode`, `AlreadyWhitelisted`, `InvalidAddress`, `NotWhitelisted`, `MathOverflowedMulDiv`, `NotSignedByBucketOwner`, `NonceAlreadyUsed`) to the `sol! { }` block and the `match_selectors!` macro call in `src/blockchain/contract_errors.rs`.

## CHANGE-5

Done — Created `tests/helpers.rs` with a `random_block(size: usize) -> (String, Vec<u8>)` helper that fills a buffer with random bytes via `getrandom` and derives a CIDv1 DagProtobuf (`0x70`) content identifier using `cid::multihash::Code::Sha2_256`, matching the Go SDK's `testutils.RandomBlock` signature.

## CHANGE-6

Skipped — Changes were limited to the Go Makefile and Go README. No Rust equivalent exists.

## CHANGE-7

Skipped — Changes were to `go.mod` / `go.sum` (Go module dependencies). No Rust equivalent.

## CHANGE-8

Skipped — Trivial Go formatting / comment change in `contracts/access_manager.go` with no Rust equivalent.

## Build fixes (pre-existing issues resolved)

Two pre-existing build failures were fixed as part of this sync:

1. **`serde::__private` removed in serde ≥ 1.0.219** — `alloy-consensus 0.11.1` directly references `serde::__private::de::Content` which no longer exists as a stable path in serde ≥ 1.0.219 (the module was renamed to a versioned `__private<N>` path). Fixed by pinning `serde = "=1.0.219"` in both the `[dependencies]` and `[target.'cfg(target_arch = "wasm32")'.dependencies]` sections of `Cargo.toml`.

2. **`vendored-protox` feature broken in `build.rs`** — Two bugs prevented the build from working without a system `protoc`:
   - `conf.skip_protoc_run()` was called before `conf` was in scope.
   - `file_descriptors.encode_to_vec()` resolved to prost 0.13's `Message` trait but `protox 0.6` returns a prost 0.12 `FileDescriptorSet`.
   
   Fixed by using UFCS `protox::prost::Message::encode_to_vec(&file_descriptors)` (protox re-exports its embedded prost 0.12) and moving `conf = conf.skip_protoc_run()` to after `conf` is initialised.
