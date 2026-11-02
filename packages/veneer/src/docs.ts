import { lstatSync } from "node:fs";
import { mkdir, rename, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import { readConfig, type DocsConfig, type Notice, type VeneerError } from "./config.ts";
import { openProject, ProjectError, type Project } from "./read.ts";
import { compareCodePoints, resolveComponents, type Component, type Prop } from "./resolve.ts";

export type DocsResult =
  | { ok: true; docsPath: string; notices: Notice[] }
  | { ok: false; errors: VeneerError[] };

function propLine(prop: Prop): object {
  return {
    name: prop.name,
    type: prop.type,
    optional: prop.optional,
    ...(prop.default !== undefined && { default: prop.default }),
    ...(prop.values !== undefined && { values: prop.values }),
    ...(prop.doc !== undefined && { doc: prop.doc }),
    file: prop.file,
    line: prop.line,
    column: prop.column,
  };
}

function componentLine(component: Component): object {
  return {
    kind: "component",
    name: component.name,
    file: component.file,
    resolved: component.resolved,
    ...(component.reason !== undefined && { reason: component.reason }),
    ...(component.description !== undefined && { description: component.description }),
    tags: component.tags,
    props: component.props.map(propLine),
    inherited: component.inherited,
    inheritedDefaults: component.inheritedDefaults,
  };
}

function docsText(intent: string | null, components: Component[]): string {
  const lines = [{ kind: "project", intent }, ...components.map(componentLine)];
  return lines.map((line) => `${JSON.stringify(line)}\n`).join("");
}

async function writeWhole(docsPath: string, text: string): Promise<void> {
  const temporary = `${docsPath}.tmp`;
  await mkdir(path.dirname(docsPath), { recursive: true });
  try {
    // A repo could commit the temporary path as a symlink. Removing it and creating the file
    // exclusively stops the write following that one.
    await rm(temporary, { force: true });
    await writeFile(temporary, text, { flag: "wx" });
    await rename(temporary, docsPath);
  } catch (error) {
    await rm(temporary, { force: true });
    throw error;
  }
}

export async function openOrError(
  root: string,
  unsaved?: Record<string, string>,
): Promise<Project | VeneerError> {
  try {
    return await openProject(root, { unsaved });
  } catch (error) {
    if (error instanceof ProjectError && error.code === "tsconfig-missing") {
      return { code: error.code, message: error.message };
    }
    throw error;
  }
}

export function outputSymlinkError(root: string): VeneerError | undefined {
  const symlink = [".rafters", ".rafters/veneer"].find((folder) =>
    lstatSync(path.join(root, folder), { throwIfNoEntry: false })?.isSymbolicLink(),
  );
  if (!symlink) return undefined;
  const message = `${symlink} is a symbolic link; veneer writes only inside the project`;
  return { code: "output-symlink", message };
}

export function sortedNotices(notices: Notice[]): Notice[] {
  return notices.toSorted(
    (a, b) => compareCodePoints(a.code, b.code) || compareCodePoints(a.message, b.message),
  );
}

export async function writeDocsFrom(
  project: Project,
  config: DocsConfig,
): Promise<{ docsPath: string; notices: Notice[] }> {
  const { components, notices } = await resolveComponents(project, config.componentFolders);
  const docsPath = path.join(project.root, ".rafters/veneer/docs.jsonl");
  await writeWhole(docsPath, docsText(config.intent, components));
  return { docsPath, notices: sortedNotices([...config.notices, ...notices]) };
}

export async function writeDocs(projectRoot: string): Promise<DocsResult> {
  const root = path.resolve(projectRoot);
  const config = await readConfig(root);
  if (!config.ok) return { ok: false, errors: config.errors };

  const symlink = outputSymlinkError(root);
  if (symlink) return { ok: false, errors: [symlink] };

  const project = await openOrError(root);
  if ("code" in project) return { ok: false, errors: [project] };

  try {
    return { ok: true, ...(await writeDocsFrom(project, config)) };
  } finally {
    project.close();
  }
}
