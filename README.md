<div align="center">
  <h1><code>Akave Wasm SDK</code></h1>
<strong>Akave SDK</strong>
</div>

## About

## Requirements

- `rustup`
- `wasm-pack`
- `node`

## 🚴 Usage

```
npm run grpc-proxy
npm run dev
```

### 🛠️ Build (only needed after changes on wasm, the `dev` process makes the initial build)

```
npm run build:wasm
```

### 🔬 Test in Headless Browsers with `wasm-pack test`

```
npm run test
```

### 🎁 Publish to NPM with `wasm-pack publish`

```
npm run publish
```

## 🔋 Batteries Included

- [`wasm-bindgen`](https://github.com/rustwasm/wasm-bindgen) for communicating
  between WebAssembly and JavaScript.
- [`console_error_panic_hook`](https://github.com/rustwasm/console_error_panic_hook)
  for logging panic messages to the developer console.
