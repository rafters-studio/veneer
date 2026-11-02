# T13 predictions

Written before it ran. The pre-edit hook sends `check` text for a page that does not exist yet. Can the program take it?

1. `checker.fileCreated(path)`, with `ts.sys` answering for the path, adds it to `getRootFileNames()`. Confidence 0.6.
2. It then gives the same diagnostics as if the file existed on disk: a bad rafters prop at its exact position. Confidence 0.6.
3. Removing it with `fileDeleted` returns the root list to what it was, and nothing was written to disk. Confidence 0.6.
4. It adds under 50 ms on rafters.studio. Confidence 0.6.
5. A new file the tsconfig's `include` would not match can be told apart before adding it, using TypeScript's own parse. Confidence 0.5.
