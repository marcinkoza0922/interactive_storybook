//! `tome preview`: serve the book locally and rebuild it whenever a file changes.

use crate::compile::compile;
use crate::output::{RUNTIME, bundle_json};
use anyhow::{Context, Result, anyhow};
use notify::event::{EventKind, ModifyKind};
use notify::{RecursiveMode, Watcher};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tiny_http::{Header, Request, Response, Server, StatusCode};

#[derive(Default)]
struct State {
    /// Bumped on every rebuild; the page reloads when it changes and the build succeeded.
    version: u64,
    bundle: Option<String>,
    assets: HashMap<String, PathBuf>,
    report: String,
    errors: bool,
}

/// Polls for rebuilds; reloads on success, and shows build errors over the page.
const LIVE_RELOAD: &str = r#"<script>
(() => {
  let version = null
  const overlay = document.createElement('pre')
  overlay.setAttribute('role', 'alert')
  overlay.style.cssText = 'position:fixed;inset:auto 1rem 1rem;max-height:50vh;overflow:auto;margin:0;padding:1rem 1.25rem;' +
    'background:#2b1d1d;color:#ffdcd2;font:13px/1.45 ui-monospace,monospace;border-radius:6px;z-index:2147483647;' +
    'white-space:pre-wrap;box-shadow:0 8px 32px rgb(0 0 0 / .45)'
  async function poll() {
    try {
      const status = await (await fetch('/__tome/status', { cache: 'no-store' })).json()
      if (version !== null && status.version !== version && !status.errors) return location.reload()
      version = status.version
      if (status.errors) {
        overlay.textContent = 'The book has errors, so this is the last version that built.\n\n' + status.report
        if (!overlay.isConnected) document.body.append(overlay)
      } else overlay.remove()
    } catch {}
    setTimeout(poll, 700)
  }
  poll()
})()
</script>"#;

#[allow(clippy::print_stdout, clippy::print_stderr, reason = "the CLI reports to the terminal")]
pub fn serve(root: &Path, port: u16, open: bool) -> Result<()> {
    let root = root.canonicalize().with_context(|| format!("{} doesn't exist", root.display()))?;
    let state = Arc::new(RwLock::new(State::default()));
    rebuild(&root, &state);

    let server = Server::http(("127.0.0.1", port)).map_err(|e| anyhow!("couldn't start the preview server on port {port}: {e}"))?;
    let url = format!("http://localhost:{port}/?preview");
    println!("Previewing at {url}\nEdits rebuild the book automatically. Press Ctrl+C to stop.");
    if open {
        open_browser(&url);
    }

    let (tx, rx) = mpsc::channel();
    let mut watcher = notify::recommended_watcher(tx).context("couldn't watch the project for changes")?;
    watcher.watch(&root, RecursiveMode::Recursive)?;
    let watch_state = Arc::clone(&state);
    let watch_root = root.clone();
    std::thread::spawn(move || {
        while let Ok(event) = rx.recv() {
            // Only real changes: some platforms also report reads, and building reads everything.
            let changed = event.is_ok_and(|e| {
                matches!(e.kind, EventKind::Create(_) | EventKind::Remove(_) | EventKind::Modify(ModifyKind::Data(_) | ModifyKind::Name(_) | ModifyKind::Any))
                    && e.paths.iter().any(|p| relevant(&watch_root, p))
            });
            if !changed {
                continue;
            }
            // Editors save in several steps; wait for them to settle.
            while rx.recv_timeout(Duration::from_millis(150)).is_ok() {}
            rebuild(&watch_root, &watch_state);
        }
    });

    for request in server.incoming_requests() {
        let state = Arc::clone(&state);
        std::thread::spawn(move || {
            if let Err(error) = respond(request, &state) {
                eprintln!("preview: {error}");
            }
        });
    }
    Ok(())
}

/// Changes worth rebuilding for: not build output, version control or editor scratch files.
fn relevant(root: &Path, path: &Path) -> bool {
    let Ok(relative) = path.strip_prefix(root) else { return false };
    let first = relative.components().next().and_then(|c| c.as_os_str().to_str()).unwrap_or("");
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    !matches!(first, "dist" | ".git" | "target" | "node_modules") && !name.ends_with('~') && !name.starts_with(".#") && !name.ends_with(".swp")
}

#[allow(clippy::print_stdout, clippy::print_stderr, reason = "the CLI reports to the terminal")]
fn rebuild(root: &Path, state: &RwLock<State>) {
    let compiled = compile(root);
    let report = compiled.diagnostics.render(root);
    let errors = compiled.diagnostics.has_errors();
    let mut state = state.write().unwrap();
    state.version += 1;
    state.errors = errors;
    state.report = report.clone();

    if errors {
        eprintln!("\n{report}\n\nBuild failed; still serving the last version that built.");
    } else {
        match bundle_json(&compiled) {
            Ok(json) => {
                state.bundle = Some(json);
                state.assets = compiled.assets.files().map(|(source, path)| (path.to_string(), source.to_path_buf())).collect();
            }
            Err(error) => eprintln!("{error}"),
        }
        if report.is_empty() {
            println!("Rebuilt.");
        } else {
            println!("\n{report}\n\nRebuilt, with warnings.");
        }
    }
}

fn respond(request: Request, state: &RwLock<State>) -> Result<()> {
    let path = request.url().split(['?', '#']).next().unwrap_or("/").trim_start_matches('/').to_string();

    if path == "__tome/status" {
        let state = state.read().unwrap();
        let body = serde_json::json!({ "version": state.version, "errors": state.errors, "report": state.report }).to_string();
        return send(request, Response::from_string(body).with_header(header("Content-Type", "application/json")));
    }
    if path.is_empty() || path == "index.html" {
        let html = RUNTIME.get_file("index.html").and_then(|f| f.contents_utf8()).unwrap_or("");
        let html = html.replacen("</body>", &format!("{LIVE_RELOAD}</body>"), 1);
        return send(request, Response::from_string(html).with_header(header("Content-Type", "text/html; charset=utf-8")));
    }
    if path == "book/book.json" {
        let bundle = state.read().unwrap().bundle.clone();
        return match bundle {
            Some(json) => send(request, Response::from_string(json).with_header(header("Content-Type", "application/json"))),
            None => send(request, Response::from_string("The book hasn't built yet; see the terminal.").with_status_code(503)),
        };
    }
    if let Some(asset) = path.strip_prefix("book/") {
        let source = state.read().unwrap().assets.get(asset).cloned();
        return match source {
            Some(source) => send_file(request, &source),
            None => send(request, Response::from_string("Not found").with_status_code(404)),
        };
    }
    match RUNTIME.get_file(&path) {
        Some(file) => send(request, Response::from_data(file.contents()).with_header(header("Content-Type", content_type(&path)))),
        None => send(request, Response::from_string("Not found").with_status_code(404)),
    }
}

/// Serve a file, honouring byte ranges so audio and video can seek and loop.
fn send_file(request: Request, source: &Path) -> Result<()> {
    let mut file = File::open(source)?;
    let length = file.metadata()?.len();
    let kind = header("Content-Type", content_type(&source.to_string_lossy()));
    let range = request
        .headers()
        .iter()
        .find(|h| h.field.equiv("Range"))
        .and_then(|h| parse_range(h.value.as_str(), length));

    match range {
        Some((start, end)) => {
            file.seek(SeekFrom::Start(start))?;
            let size = end - start + 1;
            let response = Response::new(
                StatusCode(206),
                vec![kind, header("Accept-Ranges", "bytes"), header("Content-Range", &format!("bytes {start}-{end}/{length}"))],
                file.take(size),
                Some(size as usize),
                None,
            );
            Ok(request.respond(response)?)
        }
        None => send(request, Response::from_file(file).with_header(kind).with_header(header("Accept-Ranges", "bytes"))),
    }
}

/// `bytes=start-end`, `bytes=start-` or `bytes=-suffix`, clamped to the file.
fn parse_range(value: &str, length: u64) -> Option<(u64, u64)> {
    let spec = value.strip_prefix("bytes=")?.split(',').next()?.trim();
    let (start, end) = spec.split_once('-')?;
    let last = length.checked_sub(1)?;
    let (start, end) = match (start.parse::<u64>().ok(), end.parse::<u64>().ok()) {
        (Some(start), Some(end)) => (start, end.min(last)),
        (Some(start), None) => (start, last),
        (None, Some(suffix)) => (length.saturating_sub(suffix), last),
        (None, None) => return None,
    };
    (start <= end).then_some((start, end))
}

fn send<R: Read>(request: Request, response: Response<R>) -> Result<()> {
    Ok(request.respond(response.with_header(header("Cache-Control", "no-store")))?)
}

fn header(name: &str, value: &str) -> Header {
    Header::from_bytes(name.as_bytes(), value.as_bytes()).expect("valid header")
}

pub fn content_type(path: &str) -> &'static str {
    let extension = path.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    match extension.as_str() {
        "html" => "text/html; charset=utf-8",
        "js" => "text/javascript",
        "css" => "text/css",
        "json" => "application/json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "ogg" | "opus" => "audio/ogg",
        "mp3" => "audio/mpeg",
        "m4a" | "aac" => "audio/mp4",
        "wav" => "audio/wav",
        "flac" => "audio/flac",
        "webm" => "video/webm",
        "mp4" => "video/mp4",
        "ogv" => "video/ogg",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        _ => "application/octet-stream",
    }
}

#[allow(clippy::print_stdout, clippy::print_stderr, reason = "the CLI reports to the terminal")]
fn open_browser(url: &str) {
    let command = if cfg!(target_os = "windows") {
        std::process::Command::new("cmd").args(["/C", "start", "", url]).spawn()
    } else if cfg!(target_os = "macos") {
        std::process::Command::new("open").arg(url).spawn()
    } else {
        std::process::Command::new("xdg-open").arg(url).spawn()
    };
    if command.is_err() {
        println!("Open {url} in your browser.");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_byte_ranges() {
        assert_eq!(parse_range("bytes=0-99", 1000), Some((0, 99)));
        assert_eq!(parse_range("bytes=900-", 1000), Some((900, 999)));
        assert_eq!(parse_range("bytes=-100", 1000), Some((900, 999)));
        assert_eq!(parse_range("bytes=0-5000", 1000), Some((0, 999)));
        assert_eq!(parse_range("bytes=500-100", 1000), None);
        assert_eq!(parse_range("items=0-1", 1000), None);
    }

    #[test]
    fn ignores_output_and_scratch_files() {
        let root = Path::new("/book");
        assert!(relevant(root, Path::new("/book/manuscript/01.md")));
        assert!(!relevant(root, Path::new("/book/dist/book/book.json")));
        assert!(!relevant(root, Path::new("/book/manuscript/01.md~")));
        assert!(!relevant(root, Path::new("/book/.git/index")));
    }
}
