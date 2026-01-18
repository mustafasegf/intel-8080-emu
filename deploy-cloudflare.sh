#!/bin/bash
# Build and prepare for Cloudflare Pages deployment

set -e

echo "Building for WebAssembly..."

# Build the WASM binary
cargo build --release --target wasm32-unknown-unknown

# Create dist directory
rm -rf dist
mkdir -p dist/js

# Copy static assets
cp index.html dist/
cp js/mq_js_bundle.js dist/js/
cp target/wasm32-unknown-unknown/release/intel-8080-emu.wasm dist/
cp favicon-16.png favicon-32.png dist/

# Create _headers file for proper MIME types
cat >dist/_headers <<'EOF'
/*.wasm
  Content-Type: application/wasm
  Access-Control-Allow-Origin: *

/*
  X-Frame-Options: DENY
  X-Content-Type-Options: nosniff
EOF

echo ""
echo "Build complete! Files are in ./dist/"
echo ""
echo "To deploy to Cloudflare Pages:"
echo "  1. Go to https://dash.cloudflare.com/"
echo "  2. Select 'Workers & Pages' > 'Create application' > 'Pages'"
echo "  3. Connect your Git repo, OR use direct upload:"
echo "     npx wrangler pages deploy dist --project-name=intel-8080-emu"
echo ""
echo "Or deploy directly with Wrangler CLI:"
echo "  npx wrangler pages deploy dist --project-name=intel-8080-emu"
