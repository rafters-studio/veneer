import { mkdir, rename, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import { readConfig, type Notice, type VeneerError } from "./config.ts";
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
    // exclusively stops the write following that one. A symlinked folder above it is #131.
    await rm(temporary, { force: true });
    await writeFile(temporary, text, { flag: "wx" });
    await rename(temporary, docsPath);
  } catch (error) {
    await rm(temporary, { force: true });
    throw error;
  }
}

async function open(root: string): Promise<Project | VeneerError> {
  try {
    return await openProject(root);
  } catch (error) {
    if (error instanceof ProjectError && error.code === "tsconfig-missing") {
      return { code: error.code, message: error.message };
    }
    throw error;
  }
}

export async function writeDocs(projectRoot: string): Promise<DocsResult> {
  const root = path.resolve(projectRoot);
  const config = await readConfig(root);
  if (!config.ok) return { ok: false, errors: config.errors };

  const project = await open(root);
  if ("code" in project) return { ok: false, errors: [project] };

  try {
    const { components, notices } = await resolveComponents(project, config.componentFolders);
    const docsPath = path.join(root, ".rafters/veneer/docs.jsonl");
    await writeWhole(docsPath, docsText(config.intent, components));
    return {
      ok: true,
      docsPath,
      notices: [...config.notices, ...notices].sort(
        (a, b) => compareCodePoints(a.code, b.code) || compareCodePoints(a.message, b.message),
      ),
    };
  } finally {
    project.close();
  }
}
