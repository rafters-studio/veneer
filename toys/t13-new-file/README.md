# T13: checking a page that does not exist yet

The pre-edit hook sends text for a new page. `PREDICTIONS.md` was written before it ran.

```sh
node run.mjs <dist/index.mjs> <project> src/pages/brand-new.astro src/components/ui
```

**After the program opens, it cannot be done.** `checker.fileCreated` re-parses the tsconfig, but `getScriptFileNames` caches names in a `WeakMap` keyed on `commandLine`, and the kit mutates that same object, so the new list is never seen. Predictions 1 to 3 were wrong as written.

**Before it opens, it works.** With `ts.sys` answering for the path before `openProject`, including `readDirectory`, the file is a root from the start. On rafters.studio: 90 roots in place of 89, ts(2322) at 4:9 with help `variant, size, loading`, nothing on disk, opened in 709 ms.

`readDirectory` must add the path only when the tsconfig's `include` takes it. `ts.matchFiles` with a file system holding just that path decides this: a page under `src` matched, and `elsewhere/not-included.astro` did not and stayed out. `ts.matchFiles` is internal to TypeScript, which FR-VEN-064 pins exactly at 6.0.3.

A long-running process, such as the language server, cannot add a root this way after it has opened. That belongs to FR-VEN-072.
