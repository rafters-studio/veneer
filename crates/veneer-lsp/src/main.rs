//! Language server for `.rafter` files and rafters code. It answers from
//! `veneer-check`, the same core `rafters check` runs.

fn main() {
    println!("veneer-lsp {}", veneer_check::version());
}
