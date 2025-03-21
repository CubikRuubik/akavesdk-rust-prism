# Akave WASM SDK Web Demo

This is a simple web application that demonstrates the basic functionality of the Akave WASM SDK. The demo includes features for connecting to MetaMask, managing buckets, and handling files.

## Features

- Connect to MetaMask wallet
- Create and delete buckets
- List buckets and files
- Delete files
- (TODO) Upload files

## Prerequisites

- Node.js (v14 or higher)
- MetaMask browser extension
- A web browser with WebAssembly support

## Installation

1. Install dependencies:
```bash
npm install
```

2. Build the project:
```bash
npm run build
```

## Running the Demo

1. Start the development server:
```bash
npm run dev
```

2. Open your browser and navigate to `http://localhost:8080`

3. Make sure MetaMask is installed and unlocked

4. Click "Connect Wallet" to connect your MetaMask wallet

## Usage

1. **Connecting Wallet**
   - Click the "Connect Wallet" button
   - Approve the connection in MetaMask
   - Your wallet address will be displayed

2. **Managing Buckets**
   - Enter a bucket name in the input field
   - Click "Create Bucket" to create a new bucket
   - View your buckets in the list below
   - Delete buckets using the "Delete" button

3. **Managing Files**
   - Select a bucket from the dropdown
   - View files in the selected bucket
   - Delete files using the "Delete" button
   - (TODO) Upload files using the file input

## Development

The project uses webpack for bundling and development. The main files are:

- `index.html`: The main HTML file
- `styles.css`: CSS styles
- `app.js`: Main JavaScript file with SDK integration
- `webpack.config.js`: Webpack configuration

### SDK Dependency

This demo is configured to use the local SDK from the parent directory for development and testing purposes. The dependency is specified in `package.json` as:

```json
"@akave/akave-wasm-sdk": "file:.."
```

For production use, you should replace this with the published version from npm:

```json
"@akave/akave-wasm-sdk": "^1.0.0"
```

## Notes

- The file upload functionality is currently disabled as it requires additional implementation
- Make sure you have sufficient funds in your wallet for gas fees
- The demo connects to the Akave testnet by default 