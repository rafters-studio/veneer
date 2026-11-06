# T19 predictions

Written before any build. The question is one number: how big the `.rafter` compile core is as wasm. The core takes a `.rafter` JSON string (the rafters composite shape: `manifest`, `input`, `output`, `blocks[]` with `id`, `type`, `content`, `children`, `parentId`, `meta`, `rules`) and returns HTML, an mdast JSON tree and email HTML. No file system, no network, no JS or TS parser.

Both variants: `wasm32-unknown-unknown` cdylib, opt-level "z", lto, codegen-units 1, panic abort, strip. Export through a plain `extern "C"` (alloc / compile / free over linear memory), because wasm-bindgen adds its own glue and needs its CLI; plain extern is expected to be smaller.

## A, compile core (serde_json + walker + rule attrs + three emitters)

| measure | before wasm-opt | after wasm-opt -Oz | confidence |
|---|---|---|---|
| raw | 95 KB | 80 KB | 0.55 |
| gzip -9 | 40 KB | 35 KB | 0.55 |
| brotli -11 | 34 KB | 30 KB | 0.55 |

Range I would not be surprised by: 50 to 150 KB raw. wasm-opt saves 10 to 20 percent.

What takes the space: serde_json's parser and float handling plus `core::fmt` together over half; the allocator (dlmalloc) about 5 to 10 KB; my own walker and emitters under 20 KB.

## B, A plus lightningcss (parse, minify the huttspawn sheet, inline one rule)

| measure | before wasm-opt | after wasm-opt -Oz | confidence |
|---|---|---|---|
| raw | 1.8 MB | 1.5 MB | 0.5 |
| gzip -9 | 600 KB | 550 KB | 0.5 |
| brotli -11 | 450 KB | 420 KB | 0.5 |

Range I would not be surprised by: 1.0 to 3.0 MB raw. lightningcss with default features off.

What takes the space: lightningcss property parsing and serialization (the `Property` enum's parse and to_css matches) over half; cssparser and the selectors crate next; A's code a rounding error.

## Ratio and the number

B over A, raw after wasm-opt: about 18x. The one number for the compile core (A, brotli, after wasm-opt): about 30 KB.

## It works

Each wasm, run once under node on two sample `.rafter` files, returns all three outputs without a trap. Confidence 0.85.
