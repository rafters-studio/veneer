# T16: which freehand-element rule is worth shipping

Toy for #159 (`rafters(freehand-div)`) and #158 (`scan`). It builds neither.

## Predictions, written before any combination ran

What I had seen when writing these: issues #158 and #159, `lint.ts`, `read.ts`, the file listing of
each site, `container.astro` and `grid.astro` in smugglr.dev and huttspawn, and the first 30 lines of
huttspawn's `hs-card.astro`. No element had been counted.

Facts already known from that reading, which shape the predictions:

- mail.rafters.studio holds only `node_modules`. gitpress.app has 2 `.astro` files and no components.
- The newest Container (huttspawn) stacks with `gap` (`flex flex-col gap-*`), takes `columns`, and
  renders as main, header, footer, section, article, aside or div. Older copies stop at
  div, main, section, article and aside, and carry only two of the six tags.
- No site has a Flex or Stack component. A flex row has no Container or Grid equivalent.

| # | Prediction | Reason |
|---|---|---|
| P1 | Strict qualification matches 0 custom components on every shingle site and 10 to 14 on huttspawn. | Only huttspawn has `hs-*.behavior.ts`; its `hs-card` carries all six tags. |
| P2 | As a control, strict matches 0 of the system components on shingle sites (2 tags, no behavior file) and most on huttspawn. | Container and Grid in smugglr.dev have 2 tags. |
| P3 | Loosening to "behavior and classes, no tags" adds under 3 components anywhere. "Classes only" adds `hs-panel`. Dropping the file requirement adds nothing. | Nobody writes six tags without the files. |
| P4 | `system` and `format` differ only on huttspawn. `all` covers at least 90% of files holding a div, because every page uses a Layout declared in the project. So `all` makes the "uses a component" gate worthless. | Layout is a component declared in source. |
| P5 | #159 as written (every div, in a file using a component in scan, `format`): precision 15 to 25%. | Most divs are inner structure and flex rows. |
| P6 | Divs with the listed layout classes: precision 40 to 50% overall. Split: `grid`/`grid-cols` 85%, `mx-auto`/`max-w-*`/`container` 90%, `flex` rows under 20%, `flex flex-col gap` 60% where Container stacks. | Flex rows have no replacement. |
| P7 | Divs with visual utilities and no layout class: under 15%. | Those are chrome, closer to Card than Container or Grid. |
| P8 | Divs with no class, or wrapping one child: about 10%. What helps there is deleting the div, so "use Container or Grid" misleads. | A bare div carries no layout decision. |
| P9 | Direct parent cuts the file-gate count by 60 to 70%, and adds about 10 points of precision. Ancestor cuts it by 20 to 30%, and adds under 5 points. | Most divs in a page have some component below them, few have one right below. |
| P10 | `section` and `main` with layout classes beat div on precision; bare `section`, `header`, `footer`, `nav` are noise (semantic and fine as written). | Container takes `as`, but a bare landmark loses nothing. |
| P11 | Raw equivalents: count dominated by `p` and `a`. Precision: `button` and `input` over 70%, `h1`-`h6` around 60%, `p` around 40%, `a` and `img` under 30%. Only meaningful where the component is installed. | Typography exists on most sites. Links have no component. |
| P12 | The rule's walk costs under 100 ms per site; check time is dominated by the type check: 5 to 60 s per site. | One tree walk against a whole-program check. |
| P13 | Line grep for `<div` differs from the program's count by 5 to 15%, overcounting on code samples in docs pages. | Docs sites show markup in strings. |
| P14 | Veneer's raw diagnostics match `astro check` errors on the same files, and rafters-filtered findings are a small subset. | T11 and the requirement say it is the same checker. |
| P15 | Under `scan: all`, existing rules gain few findings (under 10 type errors per site). | Sites already pass `astro check`-style review; the error count is low. |

## Outcomes

To be filled in after each run.
