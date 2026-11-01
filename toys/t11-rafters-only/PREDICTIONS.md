# T11 predictions

Written before any of this ran. The question: AstroCheck reports every type error in a
project. Can veneer report only the ones on a use of a rafters component, reading facts
from the program and the rafters config, and inferring nothing from names?

Setup: `@astrojs/language-server` 2.17.3 as it comes, `@volar/kit` 2.4.30, TypeScript 6.0.3,
the wasm `@astrojs/astro2tsx`, no native binding, no patch.

## The rules under test

Step 1, is this diagnostic about a component use? Map its start into the program's text,
find the innermost node, and accept it only when that node is a JSX attribute's name or a
JSX element's tag name.

Step 2, whose component is it? Three candidates:

- **symbol**: resolve the tag's symbol through aliases, and through `const Tag = Button`;
  its declaration's file is under `componentsPath`.
- **signature**: `getResolvedSignature(element).declaration`'s file is under `componentsPath`.
- **installed**: the symbol rule, and the file's basename is in the config's
  `installed.components`.

## Predictions

1. The symbol rule classifies all marked cases correctly, A13 counted as rafters.
   Confidence 0.65.
2. The signature rule fails on the forwardRef component (T1, T2, T3), because its signature
   is declared in `@types/react`. Confidence 0.8.
3. A13 separates the rules: a component under `componentsPath` that rafters did not install
   is reported by symbol and not by installed. Confidence 0.85.
4. A9 (an error inside a rafters element's children) and A10 (an undefined name in an
   attribute value) are not reported, because the node is neither an attribute name nor a
   tag name. Confidence 0.8.
5. A3 (missing required prop) and A11 (bad spread) land on the tag name in the source.
   Confidence 0.6.
6. A14 (`<UI.Button>`, namespace import through a barrel) resolves to `button.astro`.
   Confidence 0.6.
7. A15 (`const Tag = Button`) is the likeliest failure: Astro's output may not keep it as a
   checked JSX element. Confidence it is reported as rafters: 0.5.
8. A16: a `@deprecated` prop gives hint ts(6385) on its name, and the symbol's JSDoc tag
   carries the text written there, so a replacement can be read from source with no list
   of veneer's own. Confidence 0.6.
9. On rafters.studio, the unsaved page with `tone`, `variant` and one plain type error
   reports the first two and not the third, and classifies none of the project's baseline
   diagnostics as rafters. Confidence 0.7.
10. Classifying a whole project adds under 20 ms to the check on rafters.studio.
    Confidence 0.7.
