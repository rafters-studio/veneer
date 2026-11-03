# t17: what one file costs, and a second check() in one process

Question from rafters: can the pre-edit hook check one file with proposed content, and what does it cost?

Run from `packages/veneer` after `pnpm build`, so `@rafters/veneer` resolves, against any project:

    node ../../toys/t17-one-file/one-case.mjs <root> <file> append
    node ../../toys/t17-one-file/one-case.mjs <root> <file> unchanged-then-append

Results, 2026-11-03, main at d4bcf8c, `shingle/sites/rafters.studio`, `src/components/FeatureBlock.astro`:

| Run | Result |
|---|---|
| one file, `append`, inside `check()` | 678 ms, 1 problem, correct |
| one file, `replace`, inside `check()` | 635 ms, 1 problem, correct |
| one file, whole node process | 1.03 s wall |
| whole project through the bin | 1.26 s wall |
| `append-twice`, second pass | 149 ms, 1 problem, correct |
| `unchanged-then-append`, second pass | 150 ms, 0 problems: WRONG, the bracket is missed |

The first pass in a process is right. A second `check()` in that process with different unsaved text can drop a finding. Do not use the second-pass times as a cost: they come from a process that is giving wrong answers.
