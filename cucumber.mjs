// TypeScript step definitions load through Node's native type stripping.
export default {
  paths: ["packages/*/tests/**/*.feature"],
  import: ["packages/*/tests/**/*.steps.ts"],
  strict: true,
};
