//! Writing a compiled book to disk: the embedded runtime plus `book/` with the bundle and assets.

use crate::compile::Compiled;
use anyhow::{Context, Result, bail};
use include_dir::{Dir, include_dir};
use std::fs;
use std::path::Path;

/// The prebuilt web runtime, embedded at compile time (see build.rs).
pub static RUNTIME: Dir = include_dir!("$CARGO_MANIFEST_DIR/../runtime/dist");

/// Marks a folder as tome's output, so rebuilding may safely replace it.
const MARKER: &str = ".tome-output";

pub fn bundle_json(compiled: &Compiled) -> Result<String> {
    serde_json::to_string(&compiled.bundle).context("couldn't serialize the bundle")
}

/// A playable web build: open `index.html` from a web server, or host the folder anywhere.
pub fn write_web(compiled: &Compiled, out: &Path) -> Result<()> {
    prepare(out)?;
    RUNTIME.extract(out).with_context(|| format!("couldn't write the runtime to {}", out.display()))?;
    write_book(compiled, &out.join("book"))?;
    mark(out)
}

/// Just `book.json` and its assets, e.g. for the runtime's development server.
pub fn write_bundle_only(compiled: &Compiled, out: &Path) -> Result<()> {
    prepare(out)?;
    write_book(compiled, out)?;
    mark(out)
}

/// Record that tome made this folder, so the next build may replace it.
pub fn mark(out: &Path) -> Result<()> {
    fs::write(out.join(MARKER), "Built by tome. This folder is replaced on every build.\n")?;
    Ok(())
}

fn write_book(compiled: &Compiled, book: &Path) -> Result<()> {
    fs::create_dir_all(book.join("assets"))?;
    fs::write(book.join("book.json"), bundle_json(compiled)?)?;
    for (source, path) in compiled.assets.files() {
        fs::copy(source, book.join(path)).with_context(|| format!("couldn't copy {}", source.display()))?;
    }
    Ok(())
}

/// Empty the output folder, refusing to touch one tome didn't create.
pub fn prepare(out: &Path) -> Result<()> {
    if out.exists() {
        let is_ours = out.join(MARKER).is_file();
        let is_empty = fs::read_dir(out)?.next().is_none();
        if !is_ours && !is_empty {
            bail!("{} already exists and wasn't created by tome; choose another --out folder or empty it first", out.display());
        }
        fs::remove_dir_all(out).with_context(|| format!("couldn't clear {}", out.display()))?;
    }
    fs::create_dir_all(out).with_context(|| format!("couldn't create {}", out.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_to_replace_folders_it_did_not_create() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("out");
        fs::create_dir_all(&out).unwrap();
        fs::write(out.join("important.txt"), "keep me").unwrap();
        assert!(prepare(&out).is_err());
        assert!(out.join("important.txt").exists());

        fs::write(out.join(MARKER), "").unwrap();
        prepare(&out).unwrap();
        assert!(!out.join("important.txt").exists());
    }

    #[test]
    fn embeds_the_runtime() {
        assert!(RUNTIME.get_file("index.html").is_some());
    }
}
