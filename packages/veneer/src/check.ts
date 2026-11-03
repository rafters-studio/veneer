import path from "node:path";
import { readConfig, type Notice, type VeneerError } from "./config.ts";
import { openOrError, outputSymlinkError, sortedNotices, writeDocsFrom } from "./docs.ts";
import { lint, report, type LintDiagnostic } from "./lint.ts";

export type CheckResult =
  | {
      ok: true;
      diagnostics: LintDiagnostic[];
      report: string;
      notices: Notice[];
      docsPath?: string;
    }
  | { ok: false; errors: VeneerError[] };

function outsideProgram(file: string): Notice {
  return {
    code: "outside-program",
    message: `${file} is not in the tsconfig's files, so it was not checked`,
  };
}

export async function check(
  projectRoot: string,
  options: { save?: boolean; paths?: string[]; unsaved?: Record<string, string> } = {},
): Promise<CheckResult> {
  const root = path.resolve(projectRoot);
  const config = await readConfig(root);
  if (!config.ok) return { ok: false, errors: config.errors };

  const unsaved = options.unsaved ?? {};
  const unsavedFiles = Object.keys(unsaved);
  const isSave = options.save === true && unsavedFiles.length === 0;
  const symlink = isSave ? outputSymlinkError(root) : undefined;
  if (symlink) return { ok: false, errors: [symlink] };

  const project = await openOrError(root, unsaved);
  if ("code" in project) return { ok: false, errors: [project] };

  try {
    const roots = project.checker.getRootFileNames();
    const outside = unsavedFiles.filter((file) => !roots.includes(path.join(root, file)));
    const asked = options.paths ?? (unsavedFiles.length > 0 ? unsavedFiles : undefined);
    const diagnostics = await lint(project, {
      componentFolders: config.componentFolders,
      scan: config.scan,
      files: isSave ? undefined : asked?.filter((file) => !outside.includes(file)),
    });
    const written = isSave ? await writeDocsFrom(project, config) : undefined;
    return {
      ok: true,
      diagnostics,
      report: report(diagnostics),
      ...(written ?? {
        notices: sortedNotices([...config.notices, ...outside.map(outsideProgram)]),
      }),
    };
  } finally {
    project.close();
  }
}
