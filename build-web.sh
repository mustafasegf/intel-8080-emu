#!/bin/bash
# Build script for WebAssembly target

set -e

echo "Building for WebAssembly..."

# Build the WASM binary
cargo build --release --target wasm32-unknown-unknown

# Copy the WASM file to the project root for easy serving
cp target/wasm32-unknown-unknown/release/intel-8080-emu.wasm .

echo ""
echo "Build complete!"
echo ""
echo "To run locally, use a web server:"
echo "  python3 -m http.server 8080"
echo ""
echo "Then open: http://localhost:8080"
