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

## A write I should not have made

The brief said never to write to the site repos. I did, through node_modules.

`prep.mjs` first linked every node_modules entry, `.bin` included, into the scratch copies. `baseline.mjs`
then ran `pnpm exec astro check` in those copies at 00:22 PST on 2026-11-03. pnpm's install-before-run
rewrote bin shims through the link:

- `huttspawn/site/node_modules/.bin`: 11 shims (astro, astro-check, cf-wrangler, cucumber-js,
  platform-auth-migrations, toolchain, tsc, vp, vpr, wrangler, wrangler2). All 11 hold the scratch path.
  The `.pnpm` folder's modified time also changed; no new entry shows.
- `shingle/sites/sean.silvius.me/node_modules/.bin`: 14 shims (astro, astro-check, cf-wrangler, oxfmt,
  oxlint, playwright, playwright-core, serve, tsc, tsserver, vite, vitest, wrangler, wrangler2). I
  grepped only `astro`; the rest are inferred from their modified times.
- The other four sites' `.bin` folders show nothing modified on Nov 2 or 3.

The shims now resolve through the scratch folder, so they will break when it is removed. They were not
repaired, and the scratch folder was kept, on the coordinator's instruction: Sean decides the repair.
No package manager ran in a copy after that, and both scripts are changed so a rerun cannot repeat it.
The recorded `results/astro-check` came from the `pnpm exec` run; the changed script has not been rerun.

## What was run and how

- Facts: `facts.ts` opens each site with veneer's `openProject`, runs veneer's `lint`, and writes a row
  per JSX element from the same program. Helpers lint.ts does not export are copied verbatim and marked.
- Combinations: `combos.mjs` queries the rows offline. 10 scan definitions x 4 gates x 4 element sets x
  18 class filters per site and in total: `results/combos.csv`.
- Precision: I read every element of each family in non-exempt files, so it is a census of this corpus and
  not a sample: 94 divs, 59 sectioning elements, 206 raw elements (`results/census-*.txt`). Divs and
  sectioning have one verdict per line in `verdicts.mjs`. Raw elements were read, then given verdicts by
  grouped rules. One reader judged all of it, with no second opinion. Not read: the 44 divs in qualified
  huttspawn components, which every combination exempts. `results/precision.csv` has every combination.
- Text scans, labelled as such: the `<div` count in `sanity.mjs`, `mdx-divs.mjs`, and the line grep in
  `extra.mjs`.

Verdicts: **R** Container or Grid truly replaces it, with a gain. **W** a mechanical swap works but gains
little or needs overrides. **O** another rafters component is the answer, so the help text misleads.
**K** raw is right, or nothing installed replaces it. Strict precision is R/n, lenient is (R+W)/n.

Verdicts are judged against each site's installed Container. About ten `header` and `nav` verdicts are K because
that copy has no such mode. The current Container (huttspawn's) has header and footer modes and says never to
write raw sticky or z-* utilities, so `mail/SiteHeader.astro:17` breaks it and most `header` verdicts would
become W under current rafters. Dropping sectioning at warning still stands: W is a weak swap, not a finding.

## The corpus

| Site | Root files | Elements | Divs outside exempt files | Note |
|---|---|---|---|---|
| gitpress.app | 2 | 14 | 0 | no components |
| huttspawn | 134 | 974 | 2 | 44 more in qualified components |
| rafters.studio | 75 | 720 | 27 | |
| runlegion.dev | 49 | 295 | 24 | 21 in one hero scene |
| sean.silvius.me | 30 | 313 | 19 | 9 are identical placeholder panels |
| smugglr.dev | 55 | 503 | 22 | |
| mail.rafters.studio | - | - | - | holds only `node_modules`, nothing to open |

**The biggest limit on everything below:** these sites have already moved to rafters. Only 94 divs remain,
and no div has a grid class. Combinations with n under 20 are the whole population, not a sample of 20.
Nothing here measures an unmigrated project, where counts and precision could differ a lot.

## The table

Divs, `format`, all sites. Per-site counts are in `results/precision.csv`.

| Which divs | File gate (#159) n / files | Strict | Lenient | Every file n / files | Strict | Lenient |
|---|---|---|---|---|---|---|
| every div | 43 / 16 | 9% | 21% | 94 / 38 | 4% | 13% |
| any class | 40 / 16 | 10% | 23% | 90 / 38 | 4% | 13% |
| layout classes (as briefed) | 15 / 9 | 20% | 40% | 23 / 13 | 13% | 30% |
| visual utilities | 23 / 10 | 0% | 9% | 38 / 22 | 0% | 5% |
| no class | 3 / 2 | 0% | 0% | 4 / 3 | 0% | 0% |
| wraps one child | 9 / 8 | 11% | 22% | 17 / 15 | 6% | 18% |
| split: `grid`, `grid-cols-*` | 0 / 0 | - | - | 0 / 0 | - | - |
| split: `container`, `mx-auto`, `max-w-*` | 3 / 3 | 33% | 67% | 4 / 4 | 25% | 50% |
| split: `flex flex-col` | 7 / 5 | 43% | 86% | 9 / 7 | 33% | 78% |
| split: flex row | 8 / 6 | 0% | 0% | 14 / 9 | 0% | 0% |
| **grid, shell or flex-col** | **10 / 7** | **40%** | **80%** | 13 / 10 | 31% | 69% |
| **as recommended, warning tier** | **0 / 0** | - | - | 0 / 0 | - | - |
| **as recommended, hint tier** | **8 / 6** | **50%** | **88%** | 10 / 8 | 40% | 80% |
| holds text directly | 10 / 3 | 0% | 0% | 34 / 14 | 0% | 0% |

The last row is 0% for Container or Grid, but 31 of its 34 have another answer: 22 Typography, 9 Card.

Gates, every div: file 43 (R 4, W 5), ancestor 9 (R 3, W 2), parent 7 (R 2, W 1), every file 94 (R 4, W 8).

Scans, every div, file gate: `system` 43, `format` 43, `all` 46. Every gate: 94 for all three.

Other elements, file gate:

| Elements | n / files | R | W | O | K |
|---|---|---|---|---|---|
| section, main, article, aside, header, footer, nav | 46 / 26 | 3 | 17 | 0 | 26 |
| of those, with visual utilities | 16 / 16 | 0 | 0 | 0 | 16 |
| of those, no class | 22 / 9 | 3 | 17 | 0 | 2 |
| raw equivalents | 98 / 36 | 0 | 0 | 0 | 98 |

Raw equivalents in every file are 206: R 6, W 2. By tag: a 131 (R 0), p 29 (R 2), h2 13 (R 1), h1 12 (R 0),
img 8 (R 0), button 7 (R 2), h3 5 (R 0), table 1 (R 1). No input, select or textarea exists outside exempt files.

Qualification (`results/qualify.json`, `results/extra-summary.txt`). Own uses in scan: strict 90, four
definitions at 90, classes or tags 97, any tag 102, `all` 217. Every definition gives 43 div findings at the
file gate. At the every-file gate they give 92 to 94, because a looser definition exempts more own files.

- The strict definition matches: 11 custom components on huttspawn, 0 elsewhere.
- Control: 63 of huttspawn's 101 system files pass strict. 31 lack a behavior file, mostly sub-parts such as
  `card-title.astro`. The older copies on the shingle sites pass 0: Container there carries two tags and no behavior file.

Check time, ms: open 806 to 2020, first full diagnostics 71 to 956, lint after that 1 to 37 (cached), the
walk 4 to 37. The walk ran across all element rows, so one rule would cost no more.

## Predictions, held or not

| # | Outcome |
|---|---|
| P1 | **Held.** 11 on huttspawn, 0 elsewhere. |
| P2 | **Held.** 0 on shingle, 63 of 101 on huttspawn. |
| P3 | **Mostly held.** Files-only adds 0. Classes-only adds `hs-panel`. Dropping files adds 2 files, but only `hs-panel` is used, so "nobody writes six tags without the files" nearly holds. |
| P4 | **Half failed.** `system` and `format` differ on huttspawn only, as predicted, but give the same div counts at the file gate. `all` reaches 17 of 38 div files, not 90%. Layouts sit on pages, and pages hold few divs. |
| P5 | **Failed, lower.** 9% strict, 21% lenient; predicted 15 to 25%. |
| P6 | **Failed.** Layout 20%/40%, against 40 to 50%. Grid had no findings at all. Shell 33% on n=3, not 90%. Flex row 0%, under 20% as predicted. Flex-col 43% strict, 86% lenient, either side of 60%. |
| P7 | **Held.** 0%. |
| P8 | **Held on number, failed on reason.** Under 12%, but no classless wrapper of one child exists. Classless divs are terminal lines and script hooks, so deleting them is not the answer either. |
| P9 | **Failed.** Parent cuts 84% and ancestor 79%, not 60-70% and 20-30%. Precision rises more than predicted (29%, 33%) but on n=7 and 9. Most divs sit below components, not above them. |
| P10 | **Failed.** Sectioning with layout classes is n=2, both K. Styled sections are 0 of 16: nearly all paint full-bleed around a sized Container, which is the right pattern. Bare `section` is mostly a weak swap, and 3 that wrap only a Container truly merge. |
| P11 | **Half held.** `a` (131) and `p` (29) dominate. Precision far below every guess: button 2 of 7, headings 1 of 30, p 2 of 29, and no input to measure. No site has a Link. Huttspawn's bare text sits in `Container as="article"`, which sets it. |
| P12 | **Half held.** Walk under 100 ms. Check time is 1 to 3 s per site, not 5 to 60 s. |
| P13 | **Failed.** A multi-line-aware `<div` scan matches the program exactly in non-exempt files. A per-line class grep is 22 against the program's 23, missing the multi-line tag at `HeroTopology.astro:21`. The bigger gap is MDX, below. |
| P14 | **Held on what ran, weakly.** All 11 diagnostics on `.astro` and `.tsx` match on 5 sites, with nothing extra either way. All are hints: those sites have 0 errors. The one site with errors, huttspawn, could not be compared: `astro check` refuses its TypeScript 7.0. |
| P15 | **Held.** No type error or retired fill lands on any own component, so `scan` changes no existing-rule finding on this corpus. The only type errors on a use are 2 on system components. `results/precision-summary.txt`. |

## Hand-read samples: the misleading ones

Every verdict and reason is in `verdicts.mjs`. Of the 94 divs:

- **Flex rows, 13 K.** `MrsBlock.astro:101`, `SiteHeader.astro:19`, `stories/[...slug].astro:31`. No Flex
  component exists, Container stacks, Grid is columns. The largest single reason the help text is wrong.
- **Text in a div, 25 O.** `NarrativeSections.astro:26` and five siblings, `LegionHero.astro:23`. Typography.
- **Placeholder panels, 9 O.** `conversation-view.tsx:2` and 8 more. Card, and they are stubs.
- **Card chrome, 5 O.** `ArchitectureDiagram.astro:6`, `InstallCard.astro:27`.
- **Custom-CSS scene, 6 K, with canvas or SVG hosts, 3 K.** `LegionHero.astro:8`, `ArchitectureCanvas.astro:6`, `HeroTopology.astro:34`.
- **Script hooks, 2 K.** `Topbar.astro:43`, `LegionHero.astro:56`.
- **Terminal lines, 5 K.** `InstallCard.astro:51`.
- **HTML that requires a div, 1 K.** `MrsBlock.astro:139`, a dt and dd group in a dl.
- **Prose wrapper, 1 W.** `stories/[...slug].astro:48`.

The 4 true replacements: `OssCallout.astro:19`, `SectionHead.astro:14`, `stories/index.astro:20`
(`flex flex-col gap-*`: Container gap) and `VoiceHero.astro:37` (`w-full max-w-xl`: Container size).

## Recommendation

**#159: do not ship "every div" at warning.** At 43 findings, 9% strict, its help is wrong for at least 79% of
them. That teaches people to ignore the check. Telling layout divs from others is out of scope in the issue;
I would bring it into scope, because it is the difference between 21% and 88% lenient.

Ship, keeping #159's file gate and exemptions:

| Div carries | Severity | Help | Evidence |
|---|---|---|---|
| `grid`, `inline-grid`, `grid-cols-*`, `container`, `mx-auto` | warning | Grid, or Container size | Inferred: 0 findings here, so no measured noise and no measured value. These are the utilities the two components exist to own. |
| unprefixed `flex flex-col gap-*`, unprefixed `max-w-*` except `max-w-none` and `max-w-prose` | hint | Container gap, or Container size | Measured: n=8, R 4, W 3, K 1. Nearly half the usable ones need overrides, so a hint. |

The filters are `recommendedWarn` and `recommendedHint` in `lib.mjs`. I wrote them after reading this corpus,
so 50% and 88% are in-sample, and a fresh project will likely score lower. The one K is `MrsBlock.astro:139`.

The help should name the one component that fits, not "Container or Grid". Why the file gate: against every
file, for this rule, it costs 1 of 8 usable findings (`HeroInstallCTA.astro:17`) and removes the one that
points at the wrong component (`HeroTopology.astro:21`, Card chrome), taking lenient from 80% to 88%. That is
two findings, so a weak preference, and I keep it mainly because #159 already says it. Its known weakness: a
wholly freehand file escapes, including 21 divs in `LegionHero.astro`. No true replacement is lost here.

**The brief's bar of at least 20 hand-read findings per promising combination is not met for what I recommend.**
The warning tier has 0 findings and the hint tier 8. Those are whole populations, so this corpus cannot supply
more. Treat the hint tier as a direction with a small n, and the warning tier as untested.

I would expect Sean to weigh in on one thing: he asked for warns, and the measured half is a hint. Warning is
defensible at 88% lenient. Hint is what 50% strict on 8 findings supports.

**#158: ship `scan` as written, strict definition, `format` default.** Loosening buys nothing: at most 1 or 2
findings and 12 uses. Three things for its builder:

- `all`, read literally, may count local bindings as components declared in source: `<Tag>`, `<Content>`,
  `<Title>`, 15 uses across 3 sites. The toy excluded them. The issue should say.
- `hs-panel` is the one near miss: six tags and a classes file, no behavior file, 7 uses. The rule is clear
  and correct as written, and Sean should know its first real effect is to leave this out.
- Basename matching means sub-parts never qualify on their own. It does not bite custom components here, but
  31 of 101 system files would fail the same test.

**Drop:** visual-utility divs (0%), classless and wrapper divs (0% to 11%), flex rows (0%), `parent` and
`ancestor` gates (they lose 1 to 2 of 4 true replacements on n under 10, for a rule whose walk costs nothing
either way), sectioning elements at warning (0 of 16 styled, and the flagged shape is the right pattern),
raw equivalents in this rule (0 of 98 at the file gate).

**Worth a separate issue, not this rule:** a div directly holding text pointing at Typography. 34 findings
at every file, 22 Typography, 9 Card, 3 K. It finds more real work than the layout rule on this corpus,
though 14 of them are in one custom hero and 9 are stubs. One reader's judgement, and nobody has asked for it.

## Surprises

1. **The check cannot see MDX, and that is where freehand layout still is.** A text scan finds 66 divs in
   `.mdx`, 49 live and 17 in code fences. The only grid-class div in the whole corpus is there:
   `flowchart-hell-to-conversation-heaven.mdx:66`. So is smugglr's `compare/` markup, the same eyebrow divs
   the program flags in `NarrativeSections.astro`. MDX is not in the program, so #159 will report none of it.
2. **The Grid half of the help text had nothing to say.** The only grid class on any raw element outside
   exempt files is one `dl`. Migration has already done that work.
3. **#159's gate rewards the wrong files.** Adopt one component and every div in the file warns. Adopt none
   and nothing does: 51 of 94 divs sit in files that escape.
4. **`system` and `format` give identical totals in every gate, element and class combination in `combos.csv`.** `format`'s effect here
   is on uses: 90 more are checked, with 0 errors today.
5. **Twelve `div`s sit inside `button` in `LegionHero.astro`**, which is invalid HTML. No rule asked covers it.

## Not tested

- Any project not yet migrated. Grid-class findings, and the warning tier, are inference.
- `scan` catching a real error on a custom component: none exists in the corpus. #158's scenarios cover it.
- `astro check` against errors: huttspawn refused it, and the others have none.
- `rafters-check`, the bin itself, and the language server's worker. Only `openProject` and `lint` ran. No rule
  was built into veneer, so these rules are queries, and their check time is inferred from the walk.
- Rendered output: whether a swap looks the same was judged from code, never from a page.
- Second-reader agreement on any verdict.

## Rerun

Copy each site without `dist`, `.dev.vars`, secrets or `node_modules`, then:

```sh
pnpm install --frozen-lockfile --filter @rafters/veneer   # in this repo, never in a copy
node prep.mjs <copies> <site>=<original site root> ...
sh run-facts.sh <copies>
node baseline.mjs <copies>
node combos.mjs && node precision.mjs
node extra.mjs <copies>
```

Run on Node 26.13. The `results` here were made with the older `prep.mjs` and `baseline.mjs` described above.
Do not rerun `facts.ts` or `baseline.mjs` in the copies from this run: their `.bin` links into the originals
remain, because they are kept until Sean decides the repair. Make fresh copies first.
