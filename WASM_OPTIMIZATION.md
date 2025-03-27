# WASM Optimization Instructions

To optimize the WASM output, run the following commands in sequence:

1. wasm-gc: Removes unused functions and data
```bash
wasm-gc target/wasm32-unknown-unknown/release/akave-rs.wasm
```

2. wasm-snip: Removes specific functions and code
```bash
wasm-snip --snip-rust-fmt-code --snip-rust-panicking-code -o target/wasm32-unknown-unknown/release/akave-rs.wasm target/wasm32-unknown-unknown/release/akave-rs.wasm
```

3. wasm-strip: Removes debug symbols and other metadata
```bash
wasm-strip -o target/wasm32-unknown-unknown/release/akave-rs.wasm target/wasm32-unknown-unknown/release/akave-rs.wasm
```

4. wasm-opt: First pass with aggressive optimization
```bash
wasm-opt -O4 --enable-bulk-memory --enable-threads --enable-reference-types --enable-simd --enable-tail-call --dce --low-memory-unused --shrink-level=2 -o target/wasm32-unknown-unknown/release/akave-rs.wasm target/wasm32-unknown-unknown/release/akave-rs.wasm
```

5. wasm-opt: Second pass with size optimization
```bash
wasm-opt -Oz --enable-bulk-memory --enable-threads --enable-reference-types --enable-simd --enable-tail-call --dce --low-memory-unused --shrink-level=2 -o target/wasm32-unknown-unknown/release/akave-rs.wasm target/wasm32-unknown-unknown/release/akave-rs.wasm
```

Note: Replace 'akave-rs.wasm' with the actual output file name. These commands should be run after the build is complete. 