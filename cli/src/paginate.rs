//! Splitting a chapter into pages, attaching cues to the blocks that follow them and
//! numbering reveal steps within each page.

use crate::bundle::{Block, Cue, Footnote, ImageRef, Page, Reveal};
use crate::diagnostics::Diagnostics;
use crate::markdown::chapter::{Item, PageLimits, ParsedBlock, ParsedChapter};
use std::collections::HashMap;
use std::path::Path;

/// A page, with what reference matching needs to know about its text.
pub struct PaginatedPage {
    pub page: Page,
    /// Each block's matchable text and forced reference IDs, in order.
    pub texts: Vec<(String, Vec<String>)>,
}

#[derive(Default)]
struct Draft {
    blocks: Vec<(ParsedBlock, Vec<Cue>)>,
    illustration: Option<Option<ImageRef>>,
    paper: Option<String>,
    words: usize,
    characters: usize,
    lines: usize,
}

impl Draft {
    fn would_exceed(&self, block: &ParsedBlock, limits: &PageLimits) -> bool {
        let over = |limit: Option<usize>, current: usize, added: usize| limit.is_some_and(|max| current + added > max);
        over(limits.max_words, self.words, block.words)
            || over(limits.max_characters, self.characters, block.characters)
            || over(limits.max_lines, self.lines, block.lines)
    }

    fn add(&mut self, block: ParsedBlock, cues: Vec<Cue>) {
        self.words += block.words;
        self.characters += block.characters;
        self.lines += block.lines;
        self.blocks.push((block, cues));
    }
}

/// Fills drafts in order, holding cues and page settings until the block they belong with.
struct Drafter<'a> {
    drafts: Vec<Draft>,
    cues: Vec<Cue>,
    illustration: Option<(Option<ImageRef>, usize)>,
    paper: Option<(String, usize)>,
    limits: PageLimits,
    file: &'a Path,
    diagnostics: &'a mut Diagnostics,
}

impl Drafter<'_> {
    fn item(&mut self, item: Item) {
        match item {
            Item::PageBreak => {
                if !self.drafts.last().unwrap().blocks.is_empty() {
                    self.drafts.push(Draft::default());
                }
            }
            // Cues and illustration changes belong with the content that follows them.
            Item::Cue { cue, .. } => self.cues.push(cue),
            Item::Illustration { image, line } => self.illustration = Some((image, line)),
            Item::Paper { name, line } => self.paper = Some((name, line)),
            Item::Block(block) => self.block(block),
        }
    }

    fn block(&mut self, block: ParsedBlock) {
        let current = self.drafts.last().unwrap();
        if !current.blocks.is_empty() && current.would_exceed(&block, &self.limits) {
            self.drafts.push(Draft::default());
        }
        let current = self.drafts.last_mut().unwrap();
        if current.blocks.is_empty() && current.would_exceed(&block, &self.limits) {
            self.diagnostics.warning(Some(self.file), Some(block.line), "this paragraph alone is longer than the page limit").help =
                Some("it gets a page to itself and scrolls; split it if that's not what you want".into());
        }
        if let Some((image, _)) = self.illustration.take() {
            current.illustration = Some(image);
        }
        if let Some((paper, line)) = self.paper.take() {
            if current.paper.as_ref().is_some_and(|p| p != &paper) {
                self.diagnostics.warning(Some(self.file), Some(line), "this page already has a paper; the later `::paper` wins");
            }
            current.paper = Some(paper);
        }
        current.add(block, std::mem::take(&mut self.cues));
    }

    /// The drafts that have text. Anything after the last paragraph attaches to the end of the last page.
    fn finish(mut self) -> Vec<Draft> {
        self.drafts.retain(|d| !d.blocks.is_empty());
        let last = self.drafts.last_mut().expect("chapters have text");
        if let Some((image, line)) = self.illustration {
            if last.illustration.is_some() {
                self.diagnostics.warning(Some(self.file), Some(line), "this illustration comes after the chapter's last paragraph and is ignored");
            } else {
                last.illustration = Some(image);
            }
        }
        if let Some((_, line)) = self.paper {
            self.diagnostics.warning(Some(self.file), Some(line), "this `::paper` comes after the chapter's last paragraph and is ignored");
        }
        if !self.cues.is_empty() {
            // An empty anchor block at the same step as the last paragraph, so the cues fire with it.
            let reveal = last.blocks.last().and_then(|(b, _)| b.reveal.clone());
            let anchor = ParsedBlock {
                html: String::new(),
                words: 0,
                characters: 0,
                lines: 0,
                match_text: String::new(),
                forced_refs: vec![],
                reveal,
                line: 0,
                footnotes: vec![],
            };
            last.blocks.push((anchor, self.cues));
        }
        self.drafts
    }
}

/// Pages for a chapter. Breaks fall only between blocks: automatically before a block that
/// would take the page over a limit, and wherever the author wrote `::pagebreak`.
pub fn paginate(chapter: ParsedChapter, limits: PageLimits, file: &Path, diagnostics: &mut Diagnostics) -> Vec<PaginatedPage> {
    let mut drafter = Drafter {
        drafts: vec![Draft::default()],
        cues: Vec::new(),
        illustration: None,
        paper: None,
        limits,
        file,
        diagnostics: &mut *diagnostics,
    };
    for item in chapter.items {
        drafter.item(item);
    }
    let drafts = drafter.finish();

    let chapter_paper = chapter.paper.map(|(name, _)| name);
    let mut next_block = 0;
    drafts
        .into_iter()
        .map(|draft| {
            // Reveal steps are numbered per page, in the order their groups first appear.
            let mut steps: HashMap<usize, u32> = HashMap::new();
            let mut texts = Vec::new();
            let mut footnotes: Vec<Footnote> = Vec::new();
            let blocks = draft
                .blocks
                .into_iter()
                .map(|(block, cues)| {
                    next_block += 1;
                    let reveal = block.reveal.map(|spec| {
                        let next = steps.len() as u32 + 1;
                        Reveal {
                            step: *steps.entry(spec.group).or_insert(next),
                            effect: spec.effect,
                            duration_ms: spec.duration_ms,
                            delay_ms: spec.delay_ms,
                            easing: spec.easing,
                        }
                    });
                    texts.push((block.match_text, block.forced_refs));
                    // A page carries the footnotes its blocks refer to, matched like the rest of its text.
                    for footnote in block.footnotes.iter().filter_map(|label| chapter.footnotes.iter().find(|f| f.label == *label)) {
                        if !footnotes.iter().any(|f| f.number == footnote.number) {
                            footnotes.push(Footnote { number: footnote.number, html: footnote.html.clone() });
                            texts.push((footnote.match_text.clone(), footnote.forced_refs.clone()));
                        }
                    }
                    let cues = cues.into_iter().map(|cue| align_narration(cue, &block.html, file, block.line, diagnostics)).collect();
                    Block { id: format!("{}-{next_block}", chapter.id), html: block.html, reveal, cues }
                })
                .collect();
            let paper = draft.paper.or_else(|| chapter_paper.clone());
            let page = Page { illustration: draft.illustration, blocks, words: draft.words, references: vec![], paper, footnotes };
            PaginatedPage { page, texts }
        })
        .collect()
}

/// Below this share of matched words, a timing file probably belongs to different text.
const MIN_TIMING_MATCH: f64 = 0.6;

/// Turn a voice cue's timing file into a start time for each word of the paragraph it narrates.
fn align_narration(cue: Cue, html: &str, file: &Path, line: usize, diagnostics: &mut Diagnostics) -> Cue {
    let Cue::Voice { src, volume, delay_ms, timing: Some(segments), .. } = cue else { return cue };
    let words = crate::timing::html_words(html);
    if words.is_empty() {
        diagnostics.warning(Some(file), Some(line), "this narration has word timings but no paragraph after it to highlight");
        return Cue::Voice { src, volume, delay_ms, words: None, timing: None };
    }
    let refs: Vec<&str> = words.iter().map(String::as_str).collect();
    let (starts, share) = crate::timing::align(&refs, &segments);
    if share < MIN_TIMING_MATCH {
        diagnostics.warning(Some(file), Some(line), format!("only {:.0}% of the narration's timed words are in the paragraph after it", share * 100.0)).help =
            Some("check the timing file belongs to this line; the paragraph is highlighted as a whole instead".into());
        return Cue::Voice { src, volume, delay_ms, words: None, timing: None };
    }
    Cue::Voice { src, volume, delay_ms, words: Some(starts), timing: None }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::chapter::RevealSpec;

    fn block(words: usize, group: Option<usize>) -> Item {
        Item::Block(ParsedBlock {
            html: format!("<p>{words}</p>"),
            words,
            characters: words * 5,
            lines: 1,
            match_text: format!("{words} words"),
            forced_refs: vec![],
            reveal: group.map(|group| RevealSpec { effect: "fade".into(), duration_ms: None, delay_ms: None, easing: None, group }),
            line: 1,
            footnotes: vec![],
        })
    }

    fn chapter(items: Vec<Item>) -> ParsedChapter {
        ParsedChapter { id: "ch".into(), title: "Ch".into(), header_image: None, limits: PageLimits::default(), paper: None, items, footnotes: vec![], content_hash: "h".into() }
    }

    fn sfx() -> Item {
        Item::Cue { cue: Cue::Sfx { src: "bell".into(), volume: None, delay_ms: None, caption: None }, line: 1 }
    }

    fn run(items: Vec<Item>, max_words: Option<usize>) -> (Vec<PaginatedPage>, Diagnostics) {
        let mut diagnostics = Diagnostics::default();
        let limits = PageLimits { max_words, ..Default::default() };
        (paginate(chapter(items), limits, Path::new("ch.md"), &mut diagnostics), diagnostics)
    }

    fn shape(pages: &[PaginatedPage]) -> Vec<Vec<String>> {
        pages.iter().map(|p| p.page.blocks.iter().map(|b| b.html.clone()).collect()).collect()
    }

    #[test]
    fn fills_pages_up_to_the_word_limit() {
        let (pages, diagnostics) = run(vec![block(40, None), block(50, None), block(30, None), block(10, None)], Some(100));
        assert_eq!(shape(&pages), [vec!["<p>40</p>", "<p>50</p>"], vec!["<p>30</p>", "<p>10</p>"]]);
        assert!(diagnostics.items.is_empty());
        let words: Vec<_> = pages.iter().map(|p| p.page.words).collect();
        assert_eq!(words, [90, 40]);
    }

    #[test]
    fn breaks_where_the_author_says_and_warns_about_oversized_paragraphs() {
        let (pages, diagnostics) = run(vec![block(10, None), Item::PageBreak, Item::PageBreak, block(500, None)], Some(100));
        assert_eq!(shape(&pages), [vec!["<p>10</p>"], vec!["<p>500</p>"]]);
        assert!(diagnostics.items[0].message.contains("longer than the page limit"));
    }

    #[test]
    fn attaches_cues_and_illustrations_to_what_follows() {
        let image = ImageRef { src: "bridge.png".into(), alt: "A bridge".into() };
        let (pages, _) = run(
            vec![block(1, None), Item::PageBreak, Item::Illustration { image: Some(image.clone()), line: 1 }, sfx(), block(2, None)],
            None,
        );
        assert!(pages[0].page.illustration.is_none() && pages[0].page.blocks[0].cues.is_empty());
        assert_eq!(pages[1].page.illustration, Some(Some(image)));
        assert_eq!(pages[1].page.blocks[0].cues.len(), 1);
    }

    #[test]
    fn numbers_reveal_steps_per_page() {
        let (pages, _) = run(vec![block(1, Some(7)), block(1, Some(7)), block(1, Some(9)), Item::PageBreak, block(1, Some(12))], None);
        let steps = |p: &PaginatedPage| p.page.blocks.iter().map(|b| b.reveal.as_ref().unwrap().step).collect::<Vec<_>>();
        assert_eq!(steps(&pages[0]), [1, 1, 2]);
        assert_eq!(steps(&pages[1]), [1]);
    }

    #[test]
    fn trailing_cues_fire_with_the_last_paragraph() {
        let (pages, _) = run(vec![block(1, Some(3)), sfx()], None);
        let anchor = pages[0].page.blocks.last().unwrap();
        assert_eq!((anchor.html.as_str(), anchor.cues.len(), anchor.reveal.as_ref().map(|r| r.step)), ("", 1, Some(1)));
    }

    #[test]
    fn papers_apply_to_one_page_over_the_chapter_default() {
        let mut chapter = chapter(vec![block(1, None), Item::PageBreak, Item::Paper { name: "letter".into(), line: 1 }, block(1, None), Item::PageBreak, block(1, None)]);
        chapter.paper = Some(("plain".into(), 1));
        let mut diagnostics = Diagnostics::default();
        let pages = paginate(chapter, PageLimits::default(), Path::new("ch.md"), &mut diagnostics);
        let papers: Vec<_> = pages.iter().map(|p| p.page.paper.as_deref()).collect();
        assert_eq!(papers, [Some("plain"), Some("letter"), Some("plain")]);
    }

    #[test]
    fn carries_each_pages_footnotes_once_with_their_text() {
        let footnote = |label: &str, number| crate::markdown::footnote::Footnote {
            label: label.into(),
            number,
            html: format!("<p>{label}</p>"),
            match_text: format!("{label} text"),
            forced_refs: vec![],
        };
        let with_refs = |labels: &[&str]| match block(1, None) {
            Item::Block(mut b) => {
                b.footnotes = labels.iter().map(ToString::to_string).collect();
                Item::Block(b)
            }
            other => other,
        };
        let mut chapter = chapter(vec![with_refs(&["ash", "bell"]), with_refs(&["ash"]), Item::PageBreak, with_refs(&["bell"])]);
        chapter.footnotes = vec![footnote("ash", 1), footnote("bell", 2)];
        let mut diagnostics = Diagnostics::default();
        let pages = paginate(chapter, PageLimits::default(), Path::new("ch.md"), &mut diagnostics);
        let numbers = |p: &PaginatedPage| p.page.footnotes.iter().map(|f| f.number).collect::<Vec<_>>();
        assert_eq!((numbers(&pages[0]), numbers(&pages[1])), (vec![1, 2], vec![2]));
        assert_eq!(pages[1].texts.last().unwrap().0, "bell text");
    }

    #[test]
    fn gives_blocks_stable_ids() {
        let (pages, _) = run(vec![block(1, None), Item::PageBreak, block(1, None)], None);
        assert_eq!(pages[1].page.blocks[0].id, "ch-2");
    }
}
