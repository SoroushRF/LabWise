#!/bin/bash
# scripts/build-wasm.sh
# Builds the bridge crate to WASM and outputs to the projection frontend.
set -e

echo "🔨 Building LabWise WASM bridge..."

cd "$(dirname "$0")/.."
cd bridge

wasm-pack build --target web --out-dir ../projection/src/wasm-pkg

echo "✅ WASM build complete → projection/src/wasm-pkg/"
