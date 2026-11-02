// T15: starts the built server, with --worker from the environment, on files selected by pattern.
const { LanguageClient, TransportKind } = require("vscode-languageclient/node");
let client;
exports.activate = async () => {
  client = new LanguageClient(
    "veneer",
    "veneer",
    {
      command: process.env.T15_SERVER,
      args: ["--worker", process.env.T15_WORKER],
      transport: TransportKind.stdio,
    },
    { documentSelector: [{ scheme: "file", pattern: "**/*.astro" }, { scheme: "file", pattern: "**/*.tsx" }] },
  );
  await client.start();
};
exports.deactivate = () => client?.stop();
