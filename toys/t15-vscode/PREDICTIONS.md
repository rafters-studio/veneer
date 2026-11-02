# T15 predictions

Written before it ran. FR-VEN-072 rev 7 says the editor package sets `--worker` to "its own copy". T4's files are gone, and T4 ran a toy server, not this one. Nothing has run the built server in an editor, and no one has measured what "its own copy" costs a person installing it.

The shape under test: a small VS Code extension using vscode-languageclient, selecting `**/*.astro` by pattern, starts the built `veneer-lsp` with `--worker`. A test run launches real VS Code with `@vscode/test-electron` on the lint fixture and reads `vscode.languages.getDiagnostics`.

1. A planted `<Button tone="x">` shows a diagnostic with source `veneer` within 15 s of opening the file. Confidence 0.8.
2. With the Astro extension installed, the same file also holds a diagnostic from another source, so the two sit side by side. Confidence 0.7.
3. Without the Astro extension, the `veneer` diagnostic still shows, because the selector is a pattern. Confidence 0.8.
4. An edit that removes the misuse clears the `veneer` diagnostic within 2 s. Confidence 0.8.
5. `@rafters/veneer` with its production dependencies installed, which is the copy the worker needs, is under 100 MB. Confidence 0.5.
6. The extension's own code with vscode-languageclient, bundled or not, is under 5 MB. Confidence 0.7.
