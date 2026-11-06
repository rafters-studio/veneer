// Run the wasm-bindgen form of A once on a .rafter file: node run-bindgen.cjs <file.rafter>
const { readFileSync } = require('node:fs');
const { compile_js } = require('./wasm/bg-node/a_bg.js');

const result = JSON.parse(compile_js(readFileSync(process.argv[2], 'utf8')));
console.log(JSON.stringify(result, null, 2));
