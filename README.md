<div align="center">
  <h1><code>Akave Wasm SDK</code></h1>
<strong>Akave SDK</strong>
</div>

## About

## Requirements

- `rustup`
- `wasm-pack`
- `node`

### 🛠️ Build

```
cd www
npm run wasm
npm run install
```

## 🚴 Usage

```
npm run grpc-proxy
npm run dev
```

### 🔬 Test in Headless Browsers with `wasm-pack test`

```
cd akave-wasm-sdk
wasm-pack test --headless --firefox
```

### 🎁 Publish to NPM with `wasm-pack publish`

```
cd akave-wasm-sdk
wasm-pack publish
```

## 🔋 Batteries Included

- [`wasm-bindgen`](https://github.com/rustwasm/wasm-bindgen) for communicating
  between WebAssembly and JavaScript.
- [`console_error_panic_hook`](https://github.com/rustwasm/console_error_panic_hook)
  for logging panic messages to the developer console.
