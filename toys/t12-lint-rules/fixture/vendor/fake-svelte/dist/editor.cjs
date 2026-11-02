require("node:fs").writeFileSync(process.env.T12_MARKER, "project code ran\n");
module.exports = { toTSX: () => "export default function Foo(_props: { a?: string }) { return null; }" };
