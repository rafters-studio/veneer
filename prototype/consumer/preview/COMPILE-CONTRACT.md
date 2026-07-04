# Composite compile contract (proven by hand-target, 2026-07-04)

What veneer's generator must emit for a `.composite.json`, established by
hand-writing `search-box.compiled.mjs` and driving it in Chrome against
hostile page CSS. Each rule below was forced by an observed failure or a
consumer requirement, not designed speculatively.

## Module contract (components.jsonl consumers)

1. **`define(sheet)` is the single entry point.** Consumers (docs pages,
   studio, hosts) build ONE `CSSStyleSheet` and pass it in. Modules never
   fetch and never embed the stylesheet. Shared asset, stored once.
2. **Typed camelCase property setters** on every element, re-render on set.
   `veneer:prop-change` (bubbles, composed, `{prop, value}`) fires on
   USER-driven changes only; programmatic sets re-emit rule edges but
   suppress the event. (Standing kelex/gitpress contract, now proven live.)

## Sheet adaptation (compile step, deterministic text transforms)

3. **ONE stylesheet input: `rafters.standalone.css`.** It carries the
   complete variable chain (semantic aliases -> `--rafters-*` -> palette;
   434 definitions), but emits the semantic and alias blocks under bare
   `:root`, which never matches inside a shadow tree -- only the Tailwind
   theme block gets `:root,:host`. The compile step applies one
   deterministic transform: `:root` -> `:root,:host` where Tailwind did not
   already. Never import a second sheet; `rafters.css` is Tailwind source,
   not a runtime input. Upstream one-liner requested of rafters:
   `registryToCompiled` emits `:root,:host` throughout, which deletes this
   transform entirely.
   (Observed failure: `bg-primary` resolved to nothing; buttons transparent.
   First workaround wrongly imported rafters.css for its variables -- the
   mess was reading the wrong file when the right file had them under the
   wrong selector.)
4. **Sever property inheritance with `:host{all:initial}`,** then re-establish
   the token font baseline (`font-family: var(--font-sans)`, size, line-height,
   `color: var(--foreground)`). `adoptedStyleSheets` stop selector matching
   but inherited properties flow into shadow trees; custom properties are
   exempt from `all` per spec, so the variable layer survives the reset.
   (Observed failure: hostile `font-size: 90px !important` rendered a giant
   preview.)
5. **The composite surface paints its own ground** (`bg-background` wrapper
   inside the shadow root). Document rules targeting the host element outrank
   `:host` styles by spec and cannot be beaten; opaque surface paint makes
   host-level hostile paint irrelevant.
   (Observed failure: red host background showing through transparent rows.)

## Rule dataflow (compiled from manifest edges)

6. **Producers dispatch `rafters:rule`** (bubbles within the composite shadow
   root, NOT composed) with `{rule, value, valid, sourceBlock}`. Applied rules
   (`block.rules`, e.g. `required`) are evaluated at the producer and set
   `valid`; consumers decide behavior on validity (submit button disables on
   invalid query).
7. **One router per composite root,** compiled from the manifest's edges into
   a static `rule -> [consumer block ids]` table. Consumers expose a
   `ruleInput(rule, value, valid)` intake. No block listens globally.
8. **Boundary compilation** (matches `deriveCompositeBoundary`): unconnected
   inputs become typed camelCase host properties (`search-pending` ->
   `searchPending`); unconnected outputs re-dispatch from the host as
   composed CustomEvents named by the rule (`search-submitted`). The host app
   never sees internal edges.
9. **Unfed consumers start in their honest empty state** (button disabled,
   spinner hidden). Display follows dataflow; there is no separate visibility
   vocabulary. "Empty until fed" is the component's own design claim.

## Editor contract (the second solve)

10. **`KNOBS` export**, FieldDescriptor-shaped: boundary inputs (typed host
    props) plus block props from the classes vocabulary. Enum options are
    limited to variants whose classes the compiled sheet actually covers --
    the editor can only offer what can render truthfully.
11. **`MANIFEST_META` export** carries id, cognitiveLoad, boundary I/O, and
    verbatim DO/NEVER through to the editor surface.

## Environmental notes

- `prefers-reduced-motion` is respected end to end (`motion-reduce:` variants
  in the class vocabulary reach the compiled sheet); a spinner with
  `animation: none` under reduced motion is correct behavior, not a defect.
- Verified consumption chain: real `rafters@0.0.71` install ->
  `rafters init` (810 tokens) -> `rafters add` (8 components + 3 primitives)
  -> `exports.compiled` -> `rafters.standalone.css` -> adapted shared sheet ->
  isolated, rule-wired, knob-controllable composite preview.

## Corrections after reading the real primitives (2026-07-04, second pass)

Operator-caught: the first hand target substituted shadcn/tailwind training
patterns for rafters' own layer in two places. Read first, always.

12. **Emit `RaftersElement` subclasses, not raw HTMLElement.** The installed
    `lib/primitives/rafters-element.ts` already defines the WC base: shadow
    setup, per-component `static styles`, and `setUtilityCSS()` /
    `setTokenCSS()` shared constructable sheets. Veneer's `define(sheet)`
    entry point maps onto `RaftersElement.setUtilityCSS(adaptedSheetText)`
    at consumer init; contract rules 1-5 stand but are DELIVERED through the
    rafters base, not a parallel one.
13. **Applied rules compile to native HTML5 constraint attributes** via the
    registry's `ruleToAttrs` mapping (`required` -> `required`, `email` ->
    `type="email"`, parameterized `pattern`/`minlength`). No JS validator
    predicates in emitted code, ever -- edge validity is read from the
    browser's own `element.validity`. (First target shipped a JS `required`
    re-implementation; deleted on correction.)
14. **The runtime rule bridge is a PROPOSAL, not an existing rafters
    surface.** Static compatibility (`matchRules`), build-time attributes
    (`rule-attrs`), and editor palette/serialization (`bridge.ts`) exist
    upstream; live event dataflow between block instances does not. The
    `rafters:rule` event protocol (rules 6-8) is veneer's proposed runtime
    realization of the manifest's edges -- to be ratified with rafters
    before it hardens into components.jsonl artifacts.
15. **Editor integration rides `bridge.ts`**: palette items from manifests,
    fresh-ID instantiation (`instantiateBlocks`), serialize-back. Veneer's
    KNOBS descriptors complement that surface (live prop/boundary controls
    on an instantiated preview); they do not replace it.
