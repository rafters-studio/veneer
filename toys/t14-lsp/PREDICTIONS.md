# T14 predictions

Written before it ran. FR-VEN-072 says a Rust language server gets type diagnostics from one warm TypeScript 6 program in Node. T5 timed the warm read, not this shape. Since then the native read went away, and T13 showed a program cannot add a root once open.

The shape under test: a Rust binary speaks LSP on stdio, and starts one Node worker that holds an open Project, speaking JSON lines. On a change it sends the buffer; the worker calls `setUnsaved` and `lint` on that file; the server publishes the result.

1. Keystroke to publishDiagnostics on rafters.studio, median over 20 changes, warm: under 150 ms. Confidence 0.7.
2. The Rust layer adds under 5 ms of that. Confidence 0.75.
3. The first publish after opening a page arrives within 3 s of starting the server. Confidence 0.8.
4. A page not in the program is handled by closing and reopening the Project with it unsaved, in under 2 s, and the next change on it is warm again. Confidence 0.6.
5. A burst of 10 quick changes publishes the last text's result, and no stale result arrives after it. Confidence 0.65.
6. Killing the server with SIGKILL ends the worker, because its stdin closes. Confidence 0.85.
7. The release binary is under 5 MB. Confidence 0.6.
