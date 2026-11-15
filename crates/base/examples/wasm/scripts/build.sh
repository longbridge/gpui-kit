#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
crate_dir="$(cd "$script_dir/.." && pwd)"
workspace_dir="$(cd "$crate_dir/../../../.." && pwd)"
profile="debug"
# Keep this toolchain aligned with CI and story-web: newer nightly regresses
# recursive Task async-drop queries (E0391).
cargo_args=(+nightly-2026-10-31 build --manifest-path "$crate_dir/Cargo.toml" --target wasm32-unknown-unknown)
if [[ "${1:-}" == "--release" ]]; then profile="release"; cargo_args+=(--release); fi
cargo "${cargo_args[@]}"
wasm-bindgen "$workspace_dir/target/wasm32-unknown-unknown/$profile/gpui_base_examples_wasm.wasm" --out-dir "$crate_dir/www/src/wasm" --target web --no-typescript
