import path from "node:path";
import { loadRegistryFromDir } from "rafters-monorepo/packages/design-tokens/src/persistence.ts";
import { REGISTRY_PLUGINS } from "rafters-monorepo/packages/design-tokens/src/plugins/index.ts";

const SURFACE_PREFIX = "surface-";

// The words the project's tokens hold as surfaces. A surface-* token outside the fill namespace is not one.
export function surfaceWords(root: string): ReadonlySet<string> {
  const registry = loadRegistryFromDir(path.join(root, ".rafters/tokens"), REGISTRY_PLUGINS);
  return new Set(
    registry
      .list({ namespace: "fill" })
      .filter(({ name }) => name.startsWith(SURFACE_PREFIX))
      .map(({ name }) => name.slice(SURFACE_PREFIX.length)),
  );
}
