# t164: predictions, written before any probe in this folder ran

Results are at the end. Run from the repo root after `pnpm -r build`; `<scratch>` holds installed copies of the `read` and `lint` fixtures as `read/` and `lint/`.

    node toys/t164-second-check/probe.mjs <root> <file>
    node toys/t164-second-check/matrix.mjs <scratch> none passthrough unsaved-only generation dispose content content-reread
    node toys/t164-second-check/worker-run.mjs <scratch>/read none content
    node toys/t164-second-check/lsp-run.mjs <abs path to target/debug/veneer-lsp> <scratch>/read
    MODE=content node --expose-gc --import ./toys/t164-second-check/modes.mjs toys/t164-second-check/memory.mjs <scratch>/read 12
    node toys/t164-second-check/concurrent.mjs <scratch>/read

Already observed before writing these: the three t17 cases reproduce at 1d3d572 (= origin/main). Already read: volar-service-typescript semantic.js:16-24, @volar/typescript createProject.js:12,238-266, typescript.js:144686.

## Hypothesis

The surviving state is the TypeScript DocumentRegistry held in the module-level `documentRegistries` array (semantic.js:16). Each `check()` builds a new language service with a new host, whose per-file version counter restarts at "0" (createProject.js:12,241,247). The registry reuses an entry when `entry.sourceFile.version === version` (typescript.js:144686), without comparing text. So every file keeps the SourceFile of the first text seen in the process, whatever snapshot the new host offers.

## P1, direct state read (probe.mjs, shingle FeatureBlock, existing then appended, in openProject + lint as check does)

- Open 2: `program.getSourceFile(file).version === "0"`, same as open 1.
- Open 2: the program's SourceFile text does NOT contain `p-[3px]`.
- Open 2: Volar's source snapshot AND its generated snapshot DO contain `p-[3px]`. So kit's fsFileSnapshots and the overlay are right and only the program is stale.
- Open 2: the SourceFile is the same object as in open 1 (`===`).

## P2, interventions (matrix.mjs). Control first: a wrapper that leaves the version untouched

| mode | what it does | predicted second pass of unchanged-then-append |
| none | nothing | 0 problems |
| passthrough | wraps getScriptVersion, returns the same value | 0 problems |
| generation | prefixes each version with an open counter | 1 problem at 22:13 |
| content | version is a hash of the snapshot text | 1 problem at 22:13 |
| dispose | disposes the previous language service before the next open | 1 problem at 22:13 |

Predicted second-pass time: content well under generation and dispose, since unchanged files keep their ASTs.

## P3, shapes of the bug with no intervention

- Reverse direction, a false finding: append, then `existing` for the same file: second pass STILL reports the bracket (fresh: 0 problems).
- Third call: existing, appended, appended2: both later passes 0 problems (first text wins for good).
- A file first seen on a later call: a new path that was not on disk in the first call: correct, 1 problem (no registry entry).
- Different file: FeatureBlock then whole-file Section.astro replace: 0 problems (fresh: 1).
- Disk change between two calls with no unsaved at all: stale too.
- The same on the small lint fixture, so the issue's scenario can fail there: second pass misses it.

## P4, worker (run against dist/worker.mjs over its JSON lines, on a fixture copy)

- Plain typing in a root file, same open: clean, bracket, clean, bracket: correct every line. Versions advance inside one host.
- Staleness needs a reopen (worker.ts:72 or :99) AND a file whose entry is at "0" whose text differs at "0" in the new host.
- Predicted failing sequence: type a brand-new file N with a bracket-free text (reopen, N at "0"), close it (no disk text, so reopen, worker.ts:99), open N again with a bracket (reopen, N at "0" again): the answer is empty. A fresh process reports 1 problem.
- Predicted failing sequence for a disk change: a file C changes on disk, a new file triggers a reopen, then a lint of a file that imports a const from C uses C's old text.

### Results that broke a prediction, and the amended predictions written before re-running

- P3 reverse direction, E+B then E: predicted a false finding, observed 0 problems. Amended: the stale program holds the bracket, but `positionOf` maps it through Volar's fresh, shorter mapping and `lint.ts:248` drops it. A later text of the same shape keeps the mapping, so it reports a phantom. (Observed afterwards: yes, `read: phantom`.)
- W4, a discarded unsaved bracket: predicted a phantom in the importer, observed correct. Amended: `lint` on a `.ts` buffer filters the file out at `lint.ts:376`, so no diagnostics ran, no program was built, and the registry entry stayed at "0" with the disk text. So:
  - W4b, the same with a lint of the importer before the first reopen: predicted a phantom in `uses.astro`, fresh process none.
  - W5, W4's first two steps with the buffer still open: predicted the unsaved bracket is ignored in `uses.astro` after the reopen, fresh worker reports it.

## P5, #134 (memory.mjs, 20 opens, gc between, fixture "read" or lint)

- none: about 5 MB per open, as #134 measured.
- dispose: much worse, about 100 MB per open on the same fixture as #134 said (re-parse, old checker still held).
- content: about the same as none, not worse. So the same registry is why ASTs are shared, but it is not what retains the checker: #134 and #164 touch the same structure and have different causes. Fixing #164 by content version will not fix #134.

## P6, after the content intervention, does any later call still differ from a fresh process?

Differential: each case's later call against a fresh process running only that call: same file different text, different file, new file, disk change of an imported file, tsconfig `include` change between calls, a call with a different root. Predicted: all equal, so criterion 2 has nothing left to catch from this cause. Separately, predicted: two concurrent `check()` reject with ProjectError already-open (docs.ts:71 rethrows) rather than returning `ok: false`.

## Results

Run at 1d3d572. The reproduction alone was rerun at 15136d7 (main moved during the work, report formatting only): same answers.

- P1 held exactly: open 2 has version "0", the same SourceFile object, no mark in the program, the mark in both Volar snapshots.
- P2 held. Later-call differences from a fresh process, of 20 cases: none and passthrough 14; unsaved-only 5; generation and dispose 0 of the 19 they ran (the same-mtime case was added after); content 1 (same mtime); content-reread 0.
- P3: the reverse prediction was wrong as written, see above. The rest held.
- P4 held for W1, W2, W2b, W3, W3b, W4b, W4c. W4 was wrong as written. W5c: correct, as a .tsx lint rebuilds before the reopen. W2 and W3b also reproduced through target/debug/veneer-lsp.
- P5 held. MB per open, second half, read fixture: none 5.74, content 5.76, content-reread 10.08, dispose and generation 85.76 (6 opens; 12 opens ends in a heap out-of-memory). Later-call median: none and content 71 ms, content-reread 97, dispose 222, generation 242.
- P6: one difference left under content, a disk edit keeping the same mtime, from kit's fsFileSnapshots. The concurrent prediction was half wrong: two plain calls both fulfil, since the first finishes before the second's config read returns; a save followed by a plain call rejects with already-open.
- `pnpm bdd` with content patched in through NODE_OPTIONS: 163 scenarios passed. No unpatched baseline was run.
