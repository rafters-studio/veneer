# T20: a .rafter file to email HTML, with lightningcss doing the CSS

The question: can veneer's Rust core turn a `.rafter` file into email HTML that real mail clients render the way the designer meant, with lightningcss doing the CSS? This is a toy, not a feature. `PREDICTIONS.md` was written and committed before any build.

The answer: yes for the CSS. lightningcss parses Tailwind's compiled huttspawn output and resolves every `var()` and `calc()`. It lowers every OKLCH token to sRGB. The inlined colours match, to the byte, what a browser computes from the same stylesheet with the site's classes, in light and in dark, and an email renders in under 0.3 ms.

Today huttspawn sends its auth emails through `@rafters/platform-sender`, whose `render` writes bare `<!doctype html><html><body><p>..</p></body></html>`. That path carries no design tokens, so it is no baseline for styling. Its verification email sends a code; the transactional sample here uses a link button instead. What it cannot carry is layout and behaviour that email lacks: container queries, flex stretch, dark mode in Gmail and Outlook Windows, rounded corners in Outlook Windows. Those come out as design decisions, listed at the end.

```sh
# 1. the classes the samples use, then Tailwind's compiled output for exactly those (scratch dir, site read only)
cargo build --release
./target/release/t20-to-email classes samples/*.rafter > /tmp/classes.txt
node tools/compile-css.mjs <scratch dir with tailwindcss@4.3.3 + @tailwindcss/cli@4.3.3> /tmp/classes.txt
# 1b. OKLCH tokens to hex with rafters' own @rafters/color-utils (writes css/huttspawn.rafters-hex.css, out/color-diff.tsv)
node <rafters>/packages/color-utils/node_modules/tsx/dist/cli.mjs tools/rafters-colors.mjs css/huttspawn.compiled.css css/huttspawn.rafters-hex.css target/release/t20-to-email
# 2. the emails (out/*.html), their decision reports (out/*.report.json) and the class-based references
sh tools/render.sh
# 3. evidence
node tools/caniemail.mjs <caniemail repo clone> out/newsletter.html out/transactional.html
node tools/shoot.mjs <dir with playwright 1.63.0>        # out/shots/*.png
node tools/colors.mjs <dir with playwright 1.63.0>       # reference vs email colours
node tools/react-email.mjs <dir with @react-email/components 1.0.12>   # html and render's plainText
node tools/send-check.mjs <dir with playwright 1.63.0>   # part 2: plain text, line length, doctype, preheader
./target/release/t20-to-email bench css/huttspawn.compiled.css 200 samples/newsletter.rafter samples/transactional.rafter
```

## 1. How react-email renders (read first)

Read from the npm packages `@react-email/render` 2.2.0, `@react-email/tailwind` 2.0.7 and `@react-email/button` 0.2.1. Their sources were pulled from the published source maps.

- **Inlines:** every rule whose class is on an element and which has no at-rule and no pseudo selector inside it (`is-rule-inlinable.ts`). Mixed rules are split. The class stays only if part of its rule could not be inlined.
- **Keeps in `<style>`:** rules with an at-rule (media queries) or a pseudo-class/element. Every declaration in them gets `!important`, and class names are sanitised (`hover:x` becomes `hover_x`). There must be a `<head>` inside `<Tailwind>`, or it throws.
- **Lowers:** `var()` is substituted by text when the use and the definition share a selector or `:root`. A local custom property, or the `@property` initial value, also counts. `calc()` is reduced, `oklch()` becomes comma `rgb()` with a per-channel clamp and no gamut mapping, hex becomes `rgb()`, and `color-mix(.., transparent)` becomes `rgb(.., a)`. `padding-inline`/`-block` and `margin-inline`/`-block` become physical sides, and `calc(infinity*1px)` radius becomes 9999px. `rem` is left alone.
- **Drops:** nothing on purpose. A var with no definition stays as `var()`.
- **Dark mode:** nothing of its own. A `dark:` class compiles to a media query in `<style>`, and there is no colour-scheme meta.
- **Outlook:** layout is tables (`Body`, `Container`, `Section`, `Row`, `Column`). `Button` puts `mso-padding-alt:0` on the link and fakes its horizontal padding with `<i style="mso-font-width:..">` hair spaces inside `<!--[if mso]>`, plus `mso-text-raise` for vertical padding. `Html` gets the XHTML 1.0 Transitional doctype. There is no VML.
- **Theme source:** its Tailwind is compiled from a JS config (`@config`), and its stylesheet loader serves only Tailwind's own files (`setup-tailwind.ts`). A CSS-first `@theme` such as rafters.css cannot be given to it.

## 2. Input

- `samples/newsletter.rafter`: heading, text, image, button, separator, and a two-column grid of heading and text. `samples/transactional.rafter`: heading, text, one button, a separator and a two-line footer. Both are the composite format of `packages/composites/src/manifest.ts`. Blocks are flat with `children`/`parentId`, block types go through `resolve-block.ts` (`heading`, `text` and `image` are native aliases; `button`, `grid`, `container` and `separator` are components), and options are in `meta`. No block carries `rules`, so `rule-attrs.ts` has nothing to map (an email has no form fields).
- **Classes:** `src/rafter.rs` holds the class strings the installed huttspawn components emit, transcribed from `src/components/ui/{container,grid,typography,image,button,separator}.classes.ts` for the variants the samples use. The body wrapper is `bg-background text-foreground font-sans`, which is the site's `body { @apply bg-background text-foreground }` plus preflight's `html` font.
- **CSS:** `css/huttspawn.compiled.css` (118,766 bytes) is Tailwind 4.3.3's compiled output, the version the site has installed. It was compiled in a scratch copy by `tools/compile-css.mjs`. The input is the site's `.rafters/output/rafters.css` and the `@layer base`/`@theme` tail of `src/styles/globals.css`, with three removals: the fontsource import, the site-only imports and the `@font-face`, and auto source detection (`source(none)` plus `@source` naming the class list). Nothing was written to the site. Spot check against the site's real build (`dist/_astro/Base.DuyZm3zP.css`): `--rafters-radius-base`, `--primary`, `--primary-hover`, `--rafters-background`, `--font-sans` and `--rafters-dark-primary` are identical.

## 3. The transform, and every choice in it

`src/css.rs` reads the CSS and `src/email.rs` writes the email. The only dependencies are lightningcss (features `substitute_variables` only), serde and serde_json.

CSS:
1. lightningcss parses the compiled sheet. Rules are indexed by the class that leads their selector, which is unescaped back to how it is written in HTML. The suffix (`:hover`, `[aria-busy=true]`) and the at-rules around a rule (`@media`, `@container`, `@supports`) are kept with it. Nested rules (`&:hover`, nested `@supports`) are flattened onto their parent.
2. Variables: `:root, :host` declarations are the light map, and `.dark` on top of light is the dark map (rafters switches dark by class). `@property` initial values are added as defaults. A rule's own custom properties override for that element only (Tailwind's `--tw-*`). Substitution is lightningcss's own `UnparsedProperty::substitute_variables`, and the property is then re-parsed, which reduces `calc()`.
3. Colours: OKLCH to hex is rafters' own code, not lightningcss's and not the toy's. A layer that presents the system runs the system's code. Before the Rust step, `tools/rafters-colors.mjs` replaces every `oklch(...)` literal in the compiled CSS (107 distinct token values) with the hex from `@rafters/color-utils`: `toGamutCeiling(c, 'srgb')` first, since `oklchToSRGB`'s contract asks for that on an out-of-gamut colour, then `oklchToSRGB`. A literal with alpha becomes `rgba()` from the same components. The package is imported from the rafters checkout, read only, and run under its own `tsx` (its source imports `./x.js` names that are `.ts` files). The step takes 0.16 s and writes `css/huttspawn.rafters-hex.css`, which the transform reads; no `oklch(` is left in it. lightningcss still runs `CssColor::to_rgb()` on whatever colour is left after substitution. For these samples that is nothing: hex stays hex, and a `color-mix()` over rafters' hex (an opacity modifier) is lightningcss's arithmetic. Printing targets IE 11, so alpha comes out as `rgba()` and never as 8-digit hex.
   - **Where the two disagree** (`out/color-diff.tsv`, from `t20-to-email lower`, which prints lightningcss's own lowering of each literal): 6 of 107 literals, by at most 4 per channel. 4 of the 6 are outside sRGB, where rafters reduces chroma at the same lightness and hue (`toGamutCeiling`) and lightningcss uses CSS Color 4 mapping (`map_gamut`): `oklch(0.499 0.237 27)` is `#bb0113` against `#bd000f`, `oklch(0.857 0.186 53)` is `#ffc19a` against `#ffc097`, and two more differ by 1. The other 2 are in gamut and differ by rounding: `oklch(0.063 0.044 264)` is `#000009` against `#000007`, and `oklch(0.755 0.153 232)` is `#08bfff` against `#09bfff`. None of the 6 is a colour these samples use, so both emails are byte-identical either way.
4. `rem` becomes `px` at 16px.
5. `@supports` rules count as inlinable and win by source order. The transform has already computed the `color-mix` they guard, so the designer's colour is the one that lands.

Email:
6. Logical properties become physical, left to right: `padding-inline` becomes `padding-left`/`padding-right`, and so on.
7. Dropped, each with its reason in `out/*.report.json`: transition and animation, cursor, `scale`, `scroll-margin`, `container-type`, `position`, `flex-shrink`, outline; `:focus-visible`, `:active`, `:disabled`, `[aria-*]` and `:has()` variants; and every `@container` rule (no mail client evaluates one, so the base value stands).
8. Kept in `<style>`: `:hover`, without Tailwind's `(hover: hover)` wrapper, and dark mode as one `@media (prefers-color-scheme: dark)` block. Each element that needs either gets a generated class `rN`, and the values are `!important` so they beat the inline style. The dark block repeats hover with the dark values, so a dark reader does not hover to the light colour. Also `<meta name="color-scheme" content="light dark">` and `:root{color-scheme}`.
9. Layout is tables. The outer table takes the first container's `max-width` (`max-w-2xl`, 672px) with `align="center"`, inside an MSO-only fixed-width table, because Outlook Windows ignores `max-width` on anything but a table. A flex column becomes one row per child, with the gap as `padding-top` and the container's padding on each cell. A grid becomes one cell per column at `100/n`%, with the gap split as padding between cells.
10. Text elements get `margin:0` (preflight's job, which mail clients do not do) and repeat `font-family` from the body, because Outlook does not carry it into nested tables. Without a size of their own they get 16px, the browser default the design inherited. Unitless `line-height` and `em` `letter-spacing` become px, and `mso-line-height-rule:exactly` is added; caniemail flags both for the Word engine.
11. The image folds figure, frame and img into one `<img>` with the frame's radius, a `width` attribute (the block's `meta.width`, at most the outer width) and `border:0`.
12. The button is a table cell carrying the surface (`bgcolor`, `background-color`, radius, and `mso-padding-alt` for Outlook) around an `<a>` carrying the padding and text. Its `h-11`/`h-12` becomes vertical padding, (height minus line-height) / 2: 10px and 12px. `inline-grid` becomes `inline-block`, or `block` with a 100% table when the parent is a stretching flex column; see the screenshots.
13. The separator is a one-pixel table cell.

## 4. Results

`out/*.html` are the emails. `out/*.report.json` list every declaration inlined, kept in `<style>` or dropped, with the class it came from and the reason.

| | Measured | Predicted |
|---|---|---|
| lightningcss parses the compiled huttspawn CSS | no error, with error recovery off | no error |
| `var(` in the output | 0 in both emails | 0 |
| `oklch`/`lab`/`color-mix`/`color(` in the output | 0; a `bg-muted/80`-style rule becomes `rgba()` (unit test `an_opacity_modifier_lowers_to_rgba`) | 0 |
| caniemail `n` among the inlined properties | Apple Mail 0, Gmail 0, Outlook Windows 2 (`border-radius`, `ui-sans-serif`/`system-ui` in the font stack) | Apple 0, Gmail 2 or fewer, Outlook Windows 5 or more |
| Variant classes that cannot be inlined | `:hover` kept; focus, active, disabled, aria, `:has` and all `@container` dropped | the same |
| Gap | became table cell padding | the same |
| Dark mode re-emitted from `.dark` | `y` in Apple Mail and Outlook.com (where note 3 says Outlook.com also adds `data-og*` attributes for its own dark mode); `n` in Gmail and Outlook Windows | works in Apple Mail, not Gmail |
| CSS load (parse + index, 117 KB after the rafters hex step) | median 0.91 ms, max 1.34 ms (200 runs) | under 20 ms in total |
| Render one email after the load | newsletter median 0.24 ms, transactional 0.17 ms | under 2 ms |
| rafters colour step (Node + tsx, once per token sheet, not per email) | 0.16 s | |
| Whole process, one email | under 10 ms (`time` reports 0.00 s) | |
| Release binary | 4,157,216 bytes (3.96 MiB) | under 4 MB |
| react-email cannot take the CSS-first theme | its loader serves Tailwind's own files only; the huttspawn values went in a JS config | the same |
| Size of the toy | 1,169 lines of Rust (about 1,110 without tests), 3 dependencies | about 700 lines, 3 dependencies |

Predictions 1, 2, 3, 5, 6, 7, 8 and 10 held. Prediction 3 holds for the samples, and its `color-mix` risk is shown only by a unit test, since neither sample uses an alpha class. Prediction 4 missed on Outlook: only 2 inlined properties are `n`, because the rest of what Outlook lacks sits in `<style>` and the HTML (counted below). Prediction 9 missed: 4.16 MB in decimal megabytes. Prediction 11 missed: the line count came in about 60% over.

### caniemail (the data, not a client)

Data from `hteumeuleu/caniemail` at `cb8109c`, read from `_features/*.md` by `tools/caniemail.mjs`, taking the newest tested entry per client. caniemail has no separate key for the new Outlook for Windows: its Outlook platforms are `windows`, `windows-mail`, `macos`, `outlook-com`, `ios` and `android`. New Outlook runs on Outlook.com's engine, so `outlook-com` stands in for it. Full tables are in `out/caniemail.txt` and `out/caniemail.json`.

Not supported (`n`) anywhere in each of our emails, both emails alike:

| Client | `n` | What |
|---|---|---|
| Gmail web | 4 | `prefers-color-scheme`, `color-scheme` property, `color-scheme` meta, `target` (forced to `_blank`) |
| Gmail iOS / Android | 4 / 3 | `prefers-color-scheme`, `color-scheme` property, `target`, and `:hover` on iOS |
| Outlook Windows (Word) | 8 | `@media`, `prefers-color-scheme`, `border-radius`, `color-scheme` property and meta, `:hover`, `system-ui`/`ui-sans-serif`, HTML5 doctype |
| Outlook.com / new Outlook | 3 | `color-scheme` property and meta, `target` |
| Apple Mail macOS / iOS | 0 / 1 | the `color-scheme` meta on iOS 12.4 |

Partial (`a`) entries that matter here:
- Gmail iOS and Android with non-Google accounts get no `<style>`, so no hover and no dark.
- Gmail web caps `<style>` at 16 KB; ours is under 600 bytes.
- Outlook Windows: `padding` only on table cells (we only pad cells and the link), `max-width` only on tables (hence the MSO table), `font-weight` 500 renders as normal, `letter-spacing` and `line-height` need px and `mso-line-height-rule`, and `!important` is not supported inline (we only use it in `<style>`).

Not in caniemail's data at all: `font-family`, `color`, `mso-line-height-rule`, `mso-padding-alt`, `supported-color-schemes`, the `bgcolor` attribute and the `:root` selector.

### Screenshots (a browser, not a mail client)

`out/shots/` holds 24 screenshots. Each email, its class-based reference page (`out/*.reference.{light,dark}.html`, the same blocks with the site's classes on the site's compiled CSS) and the react-email output are shot at 600px, light and dark, in Chromium 1243 and WebKit 2359. These are browsers standing in for mail clients. WebKit is the engine closest to Apple Mail, but nothing here shows what Gmail, Outlook or Apple Mail actually do; that needs a real-client service (Litmus, Email on Acid) or real inboxes.

- **Colour:** `tools/colors.mjs` reads the computed background and text colour of body, h1, p, button and separator in each reference and each email, both schemes, and turns them into sRGB bytes through a canvas. The worst channel difference is 0 over 36 pairs. 8 of those pairs are the transparent h1 and p backgrounds and compare nothing, so 28 do. The reference is Chromium resolving the same compiled CSS, not the deployed site. What this shows is that lightningcss's variable resolution and OKLCH conversion agree with the browser's, byte for byte. One run of that script caught the reference button mid-transition. The site's button carries colour transitions, so the script now waits for the page to settle.
- **Layout, where they differ:** the reference h1 is `text-5xl` (47.8px) and the email's is `text-4xl` (39.8px). The reference button is 40px tall and the email's 44px. Both come from container queries (`@lg:text-5xl`, `@md:h-10 @md:py-2`) that apply in the browser at 600px and are dropped in the email. The first build also showed the button at content width where the site stretches it to the full column. That is fixed (choice 12), and so is a body that stayed light in dark mode.

### react-email, same content, same class strings

`tools/react-email.mjs` renders both samples with the rafters class strings verbatim and a config holding the huttspawn light values our transform resolved. Output is in `out/*.react-email.html`.

| | t20 (Rust) | react-email 2 |
|---|---|---|
| Bytes, transactional / newsletter | 4,065 / 5,279 | 4,234 / 5,289 |
| Render, warm | 0.19 / 0.26 ms (plus 0.94 ms CSS load) | 1.38 / 1.49 ms median of 50, Node 26 |
| `var(` left | 0 | 14, in the focus-ring rules in `<style>` |
| `rem` left | 0 | `3rem`, `1.75rem`, `42rem`, `5rem` (Outlook Windows: rem not supported for font-size) |
| Dark mode | `prefers-color-scheme` from `.dark` | none |
| Container query `@lg:text-5xl` | dropped (base `text-4xl` stands) | inlined as base: h1 at 3rem |
| `aria-disabled:opacity-50`, `aria-busy:cursor-progress` | dropped | inlined as base: the button renders at 50% opacity (see `out/shots/transactional.react-email.light.webkit.png`) |
| Button display | `inline-block`/`block` in a table cell | `display:inline-grid` with `align-items`, `justify-content`, `gap` inline |
| Outlook button padding | `mso-padding-alt` on the cell | hair-space `<i>` hack inside `<!--[if mso]>` |
| caniemail `n`, Outlook Windows | 8 | 9 to 10, among them `opacity`, `box-shadow`, `gap`, `align-items` |

react-email's attribute-selector variants leak because `is-rule-inlinable.ts` only treats pseudo selectors and at-rules as non-inlinable; `[aria-disabled="true"]` is neither.

## What works, with evidence

- lightningcss is the CSS engine: it parses, substitutes variables, reduces `calc()` and prints. rafters' `@rafters/color-utils` turns the OKLCH tokens into hex. There is no hand-written colour maths (unit tests in `src/css.rs`; `out/color-diff.tsv`; 0 `var(`/`oklch` in the output).
- Designer tokens survive exactly: 0 channel difference against a browser resolving the same stylesheet with the site's classes, in light and dark, over 28 compared colours (`tools/colors.mjs`).
- `.dark` becomes `prefers-color-scheme` without the designer doing anything. caniemail marks that `y` in Apple Mail and Outlook.com; Outlook.com also adds `data-og*` attributes for its own dark mode, which this toy does not target.
- Every dropped declaration is named, with its class and reason (`out/*.report.json`). That is the list a designer or a lint would need.
- Fast: about 1 ms to load the sheet, then a quarter of a millisecond per email, in a 4 MB binary.

## What a mail client would break, with evidence

- **Gmail (all) and Outlook Windows:** no dark mode. `prefers-color-scheme` is `n` in both. What those clients do to a light email in a dark-mode reader (forced inversion, for one) was not checked here and needs a real client.
- **Outlook.com / new Outlook:** no hover. caniemail: `:hover` is "only supported on type selectors", and ours is `.rN:hover`.
- **Gmail apps with non-Google accounts:** no `<style>` at all, so no hover and no dark; only the inline light design shows.
- **Outlook Windows:** no `border-radius` (moot for huttspawn, whose radius base is `0.01rem`, so the button radius is 0.19px), no `:hover`, and no `@media`. Weight 500 renders as normal (caniemail `font-weight` note), so the button label loses its medium weight. The `ui-sans-serif, system-ui` part of the stack is ignored, so the fallback is whatever Outlook picks after `Inter`, which no mail client loads.
- **Everywhere:** `Inter` is not loaded (no `@font-face` is emitted), so every client falls back down the stack.

## What the design must decide

1. **Container queries.** The components respond to their container (`@lg:text-5xl`, `@md:h-10`, `@md:px-6`), and no mail client evaluates `@container`. The toy drops them, so the email shows the small-container design even at 600px (h1 39.8px against the site's 47.8px). The email's width is known (672px max), so the transform could evaluate them statically at that width and emit a `max-width` media query for narrow screens. That is a rule about what the designer meant, not a CSS question.
2. **Dark mode where it cannot be expressed.** Gmail and Outlook Windows ignore `prefers-color-scheme`. Should the email declare `light only`, or accept each client's automatic inversion of the light palette?
3. **The email width.** The container token says 672px (`max-w-2xl`), against the industry's 600px. Outlook Windows gets a fixed 672px table.
4. **Fonts.** Inter is the token but cannot be loaded in most clients. Name an email font stack, or a web font for Apple Mail only?
5. **Hover that does nothing.** `--primary-hover` (primary-600) equals primary (primary-500) to three decimals in huttspawn's light tokens (`oklch(0.4507 ...)` against `oklch(0.451 ...)`), so the light hover rule is a no-op. This is a token question, not a transform one.
6. **Interaction states.** Focus, press, disabled and busy are dropped as meaningless in email. If any must show (a disabled "already verified" button), it has to be a separate variant of the block, not a state.
7. **Which components exist in email at all.** Only container, grid, heading, text, image, button and separator have an email rendering here; anything else is dropped and reported. The email set is a list rafters would own.

## Part 2: send-ready output

Predictions 12 to 18 in `PREDICTIONS.md` were committed before this part was built.

### What `@react-email/render` does

Read from the 2.2.0 source map (`node/render.tsx`, `shared/utils/to-plain-text.ts`, `strip-image-preload-links.ts`, `unstable-to-plain-text.ts`). It does no CSS. It does five things:
- server-renders the React tree inside an error boundary and `Suspense`;
- strips the `<link rel="preload" as="image">` tags React adds for every `<img>`;
- replaces any doctype with XHTML 1.0 Transitional;
- with `plainText`, converts the HTML to text through html-to-text (`wordwrap: false`, images and `data-skip-in-text` skipped, a link's URL after its text unless they are the same);
- with `pretty`, formats it through Prettier.

The send-ready pieces in react-email live in its components:
- `Html`: `lang="en"` and `dir="ltr"` by default;
- `Head`: `<meta http-equiv="Content-Type" content="text/html; charset=UTF-8">` and `<meta name="x-apple-disable-message-reformatting">`;
- `Preview`: a preheader `div` hidden with `display:none; overflow:hidden; line-height:1px; opacity:0; max-height:0; max-width:0` and `data-skip-in-text`, padded to 150 characters with seven invisible characters repeated;
- `Img`: `display:block; outline:none; border:none; text-decoration:none`.

### What a send from a .rafter needs, and the decision on each

All of it is built in the Rust core: `src/send.rs`, with small changes in `email.rs` and `main.rs`. `render` now also writes `out/<name>.txt` and `out/<name>.send.json`.

1. **A plain-text alternative. Built,** from the blocks, not from the HTML. Headings and paragraphs come out as written. A button is its label over its URL, so the URL sits alone on its line and stays whole. A separator is 40 dashes, as html-to-text writes `<hr>`. Images and the preheader are left out, as react-email does.
   - Evidence: `tools/send-check.mjs` finds every heading, paragraph, button label and `href` in both texts, and no image alt and no preheader text. It takes 0.001 ms per email.
   - Against react-email's `plainText` output (`out/*.react-email.txt`): the same words and order. Two differences, both choices: react-email uppercases headings (html-to-text's default) and puts the URL on the button label's line. We keep the designer's case.
2. **The doctype. Kept as HTML5.** caniemail: the HTML5 doctype is `y` in Gmail, Outlook.com and Apple Mail, and Outlook Windows ignores any doctype ("has no impact here").
   - Evidence: both emails, screenshotted at 600px in Chromium and WebKit with our doctype and with react-email's XHTML 1.0 Transitional one, are byte-identical in all 4 pairs. Both doctypes give `compatMode` `CSS1Compat` (`out/send-check.txt`).
   - XHTML Transitional's almost-standards mode differs only for images inline in table cells. Ours are `display:block`, so it has nothing to change.
3. **A preheader (the inbox preview line). Built** from a new `preview` block, whose content is the line. It is a block because the composite format has no field for it: `manifest.description` is written for designers, and the samples' descriptions say so. It is the first thing in `<body>`: react-email's hidden-`div` recipe plus `mso-hide:all`, padded to 150 characters with react-email's invisible characters.
   - Evidence: caniemail shows `display:none` failing in Gmail's mobile webmail (`n`) and partial in Outlook Windows (not on nested tables or images; ours is neither). That is why the other properties are there. Outlook Windows has `max-height`, `opacity` and `overflow` at `n`, which leaves it on `display:none` and `mso-hide` (the latter is not in caniemail's data).
   - In Chromium and WebKit the preheader computes to `display:none`, 0px tall, and is absent from the page's visible text.
   - Whether each inbox shows the line as its preview was not checked, and needs real clients.
4. **Lines under 998 octets. Built:** RFC 5322 2.1.1 caps a line at 998 octets, and a server may break a longer one wherever it likes. `wrap_lines` breaks any line past 900 octets at a space a newline cannot change: between a tag's attributes, outside quotes, or between words of text. The padding gets a plain space between groups so it can break too.
   - Evidence: our longest lines are 914 and 911 octets, and a 3,500-character paragraph wraps with its attributes untouched (unit test `a_long_paragraph_wraps_under_998_octets_without_touching_attributes`).
   - react-email's render writes each document as one line, 7,018 and 6,020 octets, so it depends on the sender's transfer encoding to survive.
5. **Absolute URLs. Built:** a relative `href` or `src` fails the render, naming the block (`block cta: href "/verify?token=x" is not an absolute URL`, exit 1). caniemail: `<base>` is removed by Gmail (all `a`) and Outlook.com, so a mail client has nothing to resolve a relative URL against. `https:`, `http:` and `mailto:` pass.
6. **Subject. Built,** in the send output: `out/<name>.send.json` is `{ subject, preview, text, html }`. That is the `{ subject, text, html }` that huttspawn's `@rafters/platform-sender` `render` already returns, plus the preview. The subject is `manifest.name`, the only reader-facing title the format has; see "What the design must decide" below.
7. **`dir="ltr"` on `<html>`. Built.** The transform turns logical properties into left-to-right physical ones, so the document should say so. caniemail: `dir` is `y` in Gmail, Outlook.com and Apple Mail, and partial in Outlook Windows (buggy on `<a>`; ours is on `<html>`).
8. **`x-apple-disable-message-reformatting`. Built.** It is react-email's `Head` default, and caniemail has no entry for it. It is one meta tag and harmless where it is ignored; its effect in Apple Mail was not checked here.
9. **The Content-Type meta. Not built.** We keep `<meta charset="utf-8">`. A sent message's charset comes from its MIME part's `Content-Type` header, which the sender writes, and caniemail has no entry for either meta.
10. **Image preload links. Not applicable.** The Rust core does not server-render React, so there are none to strip.
11. **Pretty-printing. Not built.** The wrap above is what a send needs; formatting for people to read is not.
12. **Not built and not the transform's to build:** the `List-Unsubscribe` header and an unsubscribe link. These are bulk-sender requirements of mailbox providers, not caniemail data, so they are not shown here. The newsletter sample has no unsubscribe block, and the header is the sender's. The MIME envelope (`multipart/alternative` of text and HTML, transfer encoding) is also the sender's, given `{ subject, text, html }`.

### Part 2 results

| | Measured | Predicted |
|---|---|---|
| Plain text from the blocks | every heading, paragraph, label and URL; no alt, no preheader; 0.001 ms | the same, under 0.1 ms |
| XHTML doctype against HTML5 | screenshots byte-identical, 4 of 4 pairs, both `CSS1Compat` | no visible difference |
| Preheader hidden | `display:none`, 0px, not in visible text, Chromium and WebKit; caniemail `display:none` `n` in Gmail mobile webmail | hidden; `display:none` fails in at least one client |
| Longest line | ours 914 / 911 octets; react-email 7,018 / 6,020 | under 998 for us, over for react-email |
| A relative link | render fails, naming the block | fails with a named error |
| Send shape | `{ subject, preview, text, html }` | `{ subject, text, html }` |
| Rust added | 164 lines (1,658 to 1,822, about 30 of them tests), no dependency | under 250 lines, no dependency |

All seven held. Prediction 17 held with one addition, `preview`, which platform-sender's shape does not carry because its emails have no preheader.

The new pieces add Outlook Windows `n` entries (`max-height`, `opacity`, `overflow`, all in the preheader), for 11 in all. They are there for other clients, and Outlook hides the preheader with `display:none` and `mso-hide`. Both emails grew by about 1,750 octets, nearly all of it the preheader padding. Release binary: 4,157,232 bytes, +16.

### What the design must decide, part 2

8. **The subject.** The toy uses `manifest.name`, but a composite's name is a designer's label. A send needs a reader-facing subject, maybe with data in it (`props`, as `$bind` allows elsewhere). Where does it live in the format?
9. **The preview line.** The toy adds a `preview` block type. Is that the format's answer, or a manifest field?
10. **Headings in plain text.** Keep the designer's case, as here, or uppercase them as html-to-text does to mark them?
11. **Unsubscribe.** A newsletter needs an unsubscribe link in its body and its sender needs `List-Unsubscribe`. Is the link a rafters block that every bulk composite must contain?

## Limits

- Class strings are transcribed, not computed: the toy does not run the components' `*.classes.ts`.
- No real mail client was used; the screenshots are browsers.
- Custom properties do not inherit from a parent element. No sample needed it, since Tailwind's `--tw-*` are `inherits: false`.
- Left to right only for logical properties.
- One sample image is served from the site's `public/` folder for the screenshots; the email links to `https://huttspawn.com/images/bounty-hunter.png`.
