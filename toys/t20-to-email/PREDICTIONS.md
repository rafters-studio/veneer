# T20 predictions

Written before any build. The question: can veneer's Rust core turn a `.rafter` file (the rafters composite format) into email HTML that real mail clients render as the designer meant, with lightningcss doing the CSS?

The shape under test: a Rust binary reads a `.rafter` file, maps each block to the classes the installed huttspawn rafters component would put on it, reads Tailwind's compiled output for those classes, resolves custom properties, lowers colours, inlines each element's declarations into `style`, and keeps in `<style>` only what cannot be inlined. Layout becomes tables where email needs them.

1. lightningcss parses Tailwind 4.3's compiled output for the huttspawn theme with no error and no rule it drops. Confidence 0.9.
2. Every `var()` in an inlined declaration resolves to a concrete value, so the output holds no `var(` at all. The hard ones are the `calc()` chains in the rafters tokens and the `--tw-*` variables that only have an `@property` initial value. Confidence 0.7.
3. Every colour in the output is hex or `rgb()`/`rgba()`: no `oklch`, `lab`, `color-mix` or `color()` survives. The opacity modifiers (`bg-muted/80`) are the risk. Confidence 0.8.
4. Against caniemail's data, the inlined properties have: Apple Mail 0 unsupported; Gmail (web and apps) 2 or fewer; Outlook Windows (Word engine) 5 or more, `border-radius` among them. Confidence 0.6.
5. `hover:`, `focus-visible:`, `active:`, `disabled:`, `aria-*:` and `@md:` container-query classes cannot be inlined. Pseudo-class rules go to `<style>`; container queries are dropped because no mail client supports them. Confidence 0.85.
6. Flex and grid `gap` cannot be carried as CSS; the two-column grid and the stacked containers have to become tables with padding standing in for the gap. Confidence 0.85.
7. The `.dark` class overrides in rafters.css can be re-emitted as `@media (prefers-color-scheme: dark)` rules with `!important`. caniemail will show that works in Apple Mail and not in Gmail. Confidence 0.6.
8. Transform time per email, warm, in release, including parsing the compiled CSS: under 20 ms; the inline-and-emit step alone under 2 ms. Confidence 0.6.
9. The release binary is under 4 MB. Confidence 0.55.
10. react-email's Tailwind component cannot read the rafters CSS-first `@theme` directly: its stylesheet loader serves only Tailwind's own files and its theme comes from a JS config. A fair comparison needs the huttspawn values passed in a config. Confidence 0.8.
11. The toy's Rust comes in near 700 lines, with three dependencies: lightningcss, serde, serde_json. Confidence 0.5.

## Part 2: send-ready output

Written before building part 2. `@react-email/render` 2.2.0 does no CSS; I read `render.tsx`, `to-plain-text.ts` and `strip-image-preload-links.ts` from its source map. It does four things:
- server-renders the React tree;
- strips React's image preload links;
- swaps the doctype for XHTML 1.0 Transitional;
- optionally converts to plain text with html-to-text, and optionally pretty-prints.

Its components add the rest:
- `Head`: a Content-Type meta and `x-apple-disable-message-reformatting`;
- `Html`: `lang` and `dir`;
- `Preview`: a hidden preheader padded to 150 characters.

The question: what must a send-ready email from a `.rafter` contain that T20 lacks?

12. A plain-text alternative is built from the blocks in Rust, not from the HTML. It carries every heading, paragraph and link URL, and no image or preheader text. It runs in under 0.1 ms per email. Confidence 0.8.
13. Switching to the XHTML 1.0 Transitional doctype makes no visible difference to our emails in Chromium or WebKit, so the HTML5 doctype stays. caniemail says Outlook Windows ignores any doctype. Confidence 0.7.
14. A preheader comes from a `preview` block. It is hidden with `display:none` plus zero height, zero opacity and `overflow:hidden`, and it does not show in Chromium or WebKit. caniemail shows `display:none` failing in at least one tracked client, which is why the other properties are there. Confidence 0.7.
15. Every line of the HTML stays under RFC 5322's 998-octet limit by construction, even for a 3,000-character paragraph. react-email's render emits the whole document on one line, so its output breaks the limit for both samples. Confidence 0.85.
16. A relative `href` or `src` fails the render with an error that names it. Confidence 0.9.
17. The send output takes the shape huttspawn's sender already takes from `@rafters/platform-sender`: `{ subject, text, html }`. Confidence 0.75.
18. Part 2 adds under 250 lines of Rust and no dependency. Confidence 0.6.
