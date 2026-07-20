//! Emit a real preview module + the shared runtime to a directory, so the
//! generated output can be driven in a browser. Verification aid: the unit
//! tests assert structure, this produces the artifact you can actually load.
use std::env;
use std::fs;

use veneer_adapters::{
    generator::{runtime_module, RUNTIME_MODULE_FILENAME},
    ComponentConventions, ReactAdapter,
};

fn main() {
    let out = env::args().nth(1).expect("usage: emit_preview <outdir>");
    let classes = env::args().nth(2).expect("usage: emit_preview <outdir> <classes.ts>");
    fs::create_dir_all(&out).unwrap();

    let ts = fs::read_to_string(&classes).unwrap();
    let adapter = ReactAdapter::with_conventions(ComponentConventions::for_classes_file("badge"));
    let structure = adapter.extract_structure(&ts).expect("structure");
    let block = veneer_adapters::generator::web_component_block("badge-preview", &structure);

    fs::write(format!("{out}/{RUNTIME_MODULE_FILENAME}"), runtime_module()).unwrap();
    fs::write(format!("{out}/badge-preview.js"), &block.web_component).unwrap();
    println!("wrote {out}/badge-preview.js and {RUNTIME_MODULE_FILENAME}");
    println!("classes_used: {:?}", block.classes_used);
}
