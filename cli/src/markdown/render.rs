//! Rendering one block of Markdown to HTML, resolving image paths as it goes.

use crate::assets::{AssetKind, Assets};
use crate::diagnostics::Diagnostics;
use pulldown_cmark::{CowStr, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use std::path::Path;

/// Options for rendering and counting one block. Blocks are parsed alone, away from the
/// chapter's footnote definitions, so footnote markers are recognised without them.
pub fn options() -> Options {
    Options::ENABLE_SMART_PUNCTUATION | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_OLD_FOOTNOTES
}

/// Options for splitting a chapter into blocks and for footnote definitions, in GitHub's
/// footnote syntax: one definition per `[^label]:` line, with indented paragraphs after it.
pub fn block_options() -> Options {
    Options::ENABLE_SMART_PUNCTUATION | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_FOOTNOTES
}

pub struct RenderContext<'a> {
    pub assets: &'a mut Assets,
    pub diagnostics: &'a mut Diagnostics,
    pub file: &'a Path,
    pub line: usize,
    /// The chapter's footnote labels, in order of first reference; a marker's number is its place here.
    pub footnotes: &'a mut Vec<String>,
}

#[derive(Debug, Default)]
pub struct Rendered {
    pub html: String,
    /// Images on their own don't count toward page limits.
    pub image_only: bool,
    /// The text of a top-level `# Heading`, if that's what the block is.
    pub h1: Option<String>,
    /// The label, if the block is a footnote definition (its HTML is the footnote's content).
    pub footnote: Option<String>,
    /// Labels of the footnotes the block refers to, in order.
    pub footnote_refs: Vec<String>,
}

/// Render a single top-level block.
pub fn render_block(markdown: &str, ctx: &mut RenderContext) -> Rendered {
    let mut events: Vec<Event> = Vec::new();
    let mut alt: Option<String> = None;
    let mut footnote = None;
    let mut footnote_refs = Vec::new();

    for event in Parser::new_ext(markdown, if is_footnote_definition(markdown) { block_options() } else { options() }) {
        match event {
            Event::Start(Tag::FootnoteDefinition(label)) => footnote = Some(label.to_string()),
            Event::End(TagEnd::FootnoteDefinition) => {}
            Event::FootnoteReference(label) => {
                let number = match ctx.footnotes.iter().position(|l| *l == *label) {
                    Some(index) => index + 1,
                    None => {
                        ctx.footnotes.push(label.to_string());
                        ctx.footnotes.len()
                    }
                };
                footnote_refs.push(label.to_string());
                events.push(Event::InlineHtml(CowStr::from(footnote_marker(number))));
            }
            Event::Start(Tag::Image { link_type, dest_url, title, id }) => {
                let resolved = match ctx.assets.resolve(&dest_url, ctx.file, AssetKind::Image) {
                    Ok(path) => path,
                    Err(message) => {
                        ctx.diagnostics.error(Some(ctx.file), Some(ctx.line), message);
                        dest_url.to_string()
                    }
                };
                alt = Some(String::new());
                events.push(Event::Start(Tag::Image { link_type, dest_url: CowStr::from(resolved), title, id }));
            }
            Event::End(TagEnd::Image) => {
                if alt.take().is_some_and(|a| a.trim().is_empty()) {
                    ctx.diagnostics
                        .warning(Some(ctx.file), Some(ctx.line), "an image has no alt text")
                        .help = Some("describe it for screen readers: ![A ferry in a grey harbour](harbour.png)".into());
                }
                events.push(Event::End(TagEnd::Image));
            }
            Event::Text(ref text) | Event::Code(ref text) => {
                if let Some(alt) = &mut alt {
                    alt.push_str(text);
                }
                events.push(event);
            }
            other => events.push(other),
        }
    }

    let h1 = match events.first() {
        Some(Event::Start(Tag::Heading { level: HeadingLevel::H1, .. })) => Some(plain_text_of(&events)),
        _ => None,
    };

    if let Some(figure) = figure(&events) {
        return Rendered { html: figure, image_only: true, h1, footnote, footnote_refs };
    }
    let image_only = is_image_paragraph(&events);
    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, events.into_iter());
    Rendered { html: html.trim_end().to_string(), image_only, h1, footnote, footnote_refs }
}

fn is_footnote_definition(markdown: &str) -> bool {
    markdown.starts_with("[^") && matches!(Parser::new_ext(markdown, block_options()).next(), Some(Event::Start(Tag::FootnoteDefinition(_))))
}

/// A footnote marker: a superscript button the runtime opens the footnote from.
fn footnote_marker(number: usize) -> String {
    format!(
        r#"<sup class="tome-footnote-ref"><button type="button" class="tome-footnote-marker" data-tome-footnote="{number}" aria-haspopup="dialog" aria-label="Footnote {number}">{number}</button></sup>"#
    )
}

/// A paragraph holding just one image.
fn is_image_paragraph(events: &[Event]) -> bool {
    matches!(events.first(), Some(Event::Start(Tag::Paragraph)))
        && matches!(events.get(1), Some(Event::Start(Tag::Image { .. })))
        && matches!(events.get(events.len().wrapping_sub(2)), Some(Event::End(TagEnd::Image)))
        && events[2..events.len() - 2].iter().all(|e| matches!(e, Event::Text(_) | Event::Code(_)))
}

/// `![alt](src "caption")` alone in a paragraph becomes a captioned figure.
fn figure(events: &[Event]) -> Option<String> {
    if !is_image_paragraph(events) {
        return None;
    }
    let Event::Start(Tag::Image { dest_url, title, .. }) = &events[1] else { return None };
    if title.is_empty() {
        return None;
    }
    let alt = plain_text_of(&events[2..events.len() - 2]);
    Some(format!(
        "<figure><img src=\"{}\" alt=\"{}\"><figcaption>{}</figcaption></figure>",
        escape(dest_url),
        escape(&alt),
        escape(title)
    ))
}

fn plain_text_of(events: &[Event]) -> String {
    let mut text = String::new();
    for event in events {
        match event {
            Event::Text(t) | Event::Code(t) => text.push_str(t),
            Event::SoftBreak | Event::HardBreak => text.push(' '),
            _ => {}
        }
    }
    text.trim().to_string()
}

/// Plain text of some Markdown, for counting words and matching names. Blocks are separated
/// by newlines; the second value counts the author's line breaks (paragraphs and hard breaks).
pub fn plain_text(markdown: &str) -> (String, usize) {
    let mut text = String::new();
    let mut lines = 0;
    for event in Parser::new_ext(markdown, options()) {
        match event {
            Event::Text(t) | Event::Code(t) => text.push_str(&t),
            Event::SoftBreak => text.push(' '),
            Event::HardBreak => {
                text.push('\n');
                lines += 1;
            }
            Event::End(TagEnd::Paragraph | TagEnd::Heading(_) | TagEnd::Item | TagEnd::CodeBlock) => {
                text.push('\n');
                lines += 1;
            }
            _ => {}
        }
    }
    (text, lines)
}

pub fn escape(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// Add attributes (each starting with a space) to the first element of some HTML.
pub fn add_attributes(html: &str, attributes: &str) -> String {
    if attributes.is_empty() {
        return html.to_string();
    }
    match html.strip_prefix('<').and_then(|rest| rest.find(['>', ' ', '/'])) {
        Some(end) => format!("{}{attributes}{}", &html[..end + 1], &html[end + 1..]),
        None => html.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn render(markdown: &str) -> (Rendered, Diagnostics) {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("assets/images")).unwrap();
        fs::write(dir.path().join("assets/images/map.png"), "png").unwrap();
        let mut assets = Assets::new(dir.path());
        let mut diagnostics = Diagnostics::default();
        let file = dir.path().join("manuscript/01.md");
        let mut footnotes = vec!["earlier".to_string()];
        let rendered = render_block(markdown, &mut RenderContext { assets: &mut assets, diagnostics: &mut diagnostics, file: &file, line: 3, footnotes: &mut footnotes });
        (rendered, diagnostics)
    }

    #[test]
    fn renders_with_smart_punctuation() {
        let (rendered, _) = render(r#""Nobody meets the evening boat," he said -- twice."#);
        assert_eq!(rendered.html, "<p>“Nobody meets the evening boat,” he said – twice.</p>");
        assert!(!rendered.image_only);
    }

    #[test]
    fn resolves_images_and_makes_captioned_figures() {
        let (rendered, diagnostics) = render(r#"![The old letter](map "The letter")"#);
        assert!(rendered.html.starts_with(r#"<figure><img src="assets/map."#), "{}", rendered.html);
        assert!(rendered.html.ends_with(r#"alt="The old letter"><figcaption>The letter</figcaption></figure>"#));
        assert!(rendered.image_only);
        assert!(diagnostics.items.is_empty());
    }

    #[test]
    fn warns_about_missing_alt_text_and_files() {
        let (_, diagnostics) = render("![](map) and ![x](nowhere.png)");
        let messages: Vec<_> = diagnostics.items.iter().map(|d| d.message.as_str()).collect();
        assert!(messages.iter().any(|m| m.contains("no alt text")));
        assert!(messages.iter().any(|m| m.contains("can't find the image `nowhere.png`")));
    }

    #[test]
    fn numbers_footnote_markers_per_chapter() {
        let (rendered, _) = render("Ash[^ash] and more[^earlier].");
        assert_eq!(rendered.footnote_refs, ["ash", "earlier"]);
        assert!(rendered.html.contains(r#"data-tome-footnote="2" aria-haspopup="dialog" aria-label="Footnote 2">2</button></sup> and"#), "{}", rendered.html);
        assert!(rendered.html.contains(r#"data-tome-footnote="1""#));
        assert_eq!(rendered.footnote, None);
    }

    #[test]
    fn renders_a_footnote_definition_as_its_content() {
        let (rendered, _) = render("[^ash]: Burnt *wood*.\n\n    A second paragraph.");
        assert_eq!(rendered.footnote.as_deref(), Some("ash"));
        assert_eq!(rendered.html, "<p>Burnt <em>wood</em>.</p>\n<p>A second paragraph.</p>");
    }

    #[test]
    fn counts_footnote_markers_as_no_text() {
        assert_eq!(plain_text("Ash[^ash] fell.").0, "Ash fell.\n");
    }

    #[test]
    fn detects_a_title_heading() {
        assert_eq!(render("# The Harbour").0.h1.as_deref(), Some("The Harbour"));
        assert_eq!(render("## Not a title").0.h1, None);
    }

    #[test]
    fn counts_text_and_lines() {
        let (text, lines) = plain_text("A *bold* line  \nand another.");
        assert_eq!(text, "A bold line\nand another.\n");
        assert_eq!(lines, 2);
    }

    #[test]
    fn adds_attributes_to_the_first_element() {
        assert_eq!(add_attributes("<p>Hi</p>", r#" data-tome-style="whisper""#), r#"<p data-tome-style="whisper">Hi</p>"#);
        assert_eq!(add_attributes(r#"<h2 id="x">Hi</h2>"#, " a"), r#"<h2 a id="x">Hi</h2>"#);
    }
}
