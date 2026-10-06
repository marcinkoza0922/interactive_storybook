//! Finding the files a book refers to and giving them content-hashed names in the bundle.
//!
//! A reference like `harbour` is looked up in the project's `assets/` folder (first in the
//! folder for its kind, e.g. `assets/audio/`), and the extension may be left off when only
//! one file matches. A reference starting with `./` or `../` is relative to the file it's
//! written in, which keeps image previews working in Markdown editors.

use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetKind {
    Audio,
    Image,
    /// Images or video, e.g. a background.
    Media,
    Font,
    Css,
}

impl AssetKind {
    fn extensions(self) -> &'static [&'static str] {
        match self {
            AssetKind::Audio => &["ogg", "opus", "mp3", "m4a", "aac", "wav", "flac", "webm"],
            AssetKind::Image => &["png", "jpg", "jpeg", "gif", "webp", "avif", "svg"],
            AssetKind::Media => &["png", "jpg", "jpeg", "gif", "webp", "avif", "svg", "webm", "mp4", "ogv"],
            AssetKind::Font => &["woff2", "woff", "ttf", "otf"],
            AssetKind::Css => &["css"],
        }
    }

    fn folders(self) -> &'static [&'static str] {
        match self {
            AssetKind::Audio => &["audio"],
            // Video posters usually sit next to their videos.
            AssetKind::Image => &["images", "video"],
            AssetKind::Media => &["images", "video"],
            AssetKind::Font => &["fonts"],
            AssetKind::Css => &["theme", "css"],
        }
    }

    fn noun(self) -> &'static str {
        match self {
            AssetKind::Audio => "audio",
            AssetKind::Image => "image",
            AssetKind::Media => "image or video",
            AssetKind::Font => "font",
            AssetKind::Css => "stylesheet",
        }
    }
}

/// The assets a book uses, keyed by source file, with their bundle paths.
#[derive(Debug)]
pub struct Assets {
    root: PathBuf,
    by_source: BTreeMap<PathBuf, String>,
}

impl Assets {
    pub fn new(root: &Path) -> Self {
        Assets { root: root.to_path_buf(), by_source: BTreeMap::new() }
    }

    /// Resolve a reference written in `from_file`, returning its path in the bundle
    /// (relative to `book.json`), or a message explaining what's wrong.
    pub fn resolve(&mut self, reference: &str, from_file: &Path, kind: AssetKind) -> Result<String, String> {
        let source = self.locate(reference, from_file, kind)?;
        if let Some(path) = self.by_source.get(&source) {
            return Ok(path.clone());
        }

        let extension = source.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
        if !kind.extensions().contains(&extension.as_str()) {
            return Err(format!(
                "`{reference}` isn't a supported {} file; use one of: {}",
                kind.noun(),
                kind.extensions().join(", ")
            ));
        }

        let bytes = fs::read(&source).map_err(|e| format!("couldn't read `{}`: {e}", source.display()))?;
        let stem = source.file_stem().and_then(|s| s.to_str()).unwrap_or("asset");
        let stem: String = stem.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '-' }).collect();
        let path = format!("assets/{stem}.{}.{extension}", short_hash(&bytes));
        self.by_source.insert(source, path.clone());
        Ok(path)
    }

    fn locate(&self, reference: &str, from_file: &Path, kind: AssetKind) -> Result<PathBuf, String> {
        let reference = reference.trim();
        if reference.is_empty() {
            return Err(format!("an empty {} reference", kind.noun()));
        }
        if reference.contains("://") || reference.starts_with('/') {
            return Err(format!("`{reference}` must be a file in the project, not an absolute path or URL"));
        }

        let assets = self.root.join("assets");
        let candidates: Vec<PathBuf> = if reference.starts_with("./") || reference.starts_with("../") {
            vec![from_file.parent().unwrap_or(&self.root).join(reference)]
        } else {
            kind.folders().iter().map(|f| assets.join(f).join(reference)).chain([assets.join(reference)]).collect()
        };

        for candidate in &candidates {
            if candidate.is_file() {
                return Ok(normalize(candidate));
            }
            // Without an extension: a unique file of the right kind with that name.
            if candidate.extension().is_none() {
                let matches = with_stem(candidate, kind);
                match matches.len() {
                    0 => {}
                    1 => return Ok(normalize(&matches[0])),
                    _ => {
                        let names: Vec<_> = matches.iter().filter_map(|m| m.file_name()?.to_str().map(String::from)).collect();
                        return Err(format!("`{reference}` could mean any of {}; add the extension", names.join(", ")));
                    }
                }
            }
        }

        let looked_in = candidates
            .iter()
            .filter_map(|c| c.parent()?.strip_prefix(&self.root).ok().map(|p| p.display().to_string()))
            .collect::<Vec<_>>();
        Err(format!("can't find the {} `{reference}` (looked in {})", kind.noun(), looked_in.join(", ")))
    }

    /// Every asset as (source file, bundle path).
    pub fn files(&self) -> impl Iterator<Item = (&Path, &str)> {
        self.by_source.iter().map(|(source, path)| (source.as_path(), path.as_str()))
    }
}

fn with_stem(candidate: &Path, kind: AssetKind) -> Vec<PathBuf> {
    let (Some(dir), Some(stem)) = (candidate.parent(), candidate.file_name().and_then(|s| s.to_str())) else { return vec![] };
    let Ok(entries) = fs::read_dir(dir) else { return vec![] };
    let mut found: Vec<PathBuf> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_file() && p.file_stem().and_then(|s| s.to_str()) == Some(stem))
        .filter(|p| p.extension().and_then(|e| e.to_str()).is_some_and(|e| kind.extensions().contains(&e.to_ascii_lowercase().as_str())))
        .collect();
    found.sort();
    found
}

fn normalize(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

/// First 8 bytes of SHA-256, as hex.
pub fn short_hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().take(8).map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for file in ["assets/audio/harbour.ogg", "assets/audio/bell.ogg", "assets/audio/bell.mp3", "assets/images/map.png", "manuscript/local.png"] {
            let path = dir.path().join(file);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, file).unwrap();
        }
        dir
    }

    #[test]
    fn resolves_by_name_in_the_kind_folder() {
        let dir = project();
        let mut assets = Assets::new(dir.path());
        let chapter = dir.path().join("manuscript/01.md");
        let path = assets.resolve("harbour", &chapter, AssetKind::Audio).unwrap();
        assert!(path.starts_with("assets/harbour.") && path.ends_with(".ogg"), "{path}");
        assert_eq!(assets.resolve("audio/harbour.ogg", &chapter, AssetKind::Audio).unwrap(), path, "same file, same name");
        assert!(assets.resolve("map", &chapter, AssetKind::Image).is_ok());
        assert!(assets.resolve("./local.png", &chapter, AssetKind::Image).is_ok());
    }

    #[test]
    fn explains_missing_ambiguous_and_wrong_files() {
        let dir = project();
        let mut assets = Assets::new(dir.path());
        let chapter = dir.path().join("manuscript/01.md");
        assert!(assets.resolve("storm", &chapter, AssetKind::Audio).unwrap_err().contains("can't find the audio `storm`"));
        assert!(assets.resolve("bell", &chapter, AssetKind::Audio).unwrap_err().contains("bell.mp3, bell.ogg"));
        assert!(assets.resolve("images/map.png", &chapter, AssetKind::Audio).unwrap_err().contains("isn't a supported audio file"));
        assert!(assets.resolve("https://x.test/a.png", &chapter, AssetKind::Image).is_err());
    }
}
