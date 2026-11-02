import { fileURLToPath } from "node:url";
import { defineConfig } from "vite-plus";

const here = fileURLToPath(new URL("./", import.meta.url));
// The pinned rafters source is a code-only import: its workspace packages resolve into it, and what
// they import resolves from this package, because its own manifests are never installed.
const raftersSource = (name: string): string =>
  `${here}node_modules/rafters-monorepo/packages/${name}/src/index.ts`;

export default defineConfig({
  // The fixtures' bytes are the requirement's: scenarios assert columns inside them.
  fmt: { ignorePatterns: ["test/fixtures/**"] },
  pack: {
    entry: { index: "src/index.ts", worker: "src/worker.ts" },
    dts: true,
    exports: false,
    alias: {
      "@rafters/shared": raftersSource("shared"),
      "@rafters/color-utils": raftersSource("color-utils"),
      "@rafters/math-utils": raftersSource("math-utils"),
    },
    inputOptions: { resolve: { modules: ["node_modules", `${here}node_modules`] } },
  },
});
