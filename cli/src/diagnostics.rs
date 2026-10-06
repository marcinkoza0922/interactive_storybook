//! Problems found while reading a book project, reported with file and line like a compiler.

use std::fmt;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    pub file: Option<PathBuf>,
    /// 1-based.
    pub line: Option<usize>,
    pub help: Option<String>,
}

/// Collects diagnostics; file paths are shown relative to the project root.
#[derive(Debug, Default)]
pub struct Diagnostics {
    pub items: Vec<Diagnostic>,
}

impl Diagnostics {
    pub fn error(&mut self, file: Option<&Path>, line: Option<usize>, message: impl Into<String>) -> &mut Diagnostic {
        self.push(Severity::Error, file, line, message)
    }

    pub fn warning(&mut self, file: Option<&Path>, line: Option<usize>, message: impl Into<String>) -> &mut Diagnostic {
        self.push(Severity::Warning, file, line, message)
    }

    fn push(&mut self, severity: Severity, file: Option<&Path>, line: Option<usize>, message: impl Into<String>) -> &mut Diagnostic {
        self.items.push(Diagnostic { severity, message: message.into(), file: file.map(Path::to_path_buf), line, help: None });
        self.items.last_mut().unwrap()
    }

    pub fn has_errors(&self) -> bool {
        self.items.iter().any(|d| d.severity == Severity::Error)
    }

    pub fn count(&self, severity: Severity) -> usize {
        self.items.iter().filter(|d| d.severity == severity).count()
    }

    /// Errors first, then by file and line, so related problems read together.
    pub fn sorted(&self) -> Vec<&Diagnostic> {
        let mut items: Vec<_> = self.items.iter().collect();
        items.sort_by(|a, b| b.severity.cmp(&a.severity).then_with(|| a.file.cmp(&b.file)).then_with(|| a.line.cmp(&b.line)));
        items
    }

    pub fn render(&self, root: &Path) -> String {
        self.sorted().iter().map(|d| d.render(root)).collect::<Vec<_>>().join("\n")
    }
}

impl Diagnostic {
    pub fn render(&self, root: &Path) -> String {
        let label = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        let mut out = format!("{label}: {}", self.message);
        if let Some(file) = &self.file {
            let shown = file.strip_prefix(root).unwrap_or(file);
            match self.line {
                Some(line) => out.push_str(&format!("\n  --> {}:{line}", shown.display())),
                None => out.push_str(&format!("\n  --> {}", shown.display())),
            }
        }
        if let Some(help) = &self.help {
            out.push_str(&format!("\n  help: {help}"));
        }
        out
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.render(Path::new("")))
    }
}

/// 1-based line number of a byte offset in `text`.
pub fn line_of(text: &str, offset: usize) -> usize {
    text[..offset.min(text.len())].matches('\n').count() + 1
}
