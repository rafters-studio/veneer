#!/bin/sh
# sh run.sh <real site> <clone with @rafters/veneer@0.0.6> <probe project> <lint fixture>
# Every measurement in README.md, in order. Reads the real site; writes nothing outside out/.
set -eu
site=$1
clone=$2
probe=$3
fixture=$4
here=$(cd "$(dirname "$0")" && pwd)
rust=$here/target/release/t18-oxc-check
ts=$clone/node_modules/.bin/rafters-check
m="node $here/measure.mjs"
mkdir -p "$here/out"

{
  echo "# 1. one file, cold process, --stdin with the file's own text"
  for f in src/pages/index.astro src/components/hs-header.classes.ts src/components/hs-panel.astro; do
    $m "rust site $f" 10 "$site" "$site/$f" "$rust" --stdin "$f"
    $m "rust clone $f" 10 "$clone" "$clone/$f" "$rust" --stdin "$f"
    $m "ts clone $f" 10 "$clone" "$clone/$f" "$ts" --stdin "$f"
  done

  echo "# 2. the whole project"
  $m "rust site whole" 10 "$site" - "$rust"
  $m "rust clone whole" 10 "$clone" - "$rust"
  $m "ts clone whole" 10 "$clone" - "$ts"

  echo "# 4. --stdin with a proposed text that differs from disk"
  $m "rust site proposed index.astro" 10 "$site" "$here/proposed-index.astro" "$rust" --stdin src/pages/index.astro
  $m "rust clone proposed index.astro" 10 "$clone" "$here/proposed-index.astro" "$rust" --stdin src/pages/index.astro
  $m "ts clone proposed index.astro" 10 "$clone" "$here/proposed-index.astro" "$ts" --stdin src/pages/index.astro
} | tee "$here/out/times.txt"

echo "# 3. parity"
(cd "$clone" && "$ts" > "$here/out/hs-ts.txt") || true
(cd "$clone" && "$rust" > "$here/out/hs-rust-clone.txt") || true
(cd "$site" && "$rust" > "$here/out/hs-rust-site.txt") || true
node "$here/fixture.mjs" "$rust" "$fixture" | tee "$here/out/fixture.txt"
node "$here/probes.mjs" "$ts" "$rust" "$probe" > "$here/out/probes.txt"
tail -1 "$here/out/probes.txt"
