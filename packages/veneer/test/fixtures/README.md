# Test fixtures

Each folder is a whole project; a scenario copies one and installs it.

- `read`, `no-astro`: FR-VEN-064.
- `docs`, with `docs.expected.jsonl`: FR-VEN-065 and FR-VEN-074.
- `lint`, with `lint.expected.jsonl`: FR-VEN-066. The golden comes from toys T11 and T12, with `help` written by hand from each component's declared props, never from veneer's own output.
- `fill-tokens`: issue #122. Not a project: a token folder its scenarios copy into `lint`'s `.rafters/tokens`.
- `untrusted`: FR-VEN-064. It ships a fake `@astrojs/svelte` whose `dist/editor.cjs` writes a marker file, to prove the read never loads a project's code.
