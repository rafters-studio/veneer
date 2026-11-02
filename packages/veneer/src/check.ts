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

export async function check(
  projectRoot: string,
  options: { save?: boolean; paths?: string[]; unsaved?: Record<string, string> } = {},
): Promise<CheckResult> {
  const root = path.resolve(projectRoot);
  const config = await readConfig(root);
  if (!config.ok) return { ok: false, errors: config.errors };

  const unsaved = Object.entries(options.unsaved ?? {});
  const isSave = options.save === true && unsaved.length === 0;
  const symlink = isSave ? outputSymlinkError(root) : undefined;
  if (symlink) return { ok: false, errors: [symlink] };

  const project = await openOrError(root);
  if ("code" in project) return { ok: false, errors: [project] };

  try {
    for (const [file, text] of unsaved) project.setUnsaved(file, text);
    const diagnostics = await lint(project, {
      componentFolders: config.componentFolders,
      files: isSave ? undefined : options.paths,
    });
    const written = isSave ? await writeDocsFrom(project, config) : undefined;
    return {
      ok: true,
      diagnostics,
      report: report(diagnostics),
      ...(written ?? { notices: sortedNotices(config.notices) }),
    };
  } finally {
    project.close();
  }
}
