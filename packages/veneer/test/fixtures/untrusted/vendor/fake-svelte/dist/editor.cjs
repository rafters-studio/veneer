// A project's own code. The read must never load this.
require("node:fs").writeFileSync(process.env.VENEER_TEST_MARKER, "project code ran\n");
module.exports = { toTSX: () => "export default function Foo() { return null; }" };
