#!/usr/bin/env bash
# Group a name-bearing wasm's code and data by crate: by-crate.sh <wasm>
# Custom sections (debug info, names) are left out: the stripped build has none.
set -euo pipefail
twiggy top -f csv "$1" | awk -F, '
  NR == 1 { next }
  {
    # Columns: Name,ShallowSize,ShallowSizePercent,RetainedSize,RetainedSizePercent; names may hold commas.
    size = $(NF-3); name = $0; sub(/(,[^,]*){4}$/, "", name); gsub(/"/, "", name)
    if (name ~ /^custom section/ || name ~ /subsection/) next
    if (name ~ /^data segment/) { k = "data (.rodata etc.)" }
    else {
      n = name; sub(/^<+(&mut |&)?/, "", n)
      if (match(n, /^[A-Za-z_][A-Za-z0-9_]*\[/)) k = substr(n, 1, RLENGTH - 1)
      else if (match(n, /^[A-Za-z_][A-Za-z0-9_]*::/)) k = substr(n, 1, RLENGTH - 2)
      else k = "other (tables, types, exports)"
    }
    sum[k] += size; total += size
  }
  END {
    for (k in sum) printf "%8d  %5.1f%%  %s\n", sum[k], 100 * sum[k] / total, k
    printf "%8d  100.0%%  total (code + data)\n", total
  }' | sort -rn
