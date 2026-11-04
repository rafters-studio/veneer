// node probe.mjs <root> <file>: P1. Opens twice as check() does; prints what each store holds for the file.
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";
import { lint, openProject } from "@rafters/veneer";

const require = createRequire(
  fileURLToPath(new URL("../../packages/veneer/package.json", import.meta.url)),
);
const server = createRequire(require.resolve("@astrojs/language-server/package.json"));
const { URI } = server("vscode-uri");

const [root, file] = process.argv.slice(2);
const absolute = `${root}/${file}`;
const existing = readFileSync(absolute, "utf8");
const MARK = "p-[3px]";
const appended = `${existing}\n<div class="${MARK}"></div>\n`;
let earlier;
for (const [name, text] of [
  ["open 1 existing", existing],
  ["open 2 appended", appended],
]) {
  const project = await openProject(root, { unsaved: { [file]: text } });
  const found = await lint(project, { componentFolders: [], files: [file] });
  const sourceFile = project.program().getSourceFile(absolute);
  const script = project.checker.language.scripts.get(URI.file(absolute));
  const generated = script.generated.languagePlugin.typescript.getServiceScript(
    script.generated.root,
  ).code.snapshot;
  const has = (s) => s.getText(0, s.getLength()).includes(MARK);
  console.log(
    name,
    JSON.stringify({
      sourceFileVersion: sourceFile.version,
      programTextHasMark: sourceFile.text.includes(MARK),
      volarSourceHasMark: has(script.snapshot),
      volarGeneratedHasMark: has(generated),
      sameSourceFileObjectAsEarlier: earlier ? earlier === sourceFile : null,
      diagnostics: found.length,
    }),
  );
  earlier = sourceFile;
  project.close();
}
