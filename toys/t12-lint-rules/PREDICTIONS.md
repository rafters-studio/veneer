# T12 predictions

Written before anything ran. Two unknowns stand between FR-VEN-066 and a build.

## A. Can veneer read a project without running its code?

A project ships a fake `@astrojs/svelte` whose editor entry writes a marker file when it is loaded.

1. With AstroCheck as it comes, opening the project with a `.svelte` root file runs that code and the
   marker appears. Confidence 0.75.
2. With `setIsTrusted(false)` from the language server's `importPackage.js`, called before construction,
   no marker appears. Confidence 0.8.
3. Untrusted, the diagnostics on rafters pages match the trusted run exactly. Confidence 0.75.
4. Untrusted, a page importing `Foo.svelte` gets at most one new diagnostic. Confidence 0.5.

## B. Can the check find bracket values the way `classy` did?

Walk the program for class attributes. Flag a class whose utility part has brackets; brackets in a
modifier, such as `data-[state=open]:`, do not count. Skip files under `componentsPath`.

5. Every static case in the fixture is classified right, in `.astro` and `.tsx`. Confidence 0.7.
6. Positions in `.astro` map to the class's exact source line and column. Confidence 0.6.
7. `class:list` literals are reachable in `.astro`'s generated TSX. Confidence 0.6.
8. Dynamic values produce nothing and no crash: this rule cannot see them. Confidence 0.9.
9. It adds under 20 ms on rafters.studio, and flags nothing outside bracket utilities. Confidence 0.7.
