#!/bin/sh
# Builds the browser version into dist/. Upload everything in dist/ to any
# static web host; it needs no server-side code.
set -e
cd "$(dirname "$0")"
cargo build --release --target wasm32-unknown-unknown
rm -rf dist
mkdir dist
cp target/wasm32-unknown-unknown/release/carmen.wasm web/index.html web/style.css web/mq_js_bundle.js web/storage.js web/input.js web/net.js web/favicon.ico web/apple-touch-icon.png dist/
# The loading bar needs the game's real size (see web/loader.js).
sed "s/__WASM_BYTES__/$(wc -c < dist/carmen.wasm | tr -d ' ')/" web/loader.js > dist/loader.js

# Cloudflare Pages reads _headers; other hosts ignore it. The policy lets the
# page reach the records server the game was built for (as progress.rs picks
# it), the wasm in from a blob: URL (loader.js) and compile it.
api=$(printf '%s' "${CARMEN_API-https://carmen-api.jonathan-orlowski.dev}" | sed -E 's#^(https?://[^/]+).*#\1#')
cat > dist/_headers <<HEADERS
/*
  Content-Security-Policy: default-src 'none'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self'; img-src 'self'; connect-src 'self' blob: $api; base-uri 'none'; form-action 'none'; frame-ancestors 'none'
  Permissions-Policy: camera=(), microphone=(), geolocation=(), payment=(), usb=()
HEADERS
echo "Built dist/. Try it locally with: python3 -m http.server -d dist 8000"
