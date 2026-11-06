#!/usr/bin/env bash
# Build both variants, wasm-opt them, measure, profile, and run each on both samples.
set -euo pipefail
cd "$(dirname "$0")"

CSS=${CSS:-/Volumes/store/projects/huttspawn/site/.rafters/output/rafters.css}

cargo build --release --target wasm32-unknown-unknown
cargo build --release --target wasm32-unknown-unknown --features css --target-dir target-b

mkdir -p wasm out
cp target/wasm32-unknown-unknown/release/t19_rafter_wasm.wasm wasm/a.wasm
cp target-b/wasm32-unknown-unknown/release/t19_rafter_wasm.wasm wasm/b.wasm
WOPT="--enable-bulk-memory --enable-bulk-memory-opt --enable-nontrapping-float-to-int --enable-sign-ext --enable-mutable-globals --enable-multivalue --enable-reference-types"
# Rust 1.98 targets these wasm features by default; strip removed the target_features section, so name them.
wasm-opt -Oz $WOPT wasm/a.wasm -o wasm/a.opt.wasm
wasm-opt -Oz $WOPT wasm/b.wasm -o wasm/b.opt.wasm

{
  printf '| file | raw | gzip -9 | brotli -11 |\n|---|---|---|---|\n'
  for f in a a.opt b b.opt; do
    raw=$(wc -c < "wasm/$f.wasm")
    gz=$(gzip -9 -c "wasm/$f.wasm" | wc -c)
    br=$(brotli -q 11 -c "wasm/$f.wasm" | wc -c)
    printf '| %s.wasm | %d | %d | %d |\n' "$f" "$raw" "$gz" "$br"
  done
} | tee out/sizes.md

for f in a.opt b.opt; do
  twiggy top -n 25 "wasm/$f.wasm" > "out/twiggy-$f.txt"
done
# Name-bearing profile: an unstripped, un-wasm-opted build of each, for twiggy to group by crate.
CARGO_PROFILE_RELEASE_STRIP=false cargo build --release --target wasm32-unknown-unknown --target-dir target-names
CARGO_PROFILE_RELEASE_STRIP=false cargo build --release --target wasm32-unknown-unknown --features css --target-dir target-names-b
twiggy top -n 40 target-names/wasm32-unknown-unknown/release/t19_rafter_wasm.wasm > out/twiggy-a-names.txt
twiggy top -n 40 target-names-b/wasm32-unknown-unknown/release/t19_rafter_wasm.wasm > out/twiggy-b-names.txt
bash by-crate.sh target-names/wasm32-unknown-unknown/release/t19_rafter_wasm.wasm > out/by-crate-a.txt
bash by-crate.sh target-names-b/wasm32-unknown-unknown/release/t19_rafter_wasm.wasm > out/by-crate-b.txt

for v in a b; do
  for s in signup article; do
    if [ "$v" = b ]; then
      node run.mjs "wasm/$v.opt.wasm" "samples/$s.rafter" "$CSS" > "out/$v-$s.json"
    else
      node run.mjs "wasm/$v.opt.wasm" "samples/$s.rafter" > "out/$v-$s.json"
    fi
  done
done
ls -l out
