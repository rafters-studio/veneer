// node test/run.cjs <server> <worker.mjs> <workspace> <out.json> [--astro]
const path = require("node:path");
const { spawnSync } = require("node:child_process");
const { runTests, downloadAndUnzipVSCode, resolveCliArgsFromVSCodeExecutablePath } = require("@vscode/test-electron");
(async () => {
  const [server, worker, workspace, out, astro] = process.argv.slice(2);
  const exe = await downloadAndUnzipVSCode("1.100.0");
  const [cli, ...args] = resolveCliArgsFromVSCodeExecutablePath(exe);
  const ext = path.join(path.dirname(out), astro ? "exts-astro" : "exts-none");
  if (astro) spawnSync(cli, [...args, "--extensions-dir", ext, "--install-extension", "astro-build.astro-vscode"], { stdio: "ignore", shell: process.platform === "win32" });
  await runTests({
    vscodeExecutablePath: exe,
    extensionDevelopmentPath: path.resolve(__dirname, ".."),
    extensionTestsPath: path.join(__dirname, "suite.cjs"),
    launchArgs: [workspace, "--user-data-dir", process.env.T15_USER_DATA, "--extensions-dir", ext, "--disable-workspace-trust", "--skip-welcome", "--skip-release-notes", "--disable-gpu"],
    extensionTestsEnv: { T15_SERVER: server, T15_WORKER: worker, T15_OUT: out },
  });
})().catch((e) => { console.error(String(e)); process.exit(1); });
