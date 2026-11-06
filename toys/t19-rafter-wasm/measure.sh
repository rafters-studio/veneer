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
# Rust 1.98 targets these wasm features by default; strip removed the target_features section, so name them.
WOPT="--enable-bulk-memory --enable-bulk-memory-opt --enable-nontrapping-float-to-int --enable-sign-ext --enable-mutable-globals --enable-multivalue --enable-reference-types"
wasm-opt -Oz $WOPT wasm/a.wasm -o wasm/a.opt.wasm
wasm-opt -Oz $WOPT wasm/b.wasm -o wasm/b.opt.wasm

# A with a wasm-bindgen export instead of extern "C", to compare the two (wasm-bindgen-cli 0.2.130).
cargo build --release --target wasm32-unknown-unknown --features bindgen --target-dir target-bg
BG=target-bg/wasm32-unknown-unknown/release/t19_rafter_wasm.wasm
wasm-bindgen --target web --out-dir wasm/bg-web --out-name a_bg "$BG"
wasm-bindgen --target nodejs --out-dir wasm/bg-node --out-name a_bg "$BG"
cp wasm/bg-web/a_bg_bg.wasm wasm/a-bg.wasm
wasm-opt -Oz $WOPT wasm/a-bg.wasm -o wasm/a-bg.opt.wasm

size_row() {
  printf '| %s | %d | %d | %d |\n' "$1" "$(wc -c < "$1")" "$(gzip -9 -c "$1" | wc -c)" "$(brotli -q 11 -c "$1" | wc -c)"
}
{
  printf '| file | raw | gzip -9 | brotli -11 |\n|---|---|---|---|\n'
  for f in wasm/a.wasm wasm/a.opt.wasm wasm/b.wasm wasm/b.opt.wasm wasm/a-bg.wasm wasm/a-bg.opt.wasm wasm/bg-web/a_bg.js; do
    size_row "$f"
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

for s in signup article; do
  node run.mjs wasm/a.opt.wasm "samples/$s.rafter" > "out/a-$s.json"
  node run.mjs wasm/b.opt.wasm "samples/$s.rafter" "$CSS" > "out/b-$s.json"
  node run-bindgen.cjs "samples/$s.rafter" > "out/a-bg-$s.json"
done
cmp out/a-signup.json out/a-bg-signup.json && cmp out/a-article.json out/a-bg-article.json && echo "bindgen output matches extern C output"
ls -l out
