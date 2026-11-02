# T14: a Rust language server in front of a warm program

`PREDICTIONS.md` was written before it ran. A Rust binary speaks LSP on stdio and starts one Node worker holding an open Project, with JSON lines between them.

```sh
cargo build --release
node client.mjs $PWD/target/release/t14-lsp <dist/index.mjs> <sites/rafters.studio> src/components/ui src/pages/kelex/index.astro
```

Results on rafters.studio, Apple M-series, Node 26.13, in `out-run.json`:

| | Measured | Predicted |
|---|---|---|
| Keystroke to publishDiagnostics, warm, median of 20 | 49.8 ms, max 64.4 | under 150 ms |
| The Rust layer | 0.14 ms | under 5 ms |
| First publish from starting the server | 1518 ms | under 3 s |
| A page not in the program: close and reopen with it unsaved | 227 ms, and 38 ms on its next change | under 2 s |
| A burst of 10 changes | 1 publish, the newest text | no stale result |
| SIGKILL to the server | the worker is gone, its stdin closed | it ends |
| Release binary | 1.2 MB | under 5 MB |

All seven held. Reopening took 227 ms and not a second because the modules are already loaded and the type checker's source files are shared.

The newest-wins rule: one request in flight per server, and a text arriving meanwhile replaces any waiting text, so a burst costs one or two checks and cannot publish stale results.

## Limits

- A change to one file re-lints only that file. Other open files that depend on it are not refreshed until a save.
- Memory over a long session was not measured, and each reopen leaves about 5 MB behind (#134).
- It was driven by a script, not an editor. T4 showed VS Code shows a server's diagnostics beside Astro's; that has not been re-run with this binary.
- Node comes from `PATH`.
