#!/usr/bin/env bash
# Build the wasm bundle and serve the web UI on http://localhost:8000.
#
# The page loads wasm as an ES module, which browsers refuse over file://,
# so it has to come off a real HTTP origin even for local play.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
port="${1:-8000}"

echo "Building wasm bundle…"
wasm-pack build --release --target web --out-dir web/pkg --features wasm

echo
echo "Serving ${root}/web on http://localhost:${port}"
cd "${root}/web"
python3 -m http.server "${port}"
