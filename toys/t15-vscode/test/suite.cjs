// Runs inside VS Code: opens a new page, waits for diagnostics, removes the misuse, records it all.
const vscode = require("vscode");
const { writeFileSync } = require("node:fs");
const until = async (holds, ms) => {
  const t0 = Date.now();
  while (Date.now() - t0 < ms) {
    if (holds()) return Date.now() - t0;
    await new Promise((r) => setTimeout(r, 50));
  }
  return null;
};
exports.run = async () => {
  const root = vscode.workspace.workspaceFolders[0].uri;
  const uri = vscode.Uri.joinPath(root, "src/pages/t15.astro");
  const misuse = `---\nimport Button from '@/components/ui/button.astro';\n---\n<Button tone="x">a</Button>\n`;
  await vscode.workspace.fs.writeFile(uri, Buffer.from(misuse));
  const out = { astroInstalled: !!vscode.extensions.getExtension("astro-build.astro-vscode") };
  const doc = await vscode.workspace.openTextDocument(uri);
  const editor = await vscode.window.showTextDocument(doc);
  out.languageId = doc.languageId;
  const all = () => vscode.languages.getDiagnostics(uri);
  const veneer = () => all().filter((d) => d.source === "veneer");
  out.veneerMs = await until(() => veneer().length > 0, 15000);
  out.othersMs = await until(() => all().some((d) => d.source !== "veneer"), out.astroInstalled ? 30000 : 3000);
  const show = (d) => ({ source: d.source, code: typeof d.code === "object" ? d.code.value : d.code, line: d.range.start.line + 1, column: d.range.start.character + 1, message: d.message.slice(0, 90) });
  out.diagnostics = all().map(show);
  const clean = `---\nimport Button from '@/components/ui/button.astro';\n---\n<Button>a</Button>\n`;
  await editor.edit((e) => e.replace(new vscode.Range(0, 0, doc.lineCount, 0), clean));
  out.clearedMs = await until(() => veneer().length === 0, 5000);
  writeFileSync(process.env.T15_OUT, JSON.stringify(out, null, 1));
  await vscode.workspace.fs.delete(uri);
};
