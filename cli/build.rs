//! The CLI embeds the prebuilt web runtime, so authors never need Node to build a book.

use std::path::Path;

fn main() {
    // Read at run time, not with env!(): a cached build script must not remember an old
    // location if the project folder moves.
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR");
    let dist = Path::new(&manifest_dir).join("../runtime/dist");
    println!("cargo:rerun-if-changed={}", dist.display());
    if !dist.join("index.html").is_file() {
        panic!("\n\nThe web runtime hasn't been built. Run this first:\n\n    cd runtime && npm install && npm run build\n\n");
    }
}
