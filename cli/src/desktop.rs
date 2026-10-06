//! Desktop builds: a book's web build inside Electron's prebuilt release, so authors need
//! neither Node nor a JavaScript toolchain. The release is downloaded once, verified against
//! checksums pinned here, and cached.

use crate::compile::Compiled;
use crate::output;
use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

pub const ELECTRON_VERSION: &str = "44.5.1";

const MAIN_JS: &str = include_str!("../templates/electron/main.js");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Linux,
    Windows,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arch {
    X64,
    Arm64,
}

impl Platform {
    fn electron_name(self) -> &'static str {
        match self {
            Platform::Linux => "linux",
            Platform::Windows => "win32",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Platform::Linux => "linux",
            Platform::Windows => "windows",
        }
    }
}

impl Arch {
    pub fn name(self) -> &'static str {
        match self {
            Arch::X64 => "x64",
            Arch::Arm64 => "arm64",
        }
    }
}

/// SHA-256 of each pinned release, from Electron's SHASUMS256.txt.
fn expected_sha256(platform: Platform, arch: Arch) -> &'static str {
    match (platform, arch) {
        (Platform::Linux, Arch::X64) => "5bcd217611d6843ececd6c9e9c1fcd1da3ab066c43d8b1a9e4b44689a1fba6f5",
        (Platform::Linux, Arch::Arm64) => "ee1790d743af1abd6a7e3971589dcd8635b58dab51ca1359123ba5216ce3a453",
        (Platform::Windows, Arch::X64) => "9b382492dcfee91f8f9e92c91f7972550a1b95d2299cac72279dab33a600d7db",
        (Platform::Windows, Arch::Arm64) => "6ba86d5d5cfdf4ce61d51e2d270eaca464a7f88c143d14c6d9b3f43bb0760e9e",
    }
}

fn release_name(platform: Platform, arch: Arch) -> String {
    format!("electron-v{ELECTRON_VERSION}-{}-{}.zip", platform.electron_name(), arch.name())
}

/// The cached Electron release, downloading it first if needed.
pub fn electron_release(platform: Platform, arch: Arch) -> Result<PathBuf> {
    let cache = dirs::cache_dir().context("couldn't find a cache folder for downloads")?.join("tome").join("electron");
    fs::create_dir_all(&cache)?;
    let file = cache.join(release_name(platform, arch));
    let expected = expected_sha256(platform, arch);

    if file.is_file() {
        if sha256_file(&file)? == expected {
            return Ok(file);
        }
        eprintln!("The cached {} is damaged; downloading it again.", release_name(platform, arch));
        fs::remove_file(&file)?;
    }

    let url = format!("https://github.com/electron/electron/releases/download/v{ELECTRON_VERSION}/{}", release_name(platform, arch));
    eprintln!("Downloading Electron {ELECTRON_VERSION} for {} {} (once; it's cached for later builds)…", platform.name(), arch.name());
    let partial = file.with_extension("zip.part");
    download(&url, &partial).with_context(|| format!("couldn't download {url}"))?;

    let actual = sha256_file(&partial)?;
    if actual != expected {
        fs::remove_file(&partial).ok();
        bail!("the download of {} doesn't match its published checksum (expected {expected}, got {actual}); try again", release_name(platform, arch));
    }
    fs::rename(&partial, &file)?;
    Ok(file)
}

fn download(url: &str, to: &Path) -> Result<()> {
    let mut response = ureq::get(url).call()?;
    let total: Option<u64> = response.headers().get("content-length").and_then(|v| v.to_str().ok()?.parse().ok());
    let mut reader = response.body_mut().as_reader();
    let mut out = File::create(to)?;
    let mut buffer = vec![0; 1 << 16];
    let (mut done, mut shown) = (0u64, 0u64);
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        out.write_all(&buffer[..read])?;
        done += read as u64;
        if let Some(total) = total.filter(|t| *t > 0) {
            let percent = done * 100 / total;
            if percent >= shown + 10 {
                shown = percent - percent % 10;
                eprintln!("  {shown}% of {} MB", total / 1_000_000);
            }
        }
    }
    Ok(())
}

fn sha256_file(path: &Path) -> Result<String> {
    let mut hasher = Sha256::new();
    let mut file = File::open(path)?;
    let mut buffer = vec![0; 1 << 16];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

/// What a packaged desktop app is called.
pub struct AppNames {
    /// The folder and, on Windows, the executable: the book's title, made file-name safe.
    pub product: String,
    /// The executable on Linux, and the archive's name: the book's ID.
    pub slug: String,
}

pub fn app_names(compiled: &Compiled) -> AppNames {
    let product: String = compiled.bundle.book.title.chars().filter(|c| !r#"<>:"/\|?*"#.contains(*c) && !c.is_control()).collect();
    let product = product.trim().trim_end_matches('.').to_string();
    AppNames { product: if product.is_empty() { compiled.bundle.book.id.clone() } else { product }, slug: compiled.bundle.book.id.clone() }
}

/// Build a desktop app into `out`: `<out>/<Product>/` (runnable as is) and an archive of it
/// to distribute. Returns the archive's path.
pub fn build(compiled: &Compiled, out: &Path, platform: Platform, arch: Arch, release: &Path) -> Result<PathBuf> {
    output::prepare(out)?;
    let names = app_names(compiled);
    let app_dir = out.join(&names.product);

    extract_zip(release, &app_dir).with_context(|| format!("couldn't unpack {}", release.display()))?;

    // Replace Electron's default app with the book.
    let resources = app_dir.join("resources");
    fs::remove_file(resources.join("default_app.asar")).ok();
    let app = resources.join("app");
    output::write_web(compiled, &app.join("web"))?;
    fs::write(app.join("main.js"), MAIN_JS)?;
    fs::write(app.join("package.json"), package_json(compiled)?)?;

    let (from, to) = match platform {
        Platform::Linux => ("electron", names.slug.clone()),
        Platform::Windows => ("electron.exe", format!("{}.exe", names.product)),
    };
    fs::rename(app_dir.join(from), app_dir.join(&to)).with_context(|| format!("the Electron release has no {from}"))?;

    let archive = match platform {
        Platform::Linux => {
            let archive = out.join(format!("{}-linux-{}.tar.gz", names.slug, arch.name()));
            write_tar_gz(&app_dir, &names.product, &archive)?;
            archive
        }
        Platform::Windows => {
            let archive = out.join(format!("{}-windows-{}.zip", names.slug, arch.name()));
            write_zip(&app_dir, &names.product, &archive)?;
            archive
        }
    };
    output::mark(out)?;
    Ok(archive)
}

fn package_json(compiled: &Compiled) -> Result<String> {
    let book = &compiled.bundle.book;
    let background = compiled
        .bundle
        .theme
        .as_ref()
        .and_then(|t| t.tokens.get("page-bg"))
        .filter(|c| c.starts_with('#'))
        .cloned()
        .unwrap_or_else(|| "#f6f1e7".into());
    Ok(serde_json::to_string_pretty(&serde_json::json!({
        "name": book.id,
        "productName": book.title,
        "version": "1.0.0",
        "main": "main.js",
        "private": true,
        "tome": { "id": book.id, "title": book.title, "background": background },
    }))?)
}

fn extract_zip(archive: &Path, to: &Path) -> Result<()> {
    let mut zip = zip::ZipArchive::new(File::open(archive)?)?;
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index)?;
        // enclosed_name rejects entries that would escape the target folder.
        let Some(relative) = entry.enclosed_name() else { bail!("unsafe path in the archive: {}", entry.name()) };
        let path = to.join(relative);
        if entry.is_dir() {
            fs::create_dir_all(&path)?;
            continue;
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        io::copy(&mut entry, &mut File::create(&path)?)?;
        #[cfg(unix)]
        if let Some(mode) = entry.unix_mode() {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(mode & 0o7777))?;
        }
    }
    Ok(())
}

/// Files under `dir`, relative to it, sorted for reproducible archives.
fn walk(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        for entry in fs::read_dir(&current)? {
            let path = entry?.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                files.push(path.strip_prefix(dir)?.to_path_buf());
            }
        }
    }
    files.sort();
    Ok(files)
}

fn write_tar_gz(dir: &Path, top: &str, archive: &Path) -> Result<()> {
    let encoder = flate2::write::GzEncoder::new(File::create(archive)?, flate2::Compression::default());
    let mut tar = tar::Builder::new(encoder);
    for relative in walk(dir)? {
        // Keeps permissions, so the executable stays executable.
        tar.append_path_with_name(dir.join(&relative), Path::new(top).join(&relative))?;
    }
    tar.into_inner()?.finish()?;
    Ok(())
}

fn write_zip(dir: &Path, top: &str, archive: &Path) -> Result<()> {
    let mut zip = zip::ZipWriter::new(File::create(archive)?);
    let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for relative in walk(dir)? {
        let name = Path::new(top).join(&relative).to_string_lossy().replace('\\', "/");
        zip.start_file(name, options)?;
        io::copy(&mut File::open(dir.join(&relative))?, &mut zip)?;
    }
    zip.finish()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile::compile;

    /// A stand-in for an Electron release with the files packaging touches.
    fn fake_release(dir: &Path, executable: &str) -> PathBuf {
        let path = dir.join("electron.zip");
        let mut zip = zip::ZipWriter::new(File::create(&path).unwrap());
        let executable_mode = zip::write::SimpleFileOptions::default().unix_permissions(0o755);
        let plain = zip::write::SimpleFileOptions::default().unix_permissions(0o644);
        zip.start_file(executable, executable_mode).unwrap();
        zip.write_all(b"binary").unwrap();
        for (name, contents) in [("resources/default_app.asar", "asar"), ("LICENSE", "MIT"), ("locales/en-US.pak", "pak")] {
            zip.start_file(name, plain).unwrap();
            zip.write_all(contents.as_bytes()).unwrap();
        }
        zip.finish().unwrap();
        path
    }

    fn sample() -> Compiled {
        let compiled = compile(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples/the-uneven-bell"));
        assert!(!compiled.diagnostics.has_errors());
        compiled
    }

    #[test]
    fn packages_a_linux_app() {
        let dir = tempfile::tempdir().unwrap();
        let release = fake_release(dir.path(), "electron");
        let out = dir.path().join("linux-x64");
        let archive = build(&sample(), &out, Platform::Linux, Arch::X64, &release).unwrap();

        let app = out.join("The Uneven Bell");
        assert!(app.join("the-uneven-bell").is_file(), "executable renamed after the book");
        assert!(!app.join("electron").exists());
        assert!(!app.join("resources/default_app.asar").exists());
        assert!(app.join("resources/app/main.js").is_file());
        assert!(app.join("resources/app/web/index.html").is_file());
        assert!(app.join("resources/app/web/book/book.json").is_file());
        assert!(app.join("LICENSE").is_file(), "Electron's licenses ship with the app");

        let package: serde_json::Value = serde_json::from_str(&fs::read_to_string(app.join("resources/app/package.json")).unwrap()).unwrap();
        assert_eq!(package["tome"]["id"], "the-uneven-bell");
        assert_eq!(package["main"], "main.js");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(fs::metadata(app.join("the-uneven-bell")).unwrap().permissions().mode() & 0o777, 0o755);
        }

        // The archive unpacks to the same app, still executable.
        assert_eq!(archive.file_name().unwrap(), "the-uneven-bell-linux-x64.tar.gz");
        let mut tar = tar::Archive::new(flate2::read::GzDecoder::new(File::open(&archive).unwrap()));
        let entries: Vec<(String, u32)> = tar
            .entries()
            .unwrap()
            .map(|e| {
                let e = e.unwrap();
                (e.path().unwrap().to_string_lossy().into_owned(), e.header().mode().unwrap())
            })
            .collect();
        assert!(entries.iter().any(|(p, mode)| p == "The Uneven Bell/the-uneven-bell" && mode & 0o111 != 0), "{entries:?}");
        assert!(entries.iter().any(|(p, _)| p == "The Uneven Bell/resources/app/web/book/book.json"));
    }

    #[test]
    fn packages_a_windows_app() {
        let dir = tempfile::tempdir().unwrap();
        let release = fake_release(dir.path(), "electron.exe");
        let out = dir.path().join("windows-x64");
        let archive = build(&sample(), &out, Platform::Windows, Arch::X64, &release).unwrap();
        assert!(out.join("The Uneven Bell/The Uneven Bell.exe").is_file());

        let mut zip = zip::ZipArchive::new(File::open(&archive).unwrap()).unwrap();
        assert!(zip.by_name("The Uneven Bell/The Uneven Bell.exe").is_ok());
        assert!(zip.by_name("The Uneven Bell/resources/app/web/index.html").is_ok());
    }

    #[test]
    fn makes_file_name_safe_product_names() {
        let mut compiled = sample();
        compiled.bundle.book.title = "Why? A Tale: Part 1.".into();
        assert_eq!(app_names(&compiled).product, "Why A Tale Part 1");
    }
}
