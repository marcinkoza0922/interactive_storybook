//! `references.toml` (or `references/*.toml`): spoiler-gated entries, and matching their
//! names and aliases in the text of each page.

use crate::assets::{AssetKind, Assets};
use crate::bundle::{ImageRef, Reference, ReferenceSection};
use crate::diagnostics::{Diagnostics, line_of};
use crate::markdown::render;
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use toml::Spanned;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReferencesFile {
    #[serde(default)]
    reference: Vec<ReferenceSource>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReferenceSource {
    id: Spanned<String>,
    /// Names and aliases that mark a mention, from the first chapter on.
    #[serde(default, rename = "match")]
    matches: Vec<String>,
    #[serde(default)]
    section: Vec<SectionSource>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SectionSource {
    from: Spanned<String>,
    mode: Option<Spanned<String>>,
    title: Option<String>,
    text: String,
    image: Option<Spanned<String>>,
    image_alt: Option<String>,
    /// Aliases that only apply from this section's chapter on.
    #[serde(default, rename = "match")]
    matches: Vec<String>,
}

struct Alias {
    text: String,
    from_chapter: usize,
    file: PathBuf,
    line: usize,
}

struct ReferenceAliases {
    id: String,
    aliases: Vec<Alias>,
}

/// Finds which references each page mentions.
pub struct Matcher {
    references: Vec<ReferenceAliases>,
    used: HashMap<String, bool>,
}

/// Load every references file in the project.
pub fn load(root: &Path, chapter_ids: &[String], assets: &mut Assets, diagnostics: &mut Diagnostics) -> (Vec<Reference>, Matcher) {
    let order: HashMap<&str, usize> = chapter_ids.iter().enumerate().map(|(i, id)| (id.as_str(), i)).collect();
    let mut references = Vec::new();
    let mut matcher = Matcher { references: Vec::new(), used: HashMap::new() };
    let mut seen: HashMap<String, PathBuf> = HashMap::new();

    for file in reference_files(root) {
        let Some((text, parsed)) = read_file(&file, diagnostics) else { continue };
        let mut loader = FileLoader { file: &file, text: &text, chapter_ids, order: &order, assets: &mut *assets, diagnostics: &mut *diagnostics };
        for source in parsed.reference {
            let id_line = loader.line(source.id.span());
            if seen.insert(source.id.get_ref().clone(), file.clone()).is_some() {
                let message = format!("there's already a reference with the ID `{}`", source.id.get_ref());
                loader.diagnostics.error(Some(&file), Some(id_line), message);
                continue;
            }
            if let Some((reference, aliases)) = loader.reference(source, id_line) {
                matcher.references.push(aliases);
                references.push(reference);
            }
        }
    }

    check_overlaps(&matcher, diagnostics);
    (references, matcher)
}

/// references.toml, then everything in references/, in file-name order.
fn reference_files(root: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = Vec::new();
    let single = root.join("references.toml");
    if single.is_file() {
        files.push(single);
    }
    if let Ok(entries) = std::fs::read_dir(root.join("references")) {
        let mut more: Vec<_> = entries.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.extension().is_some_and(|e| e == "toml")).collect();
        more.sort();
        files.extend(more);
    }
    files
}

fn read_file(file: &Path, diagnostics: &mut Diagnostics) -> Option<(String, ReferencesFile)> {
    let text = match std::fs::read_to_string(file) {
        Ok(text) => text,
        Err(error) => {
            diagnostics.error(Some(file), None, format!("couldn't read the file: {error}"));
            return None;
        }
    };
    match toml::from_str(&text) {
        Ok(parsed) => Some((text, parsed)),
        Err(error) => {
            diagnostics.error(Some(file), error.span().map(|s| line_of(&text, s.start)), error.message().to_string());
            None
        }
    }
}

/// One references file being loaded.
struct FileLoader<'a> {
    file: &'a Path,
    text: &'a str,
    chapter_ids: &'a [String],
    order: &'a HashMap<&'a str, usize>,
    assets: &'a mut Assets,
    diagnostics: &'a mut Diagnostics,
}

/// A loaded section, with the chapter it starts at and the aliases it adds.
struct LoadedSection {
    chapter: usize,
    line: usize,
    aliases: Vec<Alias>,
    section: ReferenceSection,
}

impl FileLoader<'_> {
    fn line(&self, span: std::ops::Range<usize>) -> usize {
        line_of(self.text, span.start)
    }

    fn alias(&self, text: &str, from_chapter: usize, line: usize) -> Alias {
        Alias { text: text.to_string(), from_chapter, file: self.file.to_path_buf(), line }
    }

    fn reference(&mut self, source: ReferenceSource, id_line: usize) -> Option<(Reference, ReferenceAliases)> {
        let id = source.id.into_inner();
        if source.section.is_empty() {
            self.diagnostics.error(Some(self.file), Some(id_line), format!("the reference `{id}` has no sections")).help =
                Some("add a [[reference.section]] with `from`, `title` and `text`".into());
            return None;
        }
        let mut aliases: Vec<Alias> = source.matches.iter().map(|m| self.alias(m, 0, id_line)).collect();
        let mut sections: Vec<LoadedSection> = source.section.into_iter().filter_map(|s| self.section(s)).collect();
        aliases.extend(sections.iter_mut().flat_map(|s| s.aliases.drain(..)));

        // Sections apply in chapter order, whatever order they're written in.
        sections.sort_by_key(|s| s.chapter);
        if let Some(first) = sections.first()
            && first.section.title.is_none()
        {
            self.diagnostics.error(Some(self.file), Some(first.line), format!("the first section of `{id}` needs a `title`")).help =
                Some("it's the name shown until a later section changes it".into());
        }
        if aliases.is_empty() {
            self.diagnostics.warning(Some(self.file), Some(id_line), format!("the reference `{id}` has no `match` names")).help =
                Some("without them it only appears where the text uses :ref[…]{id}".into());
        }
        let sections = sections.into_iter().map(|s| s.section).collect();
        Some((Reference { id: id.clone(), sections }, ReferenceAliases { id, aliases }))
    }

    fn section(&mut self, section: SectionSource) -> Option<LoadedSection> {
        let from = section.from.get_ref().clone();
        let from_line = self.line(section.from.span());
        let Some(&chapter) = self.order.get(from.as_str()) else {
            self.diagnostics.error(Some(self.file), Some(from_line), format!("there's no chapter with the ID `{from}`")).help =
                Some(format!("chapter IDs are: {}", self.chapter_ids.join(", ")));
            return None;
        };

        let mode = section.mode.as_ref().map(|m| m.get_ref().clone());
        if let Some(mode) = &mode
            && mode != "replace"
            && mode != "append"
        {
            let line = self.line(section.mode.as_ref().unwrap().span());
            self.diagnostics.error(Some(self.file), Some(line), format!("`mode` must be \"replace\" or \"append\", not \"{mode}\""));
        }
        let image = section.image.as_ref().and_then(|image| self.image(image, section.image_alt.as_deref()));
        let aliases = section.matches.iter().map(|m| self.alias(m, chapter, from_line)).collect();
        let html = render_markdown(&section.text);
        Some(LoadedSection { chapter, line: from_line, aliases, section: ReferenceSection { from, mode, title: section.title, html, image } })
    }

    fn image(&mut self, image: &Spanned<String>, alt: Option<&str>) -> Option<ImageRef> {
        let line = self.line(image.span());
        match self.assets.resolve(image.get_ref(), self.file, AssetKind::Image) {
            Ok(src) => {
                let alt = alt.unwrap_or_default().to_string();
                if alt.trim().is_empty() {
                    self.diagnostics.warning(Some(self.file), Some(line), "the image has no alt text").help =
                        Some("add `image_alt = \"…\"` describing it for screen readers".into());
                }
                Some(ImageRef { src, alt })
            }
            Err(message) => {
                self.diagnostics.error(Some(self.file), Some(line), message);
                None
            }
        }
    }
}

fn check_overlaps(matcher: &Matcher, diagnostics: &mut Diagnostics) {
    let mut owners: BTreeMap<String, Vec<&str>> = BTreeMap::new();
    for reference in &matcher.references {
        for alias in &reference.aliases {
            owners.entry(normalize(&alias.text)).or_default().push(&reference.id);
        }
    }
    for (alias, ids) in owners {
        let mut unique = ids.clone();
        unique.dedup();
        if unique.len() > 1 {
            diagnostics.warning(None, None, format!("`{alias}` matches more than one reference: {}", unique.join(", "))).help =
                Some("every mention will list all of them; use :ref[…]{id} or :noref[…] where that's wrong".into());
        }
    }
}

fn render_markdown(markdown: &str) -> String {
    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, pulldown_cmark::Parser::new_ext(markdown, render::options()));
    html.trim_end().to_string()
}

/// Treat typographic and straight quotes alike, since the text gets smart punctuation.
fn normalize(text: &str) -> String {
    text.replace(['’', '‘'], "'").replace(['“', '”'], "\"")
}

/// The first whole-word occurrence of `needle` in `haystack`.
fn find_word(haystack: &str, needle: &str) -> Option<usize> {
    if needle.is_empty() {
        return None;
    }
    let mut from = 0;
    while let Some(offset) = haystack[from..].find(needle) {
        let start = from + offset;
        let end = start + needle.len();
        let before = haystack[..start].chars().next_back();
        let after = haystack[end..].chars().next();
        if !before.is_some_and(char::is_alphanumeric) && !after.is_some_and(char::is_alphanumeric) {
            return Some(start);
        }
        from = start + needle.chars().next().map_or(1, char::len_utf8);
    }
    None
}

impl Matcher {
    /// IDs of the references mentioned on a page, in order of first mention.
    pub fn page_references(&mut self, chapter: usize, texts: &[(String, Vec<String>)], file: &Path, diagnostics: &mut Diagnostics) -> Vec<String> {
        let mut combined = String::new();
        let mut found: Vec<(usize, String)> = Vec::new();

        for (text, forced) in texts {
            let start = combined.len();
            combined.push_str(&normalize(text));
            combined.push('\n');
            for id in forced {
                if self.references.iter().any(|r| &r.id == id) {
                    found.push((start, id.clone()));
                } else {
                    diagnostics.error(Some(file), None, format!("`:ref` points to `{id}`, which isn't a reference"));
                }
            }
        }

        for reference in &self.references {
            let mut first: Option<usize> = None;
            for alias in reference.aliases.iter().filter(|a| a.from_chapter <= chapter) {
                if let Some(position) = find_word(&combined, &normalize(&alias.text)) {
                    self.used.insert(alias_key(&reference.id, &alias.text), true);
                    first = Some(first.map_or(position, |p| p.min(position)));
                }
            }
            if let Some(position) = first {
                found.push((position, reference.id.clone()));
            }
        }

        found.sort();
        let mut ids: Vec<String> = Vec::new();
        for (_, id) in found {
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
        ids
    }

    /// Warn about aliases that never matched anything (usually a typo).
    pub fn report_unused(&self, diagnostics: &mut Diagnostics) {
        for reference in &self.references {
            for alias in &reference.aliases {
                if !self.used.contains_key(&alias_key(&reference.id, &alias.text)) {
                    diagnostics.warning(Some(&alias.file), Some(alias.line), format!("`{}` (for `{}`) never appears in the text", alias.text, reference.id)).help =
                        Some("matching is case-sensitive and whole-word; aliases from a section only match from its chapter on".into());
                }
            }
        }
    }
}

fn alias_key(id: &str, alias: &str) -> String {
    format!("{id}\u{0}{alias}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn setup(toml: &str) -> (tempfile::TempDir, Vec<Reference>, Matcher, Diagnostics) {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("references.toml"), toml).unwrap();
        let mut assets = Assets::new(dir.path());
        let mut diagnostics = Diagnostics::default();
        let chapters = vec!["one".to_string(), "two".to_string()];
        let (references, matcher) = load(dir.path(), &chapters, &mut assets, &mut diagnostics);
        (dir, references, matcher, diagnostics)
    }

    const WITCH: &str = r#"
[[reference]]
id = "elara"
match = ["the Witch of Varn"]

[[reference.section]]
from = "two"
title = "Elara"
text = "The *exiled* queen."
match = ["Elara"]

[[reference.section]]
from = "one"
title = "The Witch of Varn"
text = "A healer."

[[reference]]
id = "wren"
match = ["Wren"]

[[reference.section]]
from = "one"
title = "Wren"
text = "Arrived late."
"#;

    fn texts(lines: &[&str]) -> Vec<(String, Vec<String>)> {
        lines.iter().map(|l| (l.to_string(), vec![])).collect()
    }

    #[test]
    fn loads_sections_in_chapter_order() {
        let (_, references, _, diagnostics) = setup(WITCH);
        assert!(diagnostics.items.is_empty(), "{diagnostics:?}");
        let froms: Vec<_> = references[0].sections.iter().map(|s| s.from.as_str()).collect();
        assert_eq!(froms, ["one", "two"]);
        assert_eq!(references[0].sections[1].html, "<p>The <em>exiled</em> queen.</p>");
    }

    #[test]
    fn matches_whole_words_in_order_and_gates_section_aliases() {
        let (_, _, mut matcher, mut diagnostics) = setup(WITCH);
        let page = texts(&["Wren’s coat was wet.", "Elara met the Witch of Varn."]);
        // In chapter one, "Elara" isn't an alias yet; "Wren’s" still matches "Wren".
        assert_eq!(matcher.page_references(0, &page, Path::new("x"), &mut diagnostics), ["wren", "elara"]);
        assert_eq!(matcher.page_references(1, &texts(&["Elara."]), Path::new("x"), &mut diagnostics), ["elara"]);
        assert!(matcher.page_references(0, &texts(&["Wrenfield"]), Path::new("x"), &mut diagnostics).is_empty());
    }

    #[test]
    fn honours_forced_references_and_reports_unknown_ones() {
        let (_, _, mut matcher, mut diagnostics) = setup(WITCH);
        let page = vec![("the old woman".to_string(), vec!["elara".to_string(), "nobody".to_string()])];
        assert_eq!(matcher.page_references(0, &page, Path::new("x"), &mut diagnostics), ["elara"]);
        assert!(diagnostics.items[0].message.contains("`nobody`"));
    }

    #[test]
    fn warns_about_unused_aliases() {
        let (_, _, mut matcher, mut diagnostics) = setup(WITCH);
        matcher.page_references(0, &texts(&["Wren"]), Path::new("x"), &mut diagnostics);
        matcher.report_unused(&mut diagnostics);
        let messages: Vec<_> = diagnostics.items.iter().map(|d| d.message.clone()).collect();
        assert!(messages.iter().any(|m| m.contains("`the Witch of Varn`")));
        assert!(messages.iter().any(|m| m.contains("`Elara`")));
        assert!(!messages.iter().any(|m| m.contains("`Wren`")));
    }

    #[test]
    fn reports_bad_chapters_and_missing_titles_with_lines() {
        let (_, _, _, diagnostics) = setup("[[reference]]\nid = \"x\"\nmatch = [\"X\"]\n\n[[reference.section]]\nfrom = \"nine\"\ntext = \"?\"\n\n[[reference.section]]\nfrom = \"one\"\ntext = \"No title.\"\n");
        let found: Vec<_> = diagnostics.items.iter().map(|d| (d.line, d.message.clone())).collect();
        assert!(found.iter().any(|(l, m)| *l == Some(6) && m.contains("no chapter with the ID `nine`")), "{found:?}");
        assert!(found.iter().any(|(l, m)| *l == Some(10) && m.contains("needs a `title`")), "{found:?}");
    }

    #[test]
    fn reports_unknown_keys() {
        let (_, _, _, diagnostics) = setup("[[reference]]\nid = \"x\"\nmatches = [\"X\"]\n");
        assert!(diagnostics.items[0].message.contains("matches"), "{:?}", diagnostics.items);
        assert_eq!(diagnostics.items[0].line, Some(3));
    }
}
