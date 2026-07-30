#!/usr/bin/env bash
# Recompile le pont WebAssembly et l'installe dans la livraison.
#
# Le dossier de construction vit dans `outputs/work/` et NON dans
# `outputs/webapp/` : la livraison doit rester autosuffisante et propre
# (aucun artefact de compilation dedans).
set -euo pipefail
ici=$(cd "$(dirname "$0")" && pwd)
export CARGO_TARGET_DIR="$ici/work/target"
cd "$ici/webapp/wasm"
cargo build --release --target wasm32-wasip1
cp "$CARGO_TARGET_DIR/wasm32-wasip1/release/terra_pont.wasm" "$ici/webapp/terra.wasm"
ls -l "$ici/webapp/terra.wasm"
