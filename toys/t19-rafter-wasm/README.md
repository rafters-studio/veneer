# T19: the `.rafter` compile core as wasm

One number: how big the `.rafter` compile core is as wasm. A toy, not a feature; its output is plausible, not complete. Predictions were committed before any build (`PREDICTIONS.md`, commit 72dd5e6).

**The number: 120 KB raw, 53 KB brotli -11** for the compile core (variant A, after `wasm-opt -Oz`). Adding lightningcss (variant B) makes it 2.3 MB raw, 415 KB brotli.

## What it is

`src/lib.rs` takes a `.rafter` JSON string (the rafters composite shape from `packages/composites/src/manifest.ts`: `manifest`, `blocks[]` with `id`, `type`, `content`, `children`, `parentId`, `meta`, `rules`) and returns JSON `{ html, mdast, email }`. No file system, no network, no JS or TS parser.

- serde_json `Value` in (no derive); blocks walked as `walk-blocks.ts` does (roots are blocks without `parentId`, a visited set per root, depth 50).
- Rules become HTML5 attributes as `rule-attrs.ts` does: `required`, `email`, `url`, `password` fixed sets; `pattern`, `minlength`, `maxlength` from `config` (a digit string accepted); unknown rules and `credentials` ignored; later rules win.
- Block types resolve as `resolve-block.ts` does (`composite:<id>`, the native alias table, else a component).
- mdast follows the cases of `to-mdx.ts` (paragraph, heading depth clamped 1..6, blockquote, list, thematicBreak, `Grid` and composites as `mdxJsxFlowElement`).
- Email HTML puts inline styles from a fixed 12-entry tag style map; a grid becomes a one-row table.
- `src/css.rs` (feature `css`, variant B): lightningcss parses the stylesheet with error recovery, minifies it, prints it minified, and collects top-level style rules by selector. A block with `meta.class` gets that class rule's declarations inlined into its `style` in the email output.

Export: **plain `extern "C"`** (`alloc`, `dealloc`, `compile_rafter`, and `compile_rafter_css` in B). The result is `[u32 LE length][utf-8 JSON]` in linear memory. No wasm-bindgen: it adds a JS glue file and a CLI step, and the toy needs neither. The wasm imports nothing.

lightningcss pulls getrandom (through ahash), which has no backend on `wasm32-unknown-unknown` without JS. `.cargo/config.toml` selects getrandom's `custom` backend and `css.rs` supplies a fixed seed: hash seeds only, and a compile core should be deterministic anyway.

## Sizes (bytes)

| file | raw | gzip -9 | brotli -11 |
|---|---|---|---|
| A, before wasm-opt | 143,330 | 63,361 | 54,805 |
| A, after wasm-opt -Oz | 120,225 | 60,725 | 53,488 |
| B, before wasm-opt | 2,693,335 | 570,111 | 383,402 |
| B, after wasm-opt -Oz | 2,296,447 | 598,626 | 415,145 |

wasm-opt -Oz cuts raw size by 16 percent (A) and 15 percent (B), but barely changes A's compressed size (-2 percent brotli) and makes B's compressed size **larger** (+5 percent gzip, +8 percent brotli). For B shipped compressed, the un-optimized build is smaller on the wire.

## Against the predictions

| measure | predicted | actual | |
|---|---|---|---|
| A raw, before | 95 KB | 143 KB | miss, 1.5x (inside the 50 to 150 KB range) |
| A raw, after | 80 KB | 120 KB | miss, 1.5x |
| A gzip, before / after | 40 / 35 KB | 63 / 61 KB | miss, 1.6x / 1.7x |
| A brotli, before / after | 34 / 30 KB | 55 / 53 KB | miss, 1.6x / 1.8x |
| wasm-opt saves 10 to 20 percent | | 16 percent raw | hit (raw only; compressed it saves almost nothing) |
| B raw, before | 1.8 MB | 2.69 MB | miss, 1.5x (inside the 1.0 to 3.0 MB range) |
| B raw, after | 1.5 MB | 2.30 MB | miss, 1.5x |
| B gzip, before / after | 600 / 550 KB | 570 / 599 KB | hit within 10 percent, but wasm-opt went the other way |
| B brotli, before / after | 450 / 420 KB | 383 / 415 KB | before 15 percent under; after hit |
| B over A, raw after | 18x | 19.1x | hit |
| runs on both samples | yes, 0.85 | yes, all four runs | hit |

What takes the space, against the prediction (from `out/by-crate-*.txt`, an unstripped build with identical code, grouped by the crate leading each symbol name; generic code is counted under the crate it is instantiated from, so `alloc` and `core` hold some serde_json Vec and BTreeMap code):

- A predicted: serde_json plus `core::fmt` over half; dlmalloc 5 to 10 KB; my code under 20 KB. Actual: data segments 21.5 percent (30.8 KB: panic and error strings, float parse and print tables), `core` 16.6 percent, `alloc` 15.6 percent, serde_json 14.5 percent (20.8 KB), this crate 12.1 percent (17.4 KB, hit), dlmalloc 5.5 percent (7.9 KB, hit), hashbrown 3.4 percent. serde_json itself is smaller than predicted; the miss is the standard library's share (core, alloc, data together near 54 percent), which I put inside "serde_json plus fmt" and underweighted. The largest single function is this crate's `visit` (9.9 KB).
- B predicted: lightningcss over half, then cssparser and the selectors crate. Actual: lightningcss 64.8 percent (1.75 MB; `BorderHandler::flush` 67 KB and `Property::parse` 59 KB are the biggest functions), cssparser 13.1 percent (352 KB), core 4.4 percent, smallvec 3.6 percent, data 3.3 percent; parcel_selectors only 1.0 percent (miss: smallvec and data_encoding each outweigh it). A's code (serde_json plus this crate) is 1.5 percent of B (hit).

## It works

Each optimized wasm ran once under node on both samples (`samples/signup.rafter`, `samples/article.rafter`); outputs are in `out/`. All four returned html, mdast and email without a trap.

- signup: `<input name="email" required type="email" />`, `<input minlength="12" name="password" required type="password" />` (the explicit minlength overrides password's 8), `<input maxlength="16" name="handle" pattern="[a-z0-9_]{3,16}" />` (`credentials` and an unknown rule ignored).
- article: heading level 9 clamped to `h6` / depth 6, a grid as `data-columns="2"` in HTML, a table in email and `Grid` in mdast, `composite:newsletter-signup` as `NewsletterSignup`, a self-reference skipped, a second root rendered.
- B with the huttspawn sheet (`/Volumes/store/projects/huttspawn/site/.rafters/output/rafters.css`, 157,981 bytes, read only): parsed and minified to 146,425 bytes. The sheet is a Tailwind v4 source sheet (`@theme`, `@utility`, `@apply`), so lightningcss keeps most of it as unknown at-rules; the real minification target would be the sheet after Tailwind. The article's root block has `meta.class: "dark"`, and the `.dark` rule's declarations (custom properties, about 11 KB) were inlined into its email `style`.

## Commands

```
rustup target add wasm32-unknown-unknown
brew install binaryen          # wasm-opt 133
cargo install twiggy --locked
bash measure.sh                # builds A and B, wasm-opt, sizes, twiggy, by-crate, runs
```

`measure.sh` runs, in order:

```
cargo build --release --target wasm32-unknown-unknown
cargo build --release --target wasm32-unknown-unknown --features css --target-dir target-b
wasm-opt -Oz --enable-bulk-memory --enable-bulk-memory-opt --enable-nontrapping-float-to-int \
  --enable-sign-ext --enable-mutable-globals --enable-multivalue --enable-reference-types in.wasm -o out.wasm
gzip -9 -c f.wasm | wc -c ; brotli -q 11 -c f.wasm | wc -c
twiggy top -n 25 wasm/{a,b}.opt.wasm
CARGO_PROFILE_RELEASE_STRIP=false cargo build ... --target-dir target-names[-b]   # names for twiggy
bash by-crate.sh <unstripped wasm>
node run.mjs wasm/a.opt.wasm samples/<s>.rafter
node run.mjs wasm/b.opt.wasm samples/<s>.rafter <sheet.css>
```

The wasm-opt feature flags are needed because Rust 1.98 emits bulk-memory and the other features by default, and `strip` removes the section that would tell wasm-opt so.

## Caveats

- The rafters sources were read from the rafters checkout's working tree; this agent could not run git there to confirm it matched origin/main.
- No `.rafter` files were found to use, so both samples were written for this toy in the composite shape.
- Size is the question; the emitters cover the to-mdx cases and the rule-attrs table, not `each`/`as` binding, nested composite expansion, or component rendering.
- Not tried, and each would move the number: a smaller allocator (for example a bump allocator, about 7 KB saved), dropping serde_json's error strings, `-Zbuild-std` with `panic_immediate_abort`, lightningcss with minify only (no `Targets`/compat tables).
