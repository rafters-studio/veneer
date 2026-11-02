# NFR-VEN-001 and NFR-VEN-002

shingle `sites/rafters.studio` at a5ae5dee8e436c2006b2533a2afa99cab2ccc0c8, from the GitHub tarball, installed with `--ignore-scripts`. Built `dist`, Apple machine, Node 26.13.0. Each run is a fresh process, timed from process start. 75 `.astro` and `.tsx` root files; `docs.jsonl` has 43 lines.

| Event | Call | Result | Bound |
|---|---|---|---|
| NFR-VEN-001-c1 | `src/pages/docs/index.astro` as unsaved text, first run | 1115 ms (1136 ms wall from the parent) | 3 s |
| NFR-VEN-002-c1 | save, 20 repeat runs | slowest 1223 ms (1244 ms wall); range 1147 to 1223 ms | 3 s |

The first-run figure is not a cold disk: the copy had just been installed, so the OS file cache was warm. The requirement counts the OS cache as warm only for repeat runs, so a truly cold disk will be slower.
