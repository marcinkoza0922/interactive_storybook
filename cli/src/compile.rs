//! Compiling a book project into a bundle: the whole pipeline from files to `book.json`.

use crate::assets::Assets;
use crate::bundle::{self, AudioConfig, BookMeta, Bundle, ContentsEntry, Cue, IllustrationConfig};
use crate::diagnostics::{Diagnostics, line_of};
use crate::markdown::chapter::{Item, PageLimits, ParsedChapter, parse_chapter};
use crate::{paginate, references, theme};
use serde::Deserialize;
use std::collections::{BTreeSet, HashSet};
use std::path::{Path, PathBuf};
use toml::Spanned;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BookToml {
    id: Option<String>,
    title: String,
    author: String,
    language: Option<String>,
    pagination: Option<PageLimits>,
    audio: Option<AudioToml>,
    illustrations: Option<IllustrationsToml>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AudioToml {
    /// Seconds before a page's music returns after going back to it.
    restore_delay: f64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct IllustrationsToml {
    /// Seconds a new illustration shows on narrow screens before the text.
    linger: f64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ContentsToml {
    #[serde(default)]
    entry: Vec<ContentsEntryToml>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ContentsEntryToml {
    chapter: Option<Spanned<String>>,
    heading: Option<String>,
    title: Option<String>,
    #[serde(default)]
    hidden: bool,
}

/// Styles every runtime provides, which themes may restyle.
const BUILT_IN_STYLES: &[&str] = &["handwriting", "whisper"];

pub struct Compiled {
    pub bundle: Bundle,
    pub assets: Assets,
    pub diagnostics: Diagnostics,
}

pub fn chapter_files(root: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(root.join("manuscript"))
        .map(|entries| entries.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.extension().is_some_and(|e| e == "md")).collect())
        .unwrap_or_default();
    files.sort();
    files
}

/// Compile the project at `root`. Always returns diagnostics; the bundle is only usable if
/// there are no errors.
pub fn compile(root: &Path) -> Compiled {
    // Absolute paths, so diagnostics can be shown relative to the project.
    let root = &root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let mut diagnostics = Diagnostics::default();
    let mut assets = Assets::new(root);

    let book_file = root.join("book.toml");
    let book = read_toml::<BookToml>(&book_file, &mut diagnostics);
    let book = book.unwrap_or_else(|| BookToml {
        id: None,
        title: "Untitled".into(),
        author: String::new(),
        language: None,
        pagination: None,
        audio: None,
        illustrations: None,
    });

    // Chapters, in file-name order.
    let files = chapter_files(root);
    if files.is_empty() {
        diagnostics.error(Some(&root.join("manuscript")), None, "there are no chapters").help =
            Some("add Markdown files like manuscript/01-first-chapter.md".into());
    }
    let mut chapters: Vec<(PathBuf, ParsedChapter)> = Vec::new();
    let mut ids = HashSet::new();
    for file in files {
        let source = match std::fs::read_to_string(&file) {
            Ok(source) => source,
            Err(error) => {
                diagnostics.error(Some(&file), None, format!("couldn't read the file: {error}"));
                continue;
            }
        };
        if let Some(chapter) = parse_chapter(&file, &source, &mut assets, &mut diagnostics) {
            if !ids.insert(chapter.id.clone()) {
                diagnostics.error(Some(&file), None, format!("another chapter already has the ID `{}`", chapter.id)).help =
                    Some("set a different `id` in this chapter's front matter".into());
                continue;
            }
            chapters.push((file, chapter));
        }
    }
    let chapter_ids: Vec<String> = chapters.iter().map(|(_, c)| c.id.clone()).collect();

    let theme = theme::load(root, &chapter_ids, &mut assets, &mut diagnostics);
    let mut known_styles: BTreeSet<String> = BUILT_IN_STYLES.iter().map(|s| s.to_string()).collect();
    if let Some((_, styles)) = &theme {
        known_styles.extend(styles.iter().cloned());
    }
    check_styles(&chapters, &known_styles, &mut diagnostics);
    resolve_audio(&mut chapters, &mut diagnostics);

    let (references, mut matcher) = references::load(root, &chapter_ids, &mut assets, &mut diagnostics);

    let default_limits = book.pagination.unwrap_or_default();
    let mut bundle_chapters = Vec::new();
    for (index, (file, chapter)) in chapters.into_iter().enumerate() {
        let limits = chapter.limits.or(default_limits);
        let (id, title, content_hash, header_image) = (chapter.id.clone(), chapter.title.clone(), chapter.content_hash.clone(), chapter.header_image.clone());
        let pages = paginate::paginate(chapter, limits, &file, &mut diagnostics)
            .into_iter()
            .map(|mut paginated| {
                paginated.page.references = matcher.page_references(index, &paginated.texts, &file, &mut diagnostics);
                paginated.page
            })
            .collect();
        bundle_chapters.push(bundle::Chapter { id, title, content_hash, header_image, pages });
    }
    matcher.report_unused(&mut diagnostics);

    let contents = load_contents(root, &chapter_ids, &mut diagnostics);
    let id = book.id.clone().unwrap_or_else(|| slug(&book.title));

    let bundle = Bundle {
        bundle_schema_version: bundle::SCHEMA_VERSION,
        book: BookMeta { id, title: book.title, author: book.author, language: book.language.unwrap_or_else(|| "en".into()) },
        audio: book.audio.map(|a| AudioConfig { restore_delay_ms: seconds_to_ms(a.restore_delay) }),
        illustrations: book.illustrations.map(|i| IllustrationConfig { linger_ms: seconds_to_ms(i.linger) }),
        chapters: bundle_chapters,
        contents,
        references,
        theme: theme.map(|(theme, _)| theme),
    };
    Compiled { bundle, assets, diagnostics }
}

fn seconds_to_ms(seconds: f64) -> u64 {
    (seconds.max(0.0) * 1000.0).round() as u64
}

fn read_toml<T: serde::de::DeserializeOwned>(file: &Path, diagnostics: &mut Diagnostics) -> Option<T> {
    let text = match std::fs::read_to_string(file) {
        Ok(text) => text,
        Err(_) => {
            diagnostics.error(Some(file), None, "this isn't a book project: book.toml is missing").help =
                Some("run `tome new <folder>` to start one".into());
            return None;
        }
    };
    match toml::from_str(&text) {
        Ok(value) => Some(value),
        Err(error) => {
            diagnostics.error(Some(file), error.span().map(|s| line_of(&text, s.start)), error.message().to_string());
            None
        }
    }
}

pub fn slug(text: &str) -> String {
    let slug: String = text.to_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect();
    let slug = slug.split('-').filter(|s| !s.is_empty()).collect::<Vec<_>>().join("-");
    if slug.is_empty() { "book".into() } else { slug }
}

/// Warn about `:::style{name}` / `:style[…]{name}` with a style no theme defines.
fn check_styles(chapters: &[(PathBuf, ParsedChapter)], known: &BTreeSet<String>, diagnostics: &mut Diagnostics) {
    for (file, chapter) in chapters {
        for item in &chapter.items {
            let Item::Block(block) = item else { continue };
            for part in block.html.split("data-tome-style=\"").skip(1) {
                let name = part.split('"').next().unwrap_or("");
                if !known.contains(name) {
                    let known: Vec<_> = known.iter().map(String::as_str).collect();
                    diagnostics.warning(Some(file), Some(block.line), format!("no style is called `{name}`")).help =
                        Some(format!("define it under [styles.{name}] in theme/theme.toml; known styles are {}", known.join(", ")));
                }
            }
        }
    }
}

/// Follow music and ambience through the whole book in order: expand `::ambient{stop}` into a
/// stop for every playing layer, and warn about stopping what isn't playing.
fn resolve_audio(chapters: &mut [(PathBuf, ParsedChapter)], diagnostics: &mut Diagnostics) {
    let mut music = false;
    let mut ambient: BTreeSet<String> = BTreeSet::new();

    for (file, chapter) in chapters.iter_mut() {
        let items = std::mem::take(&mut chapter.items);
        for item in items {
            let Item::Cue { cue, line } = &item else {
                chapter.items.push(item);
                continue;
            };
            match cue {
                Cue::Music { .. } => music = true,
                Cue::MusicStop { .. } => {
                    if !music {
                        diagnostics.warning(Some(file), Some(*line), "`::music{stop}`: no music is playing here");
                    }
                    music = false;
                }
                Cue::Ambient { id, .. } => {
                    ambient.insert(id.clone());
                }
                Cue::AmbientStop { id, fade_ms, delay_ms } if id == "*" => {
                    if ambient.is_empty() {
                        diagnostics.warning(Some(file), Some(*line), "`::ambient{stop}`: no ambience is playing here");
                    }
                    for playing in std::mem::take(&mut ambient) {
                        chapter.items.push(Item::Cue { cue: Cue::AmbientStop { id: playing, fade_ms: *fade_ms, delay_ms: *delay_ms }, line: *line });
                    }
                    continue;
                }
                Cue::AmbientStop { id, .. } => {
                    if !ambient.remove(id) {
                        let playing: Vec<_> = ambient.iter().map(String::as_str).collect();
                        diagnostics.warning(Some(file), Some(*line), format!("`::ambient{{stop {id}}}`: `{id}` isn't playing here")).help =
                            Some(if playing.is_empty() { "no ambience is playing".into() } else { format!("playing: {}", playing.join(", ")) });
                    }
                }
                Cue::Sfx { .. } => {}
            }
            chapter.items.push(item);
        }
    }
}

fn load_contents(root: &Path, chapter_ids: &[String], diagnostics: &mut Diagnostics) -> Option<Vec<ContentsEntry>> {
    let file = root.join("contents.toml");
    let text = std::fs::read_to_string(&file).ok()?;
    let parsed: ContentsToml = match toml::from_str(&text) {
        Ok(parsed) => parsed,
        Err(error) => {
            diagnostics.error(Some(&file), error.span().map(|s| line_of(&text, s.start)), error.message().to_string());
            return None;
        }
    };

    let mut entries = Vec::new();
    let mut listed = HashSet::new();
    for entry in parsed.entry {
        match (entry.chapter, entry.heading) {
            (Some(chapter), None) => {
                let id = chapter.get_ref().clone();
                if !chapter_ids.contains(&id) {
                    diagnostics.warning(Some(&file), Some(line_of(&text, chapter.span().start)), format!("there's no chapter with the ID `{id}`")).help =
                        Some("contents.toml is out of date; fix it, or run `tome contents --regenerate`".into());
                    continue;
                }
                listed.insert(id.clone());
                if !entry.hidden {
                    entries.push(ContentsEntry::Chapter { id, title: entry.title });
                }
            }
            (None, Some(title)) => entries.push(ContentsEntry::Heading { title }),
            _ => {
                diagnostics.error(Some(&file), None, "each [[entry]] needs either `chapter = \"id\"` or `heading = \"Title\"`");
            }
        }
    }
    for id in chapter_ids.iter().filter(|id| !listed.contains(*id)) {
        diagnostics.warning(Some(&file), None, format!("the chapter `{id}` isn't in contents.toml")).help =
            Some("add it (with `hidden = true` to leave it out of the chapter list), or run `tome contents --regenerate`".into());
    }
    Some(entries)
}

/// A fresh contents.toml listing every chapter, for the author to edit.
pub fn generate_contents(root: &Path) -> String {
    let mut text = String::from(
        "# The chapter list readers see in the menu, in order. Edit freely: rename entries with\n\
         # `title`, leave chapters out with `hidden = true`, and add part headings with\n\
         # [[entry]] heading = \"Part One\". `tome` never overwrites this file.\n",
    );
    let mut scratch = Diagnostics::default();
    let mut assets = Assets::new(root);
    for file in chapter_files(root) {
        let Ok(source) = std::fs::read_to_string(&file) else { continue };
        if let Some(chapter) = parse_chapter(&file, &source, &mut assets, &mut scratch) {
            text.push_str(&format!("\n[[entry]]\nchapter = \"{}\"  # {}\n", chapter.id, chapter.title.replace('\n', " ")));
        }
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::Cue;

    /// The sample book exercises every feature; it must always build cleanly.
    #[test]
    fn compiles_the_sample_book() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples/the-uneven-bell");
        let compiled = compile(&root);
        assert!(compiled.diagnostics.items.is_empty(), "{}", compiled.diagnostics.render(&root));

        let bundle = &compiled.bundle;
        let pages: Vec<usize> = bundle.chapters.iter().map(|c| c.pages.len()).collect();
        assert_eq!(pages, [3, 2, 1]);

        let candle_page = &bundle.chapters[0].pages[1];
        let steps: Vec<_> = candle_page.blocks.iter().map(|b| b.reveal.as_ref().map(|r| (r.step, r.effect.as_str()))).collect();
        assert_eq!(steps, [None, Some((1, "slide")), Some((2, "fade")), Some((3, "fade")), Some((4, "typewriter"))]);
        assert!(matches!(candle_page.blocks[0].cues[..], [Cue::AmbientStop { ref id, fade_ms: Some(3000), .. }] if id == "sea"));
        assert!(candle_page.blocks[4].html.contains(r#"data-tome-style="handwriting""#));

        assert_eq!(bundle.chapters[1].pages[0].illustration, Some(None), "chapter 2 clears the illustration track");
        assert_eq!(bundle.chapters[1].pages[0].references, ["the-keeper", "wren"], "\"the daughter\" matches from chapter 2");
        assert_eq!(bundle.theme.as_ref().unwrap().overrides.len(), 2);
        assert!(compiled.assets.files().count() >= 19);
    }

    #[test]
    fn expands_stopping_all_ambience_and_warns_about_stopping_silence() {
        let dir = tempfile::tempdir().unwrap();
        for file in ["assets/audio/rain.ogg", "assets/audio/wind.ogg"] {
            let path = dir.path().join(file);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, "ogg").unwrap();
        }
        std::fs::write(dir.path().join("book.toml"), "title = \"T\"\nauthor = \"A\"\n").unwrap();
        std::fs::create_dir_all(dir.path().join("manuscript")).unwrap();
        std::fs::write(dir.path().join("manuscript/01-one.md"), "# One\n\n::ambient{rain}\n::ambient{wind}\nA.\n").unwrap();
        std::fs::write(dir.path().join("manuscript/02-two.md"), "# Two\n\n::ambient{stop}\n::music{stop}\nB.\n").unwrap();

        let compiled = compile(dir.path());
        let stops: Vec<_> = compiled.bundle.chapters[1].pages[0].blocks[0]
            .cues
            .iter()
            .filter_map(|c| if let Cue::AmbientStop { id, .. } = c { Some(id.as_str()) } else { None })
            .collect();
        assert_eq!(stops, ["rain", "wind"], "stopping everything carries across chapters");
        let messages: Vec<_> = compiled.diagnostics.items.iter().map(|d| d.message.as_str()).collect();
        assert_eq!(messages, ["`::music{stop}`: no music is playing here"]);
    }
}
