// node concurrent.mjs <root>: what two check() calls at once return, with and without a save (which awaits real file I/O).
import { check } from "@rafters/veneer";

const [root] = process.argv.slice(2);
for (const [name, calls] of [
  ["two plain", [{}, {}]],
  ["save then plain", [{ save: true }, {}]],
]) {
  const settled = await Promise.allSettled(calls.map((options) => check(root, options)));
  console.log(
    name,
    settled.map((result) =>
      result.status === "fulfilled"
        ? `fulfilled ok=${result.value.ok}`
        : `rejected ${result.reason?.name} code=${result.reason?.code}`,
    ),
  );
}
