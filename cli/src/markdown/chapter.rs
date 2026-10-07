//! Reading one chapter file: front matter, then Markdown with directives, into a flat list of
//! blocks, cues, page breaks and illustration changes, ready for pagination.

use super::directive::{self, Attrs, DirectiveLine, css_name, number, rest_attributes};
use super::render::{self, RenderContext, add_attributes};
use crate::assets::{AssetKind, Assets, short_hash};
use crate::bundle::{Cue, ImageRef};
use crate::diagnostics::Diagnostics;
use pulldown_cmark::{Event, Parser};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Default, Clone, Copy, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PageLimits {
    pub max_words: Option<usize>,
    pub max_characters: Option<usize>,
    /// The author's line breaks: paragraphs and hard breaks.
    pub max_lines: Option<usize>,
}

impl PageLimits {
    /// These limits, with any unset ones taken from `defaults`.
    pub fn or(self, defaults: PageLimits) -> PageLimits {
        PageLimits {
            max_words: self.max_words.or(defaults.max_words),
            max_characters: self.max_characters.or(defaults.max_characters),
            max_lines: self.max_lines.or(defaults.max_lines),
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct FrontMatter {
    id: Option<String>,
    title: Option<String>,
    header_image: Option<String>,
    header_image_alt: Option<String>,
    pagination: Option<PageLimits>,
    /// A named paper from the theme for every page of the chapter.
    paper: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RevealSpec {
    pub effect: String,
    pub duration_ms: Option<u64>,
    pub delay_ms: Option<u64>,
    pub easing: Option<String>,
    /// Blocks in the same group appear in the same step; steps are numbered per page later.
    pub group: usize,
}

#[derive(Debug, Clone)]
pub struct ParsedBlock {
    pub html: String,
    pub words: usize,
    pub characters: usize,
    pub lines: usize,
    /// Text for reference matching, with `:noref` text removed.
    pub match_text: String,
    pub forced_refs: Vec<String>,
    pub reveal: Option<RevealSpec>,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub enum Item {
    Block(ParsedBlock),
    Cue { cue: Cue, line: usize },
    PageBreak,
    /// Some(None) clears the track.
    Illustration { image: Option<ImageRef>, line: usize },
    /// A named paper for the page holding the next block.
    Paper { name: String, line: usize },
}

#[derive(Debug)]
pub struct ParsedChapter {
    pub id: String,
    pub title: String,
    pub header_image: Option<ImageRef>,
    pub limits: PageLimits,
    /// The paper for every page without its own, from the front matter.
    pub paper: Option<(String, usize)>,
    pub items: Vec<Item>,
    pub content_hash: String,
}

enum Container {
    Reveal { spec: RevealSpec, together: bool, rest: Option<String> },
    Style(String),
    Fx(String),
}

struct ChapterParser<'a> {
    file: &'a Path,
    assets: &'a mut Assets,
    diagnostics: &'a mut Diagnostics,
    stack: Vec<(Container, usize)>,
    items: Vec<Item>,
    next_group: usize,
}

const LEAF_DIRECTIVES: &str = "music, ambient, sfx, voice, illustration, paper, pagebreak";
const CONTAINER_DIRECTIVES: &str = "reveal, style, fx";
pub const ENTRANCE_EFFECTS: &[&str] = &["fade", "slide", "typewriter"];

pub fn parse_chapter(file: &Path, source: &str, assets: &mut Assets, diagnostics: &mut Diagnostics) -> Option<ParsedChapter> {
    let (front, body, body_line) = split_front_matter(file, source, diagnostics)?;
    let mut parser = ChapterParser { file, assets, diagnostics, stack: Vec::new(), items: Vec::new(), next_group: 0 };

    parser.body(body, body_line);
    parser.report_unclosed();

    let ChapterParser { mut items, assets, diagnostics, .. } = parser;

    // A leading `# Heading` is the chapter title, unless the front matter gives one.
    let mut title = front.title.clone();
    if title.is_none()
        && let Some(Item::Block(first)) = items.first()
        && let Some(heading) = h1_text(first)
    {
        title = Some(heading);
        items.remove(0);
    }

    let id = match &front.id {
        Some(id) => id.clone(),
        None => id_from_filename(file),
    };
    if css_name("id", &id).is_err() {
        diagnostics
            .error(Some(file), None, format!("the chapter ID `{id}` must be lowercase letters, digits and dashes"))
            .help = Some("set `id = \"the-harbour\"` in the front matter, or rename the file".into());
    }
    let title = title.unwrap_or_else(|| {
        diagnostics.warning(Some(file), None, "the chapter has no title").help =
            Some("start it with `# Title`, or set `title` in the front matter".into());
        id.clone()
    });

    if !items.iter().any(|i| matches!(i, Item::Block(_))) {
        diagnostics.error(Some(file), None, "the chapter has no text");
        return None;
    }

    let header_image = front.header_image.as_deref().and_then(|src| match assets.resolve(src, file, AssetKind::Image) {
        Ok(src) => Some(ImageRef { src, alt: front.header_image_alt.clone().unwrap_or_default() }),
        Err(message) => {
            diagnostics.error(Some(file), None, message);
            None
        }
    });

    Some(ParsedChapter {
        id,
        title,
        header_image,
        limits: front.pagination.unwrap_or_default(),
        paper: front.paper.map(|name| (name, 1)),
        items,
        content_hash: short_hash(source.as_bytes()),
    })
}

fn h1_text(block: &ParsedBlock) -> Option<String> {
    block.html.starts_with("<h1").then(|| block.match_text.trim().to_string())
}

/// `01-the-harbour.md` → `the-harbour`.
pub fn id_from_filename(file: &Path) -> String {
    let stem = file.file_stem().and_then(|s| s.to_str()).unwrap_or("chapter");
    let without_number = stem.trim_start_matches(|c: char| c.is_ascii_digit() || c == '-' || c == '_' || c == ' ' || c == '.');
    let name = if without_number.is_empty() { stem } else { without_number };
    let slug: String = name.to_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect();
    slug.split('-').filter(|s| !s.is_empty()).collect::<Vec<_>>().join("-")
}

fn split_front_matter<'a>(file: &Path, source: &'a str, diagnostics: &mut Diagnostics) -> Option<(FrontMatter, &'a str, usize)> {
    let Some(rest) = source.strip_prefix("+++\n").or_else(|| source.strip_prefix("+++\r\n")) else {
        return Some((FrontMatter::default(), source, 1));
    };
    let Some(end) = rest.find("\n+++") else {
        diagnostics.error(Some(file), Some(1), "the front matter starting with `+++` is never closed");
        return None;
    };
    let toml_text = &rest[..end];
    let after = &rest[end + 4..];
    let body = after.strip_prefix("\r\n").or_else(|| after.strip_prefix('\n')).unwrap_or(after);
    let body_line = toml_text.lines().count() + 3;

    match toml::from_str::<FrontMatter>(toml_text) {
        Ok(front) => Some((front, body, body_line)),
        Err(error) => {
            let line = error.span().map(|span| 1 + crate::diagnostics::line_of(toml_text, span.start));
            diagnostics.error(Some(file), line, format!("front matter: {}", error.message()));
            Some((FrontMatter::default(), body, body_line))
        }
    }
}

impl ChapterParser<'_> {
    /// Split the body into Markdown segments and directives.
    fn body(&mut self, body: &str, body_line: usize) {
        let mut segment = String::new();
        let mut segment_line = body_line;
        let mut fence: Option<String> = None;

        for (index, line) in body.lines().enumerate() {
            let line_number = body_line + index;
            let trimmed = line.trim_start();

            // Directives inside fenced code blocks are just text.
            if let Some(open) = &fence {
                if trimmed.starts_with(open.as_str()) {
                    fence = None;
                }
            } else if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
                fence = Some(trimmed[..3].to_string());
            } else {
                match directive::parse_line(line) {
                    Ok(Some(directive)) => {
                        self.flush(&segment, segment_line);
                        segment.clear();
                        segment_line = line_number + 1;
                        self.directive(directive, line_number);
                        continue;
                    }
                    Ok(None) => {}
                    Err(message) => {
                        self.diagnostics.error(Some(self.file), Some(line_number), message);
                        segment_line = line_number + 1;
                        self.flush(&segment, segment_line);
                        segment.clear();
                        continue;
                    }
                }
            }
            if segment.is_empty() {
                segment_line = line_number;
            }
            segment.push_str(line);
            segment.push('\n');
        }
        self.flush(&segment, segment_line);
    }

    fn report_unclosed(&mut self) {
        for (container, line) in &self.stack {
            let name = match container {
                Container::Reveal { .. } => "reveal",
                Container::Style(_) => "style",
                Container::Fx(_) => "fx",
            };
            self.diagnostics.error(Some(self.file), Some(*line), format!("`:::{name}` is never closed")).help = Some("end it with a line containing just `:::`".into());
        }
    }

    fn directive(&mut self, directive: DirectiveLine, line: usize) {
        match directive {
            DirectiveLine::Close => {
                if self.stack.pop().is_none() {
                    self.diagnostics.warning(Some(self.file), Some(line), "`:::` closes nothing");
                }
            }
            DirectiveLine::Leaf { name, attrs } => self.leaf(&name, &attrs, line),
            DirectiveLine::Open { name, attrs } => {
                if let Some(container) = self.container(&name, &attrs, line) {
                    self.stack.push((container, line));
                }
            }
        }
    }

    fn leaf(&mut self, name: &str, attrs: &Attrs, line: usize) {
        match name {
            "pagebreak" => self.items.push(Item::PageBreak),
            "paper" => {
                self.check_options(name, attrs, &[], line);
                match attrs.main(&[]) {
                    Some(paper) => self.items.push(Item::Paper { name: paper.to_string(), line }),
                    None => self.error(line, "`::paper` needs the name of a paper from the theme, like `::paper{letter}`"),
                }
            }
            "music" | "ambient" | "sfx" | "voice" => {
                if let Some(cue) = self.cue(name, attrs, line) {
                    self.items.push(Item::Cue { cue, line });
                }
            }
            "illustration" => {
                self.check_options(name, attrs, &["alt"], line);
                let image = if attrs.has_word("none") {
                    None
                } else {
                    let Some(src) = attrs.main(&[]) else {
                        self.error(line, "`::illustration` needs an image, like `::illustration{bridge alt=\"…\"}`, or `none`");
                        return;
                    };
                    let src = match self.assets.resolve(src, self.file, AssetKind::Image) {
                        Ok(src) => src,
                        Err(message) => return self.error(line, message),
                    };
                    let alt = attrs.get("alt").unwrap_or("").to_string();
                    if alt.trim().is_empty() {
                        self.diagnostics.warning(Some(self.file), Some(line), "the illustration has no alt text").help =
                            Some("describe it for screen readers: `alt=\"A bell above an empty chair\"`".into());
                    }
                    Some(ImageRef { src, alt })
                };
                self.items.push(Item::Illustration { image, line });
            }
            other => self.unknown(other, LEAF_DIRECTIVES, line),
        }
    }

    fn cue(&mut self, name: &str, attrs: &Attrs, line: usize) -> Option<Cue> {
        let stop = attrs.has_word("stop");
        let allowed: &[&str] = match (name, stop) {
            ("sfx", _) => &["volume", "delay", "caption"],
            ("voice", _) => &["volume", "delay", "timing"],
            (_, true) => &["fade", "delay", "id"],
            _ => &["volume", "fade", "delay", "id", "caption"],
        };
        self.check_options(name, attrs, allowed, line);
        let fade_ms = self.seconds(attrs, "fade", line);
        let delay_ms = self.seconds(attrs, "delay", line);
        let volume = self.volume(attrs, line);

        if stop && name == "music" {
            return Some(Cue::MusicStop { fade_ms, delay_ms });
        }
        if stop && name == "ambient" {
            // Without a name, every playing layer stops; the compiler expands "*" later.
            let id = attrs.get("id").or(attrs.main(&["stop"])).unwrap_or("*").to_string();
            return Some(Cue::AmbientStop { id, fade_ms, delay_ms });
        }
        if stop {
            self.error(line, match name {
                "voice" => "narration stops by itself when the page turns; there's no `::voice{stop}`",
                _ => "sound effects play once and can't be stopped",
            });
            return None;
        }

        let Some(sound) = attrs.main(&[]) else {
            self.error(line, format!("`::{name}` needs a sound, like `::{name}{{harbour}}`"));
            return None;
        };
        let src = match self.assets.resolve(sound, self.file, AssetKind::Audio) {
            Ok(src) => src,
            Err(message) => {
                self.error(line, message);
                return None;
            }
        };
        let caption = if name == "voice" { None } else { self.caption(name, attrs, line) };
        Some(match name {
            "music" => Cue::Music { src, volume, fade_ms, delay_ms, caption },
            "ambient" => {
                let id = attrs.get("id").unwrap_or(sound).to_string();
                let id = id.rsplit('/').next().unwrap_or(&id).split('.').next().unwrap_or(&id).to_string();
                Cue::Ambient { id, src, volume, fade_ms, delay_ms, caption }
            }
            "voice" => {
                let timing = self.voice_timing(sound, attrs.get("timing"), line);
                Cue::Voice { src, volume, delay_ms, words: None, timing }
            }
            _ => Cue::Sfx { src, volume, delay_ms, caption },
        })
    }

    /// A sound's caption for readers who can't hear it; a warning without one, like missing alt text.
    fn caption(&mut self, name: &str, attrs: &Attrs, line: usize) -> Option<String> {
        let caption = attrs.get("caption").map(str::trim).filter(|c| !c.is_empty());
        if caption.is_none() {
            self.diagnostics.warning(Some(self.file), Some(line), format!("the `::{name}` cue has no caption")).help =
                Some("describe it for readers who can't hear it: `caption=\"a bell tolls, uneven\"`".into());
        }
        caption.map(str::to_string)
    }

    /// Word timings for a line of narration: the file named with `timing=` (relative to the
    /// recording), or a .vtt, .srt or .json file beside the recording with the same name.
    fn voice_timing(&mut self, sound: &str, named: Option<&str>, line: usize) -> Option<Vec<crate::timing::Segment>> {
        let recording = self.assets.source(sound, self.file, AssetKind::Audio).ok()?;
        let folder = recording.parent()?.to_path_buf();
        let file = match named {
            Some(name) if name.starts_with("./") || name.starts_with("../") => self.file.parent()?.join(name),
            Some(name) => folder.join(name),
            None => ["vtt", "srt", "json"].iter().map(|ext| recording.with_extension(ext)).find(|p| p.is_file())?,
        };
        let Ok(text) = std::fs::read_to_string(&file) else {
            self.error(line, format!("can't find the timing file `{}`", named.unwrap_or_default()));
            return None;
        };
        let extension = file.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
        match crate::timing::parse(&text, &extension) {
            Ok(segments) => Some(segments),
            Err(message) => {
                let shown = file.file_name().and_then(|n| n.to_str()).unwrap_or("timing file");
                self.error(line, format!("{shown}: {message}"));
                None
            }
        }
    }

    fn container(&mut self, name: &str, attrs: &Attrs, line: usize) -> Option<Container> {
        match name {
            "reveal" => self.reveal(attrs, line),
            "style" => match attrs.main(&[]).map(|s| css_name("style", s)) {
                Some(Ok(style)) => Some(Container::Style(style.to_string())),
                Some(Err(message)) => {
                    self.error(line, message);
                    None
                }
                None => {
                    self.error(line, "`:::style` needs a style name, like `:::style{handwriting}`");
                    None
                }
            },
            "fx" => {
                let Some(effect) = attrs.main(&[]).or(attrs.get("effect")) else {
                    self.error(line, "`:::fx` needs an effect, like `:::fx{wave}`");
                    return None;
                };
                match rest_attributes(attrs, effect) {
                    Ok(html) => Some(Container::Fx(html)),
                    Err(message) => {
                        self.error(line, message);
                        None
                    }
                }
            }
            other => {
                self.unknown(other, CONTAINER_DIRECTIVES, line);
                None
            }
        }
    }

    fn reveal(&mut self, attrs: &Attrs, line: usize) -> Option<Container> {
        self.check_options("reveal", attrs, &["effect", "duration", "delay", "easing", "rest"], line);
        let effect = attrs.get("effect").or(attrs.main(&["together"])).unwrap_or("fade").to_string();
        if !ENTRANCE_EFFECTS.contains(&effect.as_str()) {
            self.error(line, format!("unknown entrance effect `{effect}`; the effects are {}", ENTRANCE_EFFECTS.join(", ")));
            return None;
        }
        let easing = attrs.get("easing").map(String::from);
        if easing.as_deref().is_some_and(|e| e.contains(['"', '<', '>', ';', '{', '}'])) {
            self.error(line, "`easing` must be a CSS easing like `ease-in-out` or `cubic-bezier(0.2, 0, 0, 1)`");
            return None;
        }
        let rest = match attrs.get("rest") {
            Some(effect) => match rest_attributes(&Attrs::default(), effect) {
                Ok(html) => Some(html),
                Err(message) => {
                    self.error(line, message);
                    None
                }
            },
            None => None,
        };
        let together = attrs.has_word("together");
        let spec = RevealSpec {
            effect,
            duration_ms: self.seconds(attrs, "duration", line),
            delay_ms: self.seconds(attrs, "delay", line),
            easing,
            group: self.new_group(),
        };
        Some(Container::Reveal { spec, together, rest })
    }

    /// Turn the Markdown between directives into blocks.
    fn flush(&mut self, segment: &str, first_line: usize) {
        if segment.trim().is_empty() {
            return;
        }
        for (range_start, range_end) in top_level_blocks(segment) {
            let source = &segment[range_start..range_end];
            let line = first_line + crate::diagnostics::line_of(segment, range_start) - 1;
            self.block(source, line);
        }
    }

    fn block(&mut self, source: &str, line: usize) {
        let inline = directive::apply_inline(source);
        for message in inline.errors {
            self.error(line, message);
        }

        let rendered = render::render_block(
            &inline.render,
            &mut RenderContext { assets: self.assets, diagnostics: self.diagnostics, file: self.file, line },
        );
        let (text, lines) = render::plain_text(&inline.matching);

        // Attributes from enclosing containers, innermost first.
        let mut attributes = String::new();
        let mut reveal = None;
        let mut has_style = false;
        let mut has_rest = false;
        for (container, _) in self.stack.iter().rev() {
            match container {
                Container::Style(style) if !has_style => {
                    attributes.push_str(&format!(" data-tome-style=\"{style}\""));
                    has_style = true;
                }
                Container::Fx(html) if !has_rest => {
                    attributes.push_str(html);
                    has_rest = true;
                }
                Container::Reveal { spec, rest, .. } if reveal.is_none() => {
                    reveal = Some(spec.clone());
                    if let (Some(html), false) = (rest, has_rest) {
                        attributes.push_str(html);
                        has_rest = true;
                    }
                }
                _ => {}
            }
        }
        // Each block in a reveal is its own step, unless the reveal is `together`.
        if let Some(spec) = &mut reveal {
            let together = self.stack.iter().rev().find_map(|(c, _)| match c {
                Container::Reveal { together, .. } => Some(*together),
                _ => None,
            });
            if together != Some(true) {
                spec.group = self.new_group();
            }
        }

        let image_only = rendered.image_only;
        self.items.push(Item::Block(ParsedBlock {
            html: add_attributes(&rendered.html, &attributes),
            words: if image_only { 0 } else { text.split_whitespace().count() },
            characters: if image_only { 0 } else { text.trim().chars().count() },
            lines: if image_only { 0 } else { lines.max(1) },
            match_text: rendered.h1.unwrap_or(text),
            forced_refs: inline.forced_refs,
            reveal,
            line,
        }));
    }

    fn new_group(&mut self) -> usize {
        self.next_group += 1;
        self.next_group
    }

    fn seconds(&mut self, attrs: &Attrs, key: &str, line: usize) -> Option<u64> {
        let value = attrs.get(key)?;
        match number(key, value) {
            Ok(seconds) => Some((seconds * 1000.0).round() as u64),
            Err(message) => {
                self.error(line, format!("{message} (seconds)"));
                None
            }
        }
    }

    fn volume(&mut self, attrs: &Attrs, line: usize) -> Option<f64> {
        let value = attrs.get("volume")?;
        match number("volume", value) {
            Ok(volume) if volume <= 1.0 => Some(volume),
            _ => {
                self.error(line, format!("`volume` must be between 0 and 1, not `{value}`"));
                None
            }
        }
    }

    fn check_options(&mut self, name: &str, attrs: &Attrs, allowed: &[&str], line: usize) {
        for key in attrs.named.keys() {
            if !allowed.contains(&key.as_str()) {
                let options = if allowed.is_empty() { "none".to_string() } else { allowed.join(", ") };
                self.diagnostics.warning(Some(self.file), Some(line), format!("`::{name}` has no option `{key}`")).help =
                    Some(format!("its options are: {options}"));
            }
        }
    }

    fn unknown(&mut self, name: &str, known: &str, line: usize) {
        self.diagnostics.error(Some(self.file), Some(line), format!("unknown directive `{name}`")).help = Some(format!("the directives here are: {known}"));
    }

    fn error(&mut self, line: usize, message: impl Into<String>) {
        self.diagnostics.error(Some(self.file), Some(line), message);
    }
}

/// Byte ranges of the top-level Markdown blocks in some text.
fn top_level_blocks(text: &str) -> Vec<(usize, usize)> {
    let mut blocks = Vec::new();
    let mut depth = 0usize;
    let mut start = 0;
    for (event, range) in Parser::new_ext(text, render::options()).into_offset_iter() {
        match event {
            Event::Start(_) => {
                if depth == 0 {
                    start = range.start;
                }
                depth += 1;
            }
            Event::End(_) => {
                depth -= 1;
                if depth == 0 {
                    blocks.push((start, range.end));
                }
            }
            // A thematic break (`***`), e.g. a scene break, stands alone.
            Event::Rule if depth == 0 => blocks.push((range.start, range.end)),
            _ => {}
        }
    }
    blocks
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn parse(source: &str) -> (Option<ParsedChapter>, Diagnostics) {
        let dir = tempfile::tempdir().unwrap();
        for file in ["assets/audio/harbour.ogg", "assets/audio/rain.ogg", "assets/audio/bell.ogg", "assets/voice/line-1.ogg", "assets/images/bridge.png"] {
            let path = dir.path().join(file);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, file).unwrap();
        }
        let file = dir.path().join("manuscript/01-the-harbour.md");
        let mut assets = Assets::new(dir.path());
        let mut diagnostics = Diagnostics::default();
        let chapter = parse_chapter(&file, source, &mut assets, &mut diagnostics);
        (chapter, diagnostics)
    }

    fn blocks(chapter: &ParsedChapter) -> Vec<&ParsedBlock> {
        chapter.items.iter().filter_map(|i| if let Item::Block(b) = i { Some(b) } else { None }).collect()
    }

    #[test]
    fn takes_the_title_from_the_first_heading_and_the_id_from_the_file() {
        let (chapter, diagnostics) = parse("# The Harbour\n\nThe ferry came in late.\n");
        let chapter = chapter.unwrap();
        assert_eq!((chapter.id.as_str(), chapter.title.as_str()), ("the-harbour", "The Harbour"));
        assert_eq!(blocks(&chapter).len(), 1);
        assert!(diagnostics.items.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn reads_front_matter() {
        let (chapter, _) = parse("+++\nid = \"harbour\"\ntitle = \"At the Harbour\"\n[pagination]\nmax_words = 50\n+++\n# Ignored? No: kept as text\n");
        let chapter = chapter.unwrap();
        assert_eq!(chapter.id, "harbour");
        assert_eq!(chapter.title, "At the Harbour");
        assert_eq!(chapter.limits.max_words, Some(50));
        assert_eq!(blocks(&chapter).len(), 1, "with a front-matter title, a heading stays in the text");
    }

    #[test]
    fn captions_sounds_and_warns_when_one_has_none() {
        let (chapter, diagnostics) = parse("# T\n\n::sfx{bell caption=\"a bell tolls, uneven\"}\n::ambient{rain}\nOne.\n");
        let captions: Vec<_> = chapter
            .unwrap()
            .items
            .into_iter()
            .filter_map(|i| match i {
                Item::Cue { cue: Cue::Sfx { caption, .. } | Cue::Ambient { caption, .. }, .. } => Some(caption),
                _ => None,
            })
            .collect();
        assert_eq!(captions, [Some("a bell tolls, uneven".to_string()), None]);
        assert_eq!(diagnostics.items.len(), 1, "{diagnostics:?}");
        assert!(diagnostics.items[0].message.contains("no caption"));
    }

    #[test]
    fn parses_cues_breaks_and_illustrations_in_order() {
        let source = "# T\n\n::music{harbour volume=0.8 fade=2 caption=music}\n::ambient{rain caption=rain}\nOne.\n\n::pagebreak\n::illustration{bridge alt=\"A bridge\"}\n::sfx{bell delay=1.5 caption=\"a bell\"}\n::voice{line-1 delay=0.5}\nTwo.\n\n::ambient{stop}\n::music{stop fade=0}\n::illustration{none}\nThree.\n";
        let (chapter, diagnostics) = parse(source);
        assert!(diagnostics.items.is_empty(), "{diagnostics:?}");
        let kinds: Vec<String> = chapter
            .unwrap()
            .items
            .iter()
            .map(|i| match i {
                Item::Block(b) => format!("block {}", b.match_text.trim()),
                Item::Cue { cue, .. } => match cue {
                    Cue::Music { volume, fade_ms, .. } => format!("music {volume:?} {fade_ms:?}"),
                    Cue::Ambient { id, .. } => format!("ambient {id}"),
                    Cue::Sfx { delay_ms, .. } => format!("sfx {delay_ms:?}"),
                    Cue::MusicStop { fade_ms, .. } => format!("music stop {fade_ms:?}"),
                    Cue::AmbientStop { id, .. } => format!("ambient stop {id}"),
                    Cue::Voice { delay_ms, .. } => format!("voice {delay_ms:?}"),
                },
                Item::PageBreak => "break".into(),
                Item::Illustration { image, .. } => format!("illustration {}", image.as_ref().map_or("none", |i| i.alt.as_str())),
                Item::Paper { name, .. } => format!("paper {name}"),
            })
            .collect();
        assert_eq!(
            kinds,
            [
                "music Some(0.8) Some(2000)",
                "ambient rain",
                "block One.",
                "break",
                "illustration A bridge",
                "sfx Some(1500)",
                "voice Some(500)",
                "block Two.",
                "ambient stop *",
                "music stop Some(0)",
                "illustration none",
                "block Three.",
            ]
        );
    }

    #[test]
    fn gives_reveal_blocks_steps_and_container_attributes() {
        let source = "# T\n\n:::reveal{effect=typewriter delay=0.3}\nOne.\n\n:::style{handwriting}\nTwo.\n:::\n:::\n:::reveal{together rest=pulse}\nThree.\n\nFour.\n:::\n:::fx{wave speed=2}\nFive.\n:::\n";
        let (chapter, diagnostics) = parse(source);
        assert!(diagnostics.items.is_empty(), "{diagnostics:?}");
        let chapter = chapter.unwrap();
        let blocks = blocks(&chapter);
        let groups: Vec<_> = blocks.iter().map(|b| b.reveal.as_ref().map(|r| r.group)).collect();
        assert_ne!(groups[0], groups[1], "separate steps");
        assert_eq!(groups[2], groups[3], "together");
        assert_eq!(groups[4], None);
        assert_eq!(blocks[0].reveal.as_ref().unwrap().effect, "typewriter");
        assert_eq!(blocks[0].reveal.as_ref().unwrap().delay_ms, Some(300));
        assert_eq!(blocks[1].html, r#"<p data-tome-style="handwriting">Two.</p>"#);
        assert_eq!(blocks[2].html, r#"<p data-tome-rest="pulse">Three.</p>"#);
        assert_eq!(blocks[4].html, r#"<p data-tome-rest="wave" style="--tome-rest-duration: 2s">Five.</p>"#);
    }

    #[test]
    fn ignores_directives_in_code_and_reports_mistakes_with_lines() {
        let source = "# T\n\n```\n::music{nothing}\n```\n\n::music{storm}\n::sparkle\n:::reveal{effect=spin}\nText.\n:::style{whisper}\nUnclosed.\n";
        let (_, diagnostics) = parse(source);
        let found: Vec<_> = diagnostics.items.iter().map(|d| (d.line, d.message.clone())).collect();
        assert!(found.iter().any(|(l, m)| *l == Some(7) && m.contains("can't find the audio `storm`")), "{found:?}");
        assert!(found.iter().any(|(l, m)| *l == Some(8) && m.contains("unknown directive `sparkle`")), "{found:?}");
        assert!(found.iter().any(|(l, m)| *l == Some(9) && m.contains("unknown entrance effect `spin`")), "{found:?}");
        assert!(found.iter().any(|(l, m)| *l == Some(11) && m.contains("never closed")), "{found:?}");
        assert!(!found.iter().any(|(_, m)| m.contains("nothing")), "directives in code blocks are text");
    }

    #[test]
    fn counts_words_but_not_images() {
        let (chapter, _) = parse("# T\n\nThree short words.\n\n![A bridge](bridge)\n");
        let chapter = chapter.unwrap();
        let blocks = blocks(&chapter);
        assert_eq!((blocks[0].words, blocks[0].lines), (3, 1));
        assert_eq!((blocks[1].words, blocks[1].lines), (0, 0));
    }

    #[test]
    fn makes_ids_from_filenames() {
        assert_eq!(id_from_filename(Path::new("01-The Harbour.md")), "the-harbour");
        assert_eq!(id_from_filename(Path::new("2_coda.md")), "coda");
        assert_eq!(id_from_filename(Path::new("prologue.md")), "prologue");
    }
}
