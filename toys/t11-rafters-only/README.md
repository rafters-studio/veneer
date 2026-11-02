# T11: report only the rafters components

AstroCheck reports every type error in a project. T11 asks whether veneer can report only
the ones on a use of a rafters component, from facts in the program and the rafters config,
inferring nothing from names. `PREDICTIONS.md` was written before anything ran.

## Rerun

```sh
pnpm install --ignore-workspace
(cd fixture && pnpm install --ignore-workspace)
node run.ts fixture > out-fixture.json
```

The real project, shingle `sites/rafters.studio` at a5ae5de, with unsaved text so nothing on
disk changes (`plant/kelex-index.astro` is that page plus three planted errors):

```sh
node run.ts <shingle>/sites/rafters.studio --plant src/pages/kelex/index.astro plant/kelex-index.astro > out-studio.json
```

Run on an Apple M-series machine, Node 26.13, TypeScript 6.0.3, `@astrojs/language-server`
2.17.3 as it comes, with its wasm `astro2tsx` and no native binding.

## The rule that works

A diagnostic is a rafters report when both hold:

1. Mapped into the program, its start is on a JSX attribute's name or a JSX element's tag name.
2. The tag's symbol, followed through aliases and `const Tag = Button`, is declared in a file
   under a `componentsPath` folder.

## Results

| Rule | Fixture (24 cases) | rafters.studio (3 cases) |
|---|---|---|
| symbol, under `componentsPath` | 23 | 3 |
| signature, under `componentsPath` | 20 | 3 |
| symbol, and listed in `installed.components` | 22 | 1 |

The one miss in the first row, A11, is not a classification error. See below.

- **Right by symbol:** undeclared prop, bad value and missing required prop, in `.astro` and
  on a forwardRef `.tsx` component; renamed import; barrel; namespace import; `const Tag =
  Button`; nested inside the project's own component.
- **Correctly left out:** the project's own `Button`, a third-party `Button`, plain elements,
  plain type errors, an error in a rafters element's children, an undefined name in an
  attribute value, an error inside a rafters component's own file.
- **Signature fails on forwardRef**, because the signature is declared in `@types/react`.
- **`installed.components` cannot be the source.** rafters.studio lists 6 names and has 42
  `.astro` components under `componentsPath`, and `button` is not listed.
- **Classifying costs almost nothing:** 1.6 to 2.0 ms on rafters.studio's diagnostics.

## What it showed that nobody asked

- **A spread is not checked.** `<Button {...wrong}>` with a property Button does not declare
  gives no diagnostic at all (A11). TypeScript does not apply its excess-prop check to JSX
  spreads, so "a component's type is its rule" cannot catch it.
- **A retired prop can name its replacement from source.** A `@deprecated` prop gives hint
  ts(6385) on its name. `getProperty` on the type of the resolved signature's first parameter
  returns the JSDoc tag's text, `Use variant="ghost".`, with no list of veneer's own.
  `getSymbolAtLocation` on the attribute returns no tags. This covers retired props, not
  retired values: a member of a string union carries no JSDoc tag, so fill's retired values
  (veneer#122) are still unsolved.
- **The whole-project check is near the 3 s target.** Construct and check took 2.26 to 3.01 s
  across four runs on rafters.studio, before about 0.4 s of module load (T8's figure), so a
  save may exceed NFR-VEN-002's 3 s. T8 measured 1.05 to 2.10 s for the same step.

## Predictions against results

| # | Prediction | Result |
|---|---|---|
| 1 | Symbol rule gets every case | Missed only A11, which has no diagnostic |
| 2 | Signature fails on forwardRef | Held |
| 3 | A13 separates folder from installed | Held |
| 4 | A9 and A10 not reported | Held |
| 5 | A3 and A11 land on the tag name | A3 held, A11 wrong: no diagnostic |
| 6 | A14 resolves to button.astro | Held |
| 7 | A15 is the likeliest failure, 0.5 | Reported correctly |
| 8 | `@deprecated` text readable | Held, by the parameter route only |
| 9 | rafters.studio reports the planted two, not the plain error or baseline | Held |
| 10 | Classifying under 20 ms | Held: about 2 ms |

## Not tested

Components reached through `primitivesPath` or an external `componentsPath` entry, `.rafter`
composites, Astro compile errors, and which diagnostics are reported from a component's own
file while rafters itself is being developed.
