import { defineConfig } from "vite-plus";

export default defineConfig({
  // The fixtures' bytes are the requirement's: scenarios assert columns inside them.
  fmt: { ignorePatterns: ["test/fixtures/**"] },
  pack: {
    entry: { index: "src/index.ts", worker: "src/worker.ts" },
    dts: true,
    exports: false,
  },
});
