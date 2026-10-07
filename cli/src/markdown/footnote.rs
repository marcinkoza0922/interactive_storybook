//! Footnotes: definitions gathered from anywhere in a chapter file, numbered in the order
//! the chapter first refers to them.

use crate::diagnostics::Diagnostics;
use std::collections::HashMap;
use std::path::Path;

/// A footnote with its number in the chapter, ready to go on the pages that refer to it.
#[derive(Debug, Clone)]
pub struct Footnote {
    pub label: String,
    pub number: u32,
    pub html: String,
    /// Text for reference matching, like a block's.
    pub match_text: String,
    pub forced_refs: Vec<String>,
}

/// A footnote's content as defined, before it's numbered.
#[derive(Debug)]
pub struct Definition {
    pub html: String,
    pub match_text: String,
    pub forced_refs: Vec<String>,
    pub line: usize,
}

/// A chapter's footnotes while it's being read.
#[derive(Debug, Default)]
pub struct Footnotes {
    /// Labels in order of first reference; a footnote's number is its place here, from 1.
    pub labels: Vec<String>,
    referenced_at: HashMap<String, usize>,
    definitions: HashMap<String, Definition>,
}

impl Footnotes {
    pub fn referenced(&mut self, labels: &[String], line: usize) {
        for label in labels {
            self.referenced_at.entry(label.clone()).or_insert(line);
        }
    }

    pub fn define(&mut self, label: String, definition: Definition, file: &Path, diagnostics: &mut Diagnostics) {
        if let Some(first) = self.definitions.get(&label) {
            let message = format!("the footnote `[^{label}]` is already defined on line {}; this one is ignored", first.line);
            diagnostics.warning(Some(file), Some(definition.line), message);
            return;
        }
        self.definitions.insert(label, definition);
    }

    /// The chapter's footnotes in number order, reporting markers without a definition and
    /// definitions nothing refers to.
    pub fn finish(mut self, file: &Path, diagnostics: &mut Diagnostics) -> Vec<Footnote> {
        let mut footnotes = Vec::new();
        for (index, label) in self.labels.iter().enumerate() {
            match self.definitions.remove(label) {
                Some(definition) => footnotes.push(Footnote {
                    label: label.clone(),
                    number: index as u32 + 1,
                    html: definition.html,
                    match_text: definition.match_text,
                    forced_refs: definition.forced_refs,
                }),
                None => {
                    let line = self.referenced_at.get(label).copied();
                    diagnostics.error(Some(file), line, format!("the footnote `[^{label}]` has no definition")).help =
                        Some(format!("add a line like `[^{label}]: The footnote's text.` anywhere in this chapter"));
                }
            }
        }
        let mut unused: Vec<_> = self.definitions.into_iter().collect();
        unused.sort_by_key(|(_, d)| d.line);
        for (label, definition) in unused {
            diagnostics.warning(Some(file), Some(definition.line), format!("nothing refers to the footnote `[^{label}]`")).help =
                Some(format!("mark where it belongs with `[^{label}]` after a word, or remove it"));
        }
        footnotes
    }
}

#[cfg(test)]
mod tests {
    use crate::assets::Assets;
    use crate::diagnostics::Diagnostics;
    use crate::markdown::chapter::{Item, parse_chapter};

    fn parse(source: &str) -> (Option<crate::markdown::chapter::ParsedChapter>, Diagnostics) {
        let dir = tempfile::tempdir().unwrap();
        let mut assets = Assets::new(dir.path());
        let mut diagnostics = Diagnostics::default();
        let chapter = parse_chapter(&dir.path().join("manuscript/01-ash.md"), source, &mut assets, &mut diagnostics);
        (chapter, diagnostics)
    }

    #[test]
    fn gathers_definitions_out_of_the_text_and_numbers_by_first_reference() {
        let source = "# T\n\n[^bell]: An *uneven* bell.\n\nThe bell[^bell] and the ash[^ash].\n\nAsh again[^ash].\n\n[^ash]: Burnt wood.\n";
        let (chapter, diagnostics) = parse(source);
        assert!(diagnostics.items.is_empty(), "{diagnostics:?}");
        let chapter = chapter.unwrap();
        let blocks: Vec<_> = chapter.items.iter().filter_map(|i| if let Item::Block(b) = i { Some(b) } else { None }).collect();
        assert_eq!(blocks.len(), 2, "definitions aren't in the page flow");
        assert_eq!(blocks[0].footnotes, ["bell", "ash"]);
        assert_eq!((blocks[0].words, blocks[0].match_text.as_str()), (5, "The bell and the ash.\n"));
        let numbered: Vec<_> = chapter.footnotes.iter().map(|f| (f.number, f.label.as_str(), f.html.as_str())).collect();
        assert_eq!(numbered, [(1, "bell", "<p>An <em>uneven</em> bell.</p>"), (2, "ash", "<p>Burnt wood.</p>")]);
    }

    #[test]
    fn reports_missing_unused_and_repeated_definitions() {
        let (_, diagnostics) = parse("# T\n\nA[^gone] b.\n\n[^spare]: Never used.\n\n[^twice]: One.\n\n[^twice]: Two.\n\nC[^twice].\n");
        let found: Vec<_> = diagnostics.items.iter().map(|d| (d.line, d.message.as_str())).collect();
        assert_eq!(found, [
            (Some(9), "the footnote `[^twice]` is already defined on line 7; this one is ignored"),
            (Some(3), "the footnote `[^gone]` has no definition"),
            (Some(5), "nothing refers to the footnote `[^spare]`"),
        ]);
    }
}
