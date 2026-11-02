import { defineConfig } from "vite-plus";

export default defineConfig({
  pack: {
    entry: { index: "src/index.ts" },
    dts: true,
    exports: false,
  },
});
