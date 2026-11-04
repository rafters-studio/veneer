// node --import ./modes.mjs ...: MODE=none|passthrough|generation|content|dispose.
// Each mode is an intervention at the seam veneer already patches (read.ts:80-83), kept in a toy so src/ is untouched.
import { createHash } from "node:crypto";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";

const pkg = fileURLToPath(new URL("../../packages/veneer/package.json", import.meta.url));
const require = createRequire(pkg);
const server = createRequire(require.resolve("@astrojs/language-server/package.json"));
const kit = server("@volar/kit/lib/createChecker.js");
const mode = process.env.MODE ?? "none";

let opens = 0;
let lastContext;
const capture = {
  name: "t164-capture",
  capabilities: {},
  create(context) {
    lastContext ??= context;
    return {};
  },
};
const contents = new WeakMap();
const hashOf = (snapshot) => {
  let hash = contents.get(snapshot);
  if (!hash) {
    hash = createHash("sha1").update(snapshot.getText(0, snapshot.getLength())).digest("base64");
    contents.set(snapshot, hash);
  }
  return hash;
};

// content-reread: as content, and kit's fsFileSnapshots (createChecker.js:28,51) can never match across opens.
if (mode === "content-reread") {
  const { sys } = require("typescript");
  const modified = sys.getModifiedTime;
  sys.getModifiedTime = (file) => {
    const time = modified.call(sys, file);
    return time && time.valueOf() >= 0 ? new Date(time.valueOf() + opens) : time;
  };
}

if (mode !== "none") {
  const original = kit.createTypeScriptChecker;
  kit.createTypeScriptChecker = (languages, services, tsconfig, references, setup) => {
    opens += 1;
    const open = opens;
    if (mode === "dispose" && lastContext) {
      lastContext.inject("typescript/languageService").dispose();
    }
    lastContext = undefined;
    return original(languages, [...services, capture], tsconfig, references, (args) => {
      setup?.(args);
      const host = args.project.typescript.languageServiceHost;
      const versionOf = host.getScriptVersion.bind(host);
      host.getScriptVersion = (file) => {
        const version = versionOf(file);
        if (version === "" || mode === "passthrough" || mode === "dispose") return version;
        if (mode === "generation") return `${open}:${version}`;
        // A fix aimed only at the issue's scenario: unsaved entries have a negative modified time (read.ts:182).
        if (mode === "unsaved-only") {
          return args.project.typescript.sys.getModifiedTime?.(file)?.valueOf() < 0
            ? `${open}:${version}`
            : version;
        }
        const snapshot = host.getScriptSnapshot(file);
        return snapshot ? hashOf(snapshot) : version;
      };
    });
  };
}
