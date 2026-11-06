#!/bin/sh
# Render both samples with the release binary. Run from the toy's directory.
set -e
cargo build --release --quiet
for f in newsletter transactional; do
  ./target/release/t20-to-email render "samples/$f.rafter" --css css/huttspawn.compiled.css --out out
done
