# Akave-rs with React + TypeScript + Vite + TanStack Query

This template provides a minimal setup to get Akave-rs working in Vite with React and the api under TanStack Query.

All the SDK related code in `src/hooks/useAkave.ts`. This file provides a wrapped tanStack query hook over the SDK for ease of use.

## Install:

```bash
npm install
npm run postinstall # This will copy the akave-rs wasm SDK into examples/web-demo-react-vite/akave-rs so it can be imported inside the application. Remember to compile the SDK for wasm before doing this.
```

## Usage:

```bash
npm run dev
```
