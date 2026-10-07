//! The book bundle written to `book.json`: the contract with the runtime. These types mirror
//! `runtime/src/lib/bundle/types.ts`; change both together and bump the schema version on
//! breaking changes.

use serde::Serialize;
use std::collections::BTreeMap;

pub const SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Serialize)]
pub struct Bundle {
    pub bundle_schema_version: u32,
    pub book: BookMeta,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio: Option<AudioConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub illustrations: Option<IllustrationConfig>,
    pub chapters: Vec<Chapter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contents: Option<Vec<ContentsEntry>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub references: Vec<Reference>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<Theme>,
}

#[derive(Debug, Serialize)]
pub struct BookMeta {
    pub id: String,
    pub title: String,
    pub author: String,
    pub language: String,
}

#[derive(Debug, Serialize)]
pub struct AudioConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub restore_delay_ms: Option<u64>,
    /// Music and ambience volume while narration is heard (1: no ducking).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duck_level: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct IllustrationConfig {
    pub linger_ms: u64,
}

#[derive(Debug, Serialize)]
pub struct Chapter {
    pub id: String,
    pub title: String,
    pub content_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header_image: Option<ImageRef>,
    pub pages: Vec<Page>,
}

#[derive(Debug, Default, Serialize)]
pub struct Page {
    /// Absent: carry over. Some(None): clear the track.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub illustration: Option<Option<ImageRef>>,
    pub blocks: Vec<Block>,
    /// Words of text on the page, footnotes aside, for estimating reading time.
    pub words: usize,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub references: Vec<String>,
    /// A named paper from the theme, for this page.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paper: Option<String>,
    /// The footnotes the page's blocks refer to, in order of first reference.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub footnotes: Vec<Footnote>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Footnote {
    /// Numbered per chapter; the `data-tome-footnote` of its markers.
    pub number: u32,
    pub html: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Block {
    pub id: String,
    pub html: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reveal: Option<Reveal>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub cues: Vec<Cue>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Reveal {
    pub step: u32,
    pub effect: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delay_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub easing: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Cue {
    Music {
        src: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        volume: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        fade_ms: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        delay_ms: Option<u64>,
        /// Shown to readers with captions on, in place of the sound.
        #[serde(skip_serializing_if = "Option::is_none")]
        caption: Option<String>,
    },
    MusicStop {
        #[serde(skip_serializing_if = "Option::is_none")]
        fade_ms: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        delay_ms: Option<u64>,
    },
    Ambient {
        id: String,
        src: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        volume: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        fade_ms: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        delay_ms: Option<u64>,
        /// Shown to readers with captions on, in place of the sound.
        #[serde(skip_serializing_if = "Option::is_none")]
        caption: Option<String>,
    },
    AmbientStop {
        id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        fade_ms: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        delay_ms: Option<u64>,
    },
    Sfx {
        src: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        volume: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        delay_ms: Option<u64>,
        /// Shown to readers with captions on, in place of the sound.
        #[serde(skip_serializing_if = "Option::is_none")]
        caption: Option<String>,
    },
    /// A line of narration, queued behind any line already playing on the page.
    Voice {
        src: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        volume: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        delay_ms: Option<u64>,
        /// When each word of the narrated paragraph starts, in ms, for highlighting; null for
        /// words after the narration ends.
        #[serde(skip_serializing_if = "Option::is_none")]
        words: Option<Vec<Option<u64>>>,
        /// The timing file's segments, until pagination aligns them to the paragraph.
        #[serde(skip)]
        timing: Option<Vec<crate::timing::Segment>>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ImageRef {
    pub src: String,
    pub alt: String,
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ContentsEntry {
    Chapter {
        id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<String>,
    },
    Heading {
        title: String,
    },
}

#[derive(Debug, Serialize)]
pub struct Reference {
    pub id: String,
    pub sections: Vec<ReferenceSection>,
}

#[derive(Debug, Serialize)]
pub struct ReferenceSection {
    pub from: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub html: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<ImageRef>,
}

#[derive(Debug, Default, Serialize)]
pub struct Theme {
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub tokens: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub fonts: Vec<FontFile>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub styles: BTreeMap<String, BTreeMap<String, String>>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub papers: BTreeMap<String, Paper>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backgrounds: Option<Backgrounds>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decoration: Option<Decoration>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub landing: Option<LandingTheme>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_css: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub overrides: Vec<ThemeOverride>,
}

#[derive(Debug, Serialize)]
pub struct FontFile {
    pub family: String,
    pub src: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
}

/// Absent fields carry over in overrides; `Some(None)` serializes as null, which clears.
#[derive(Debug, Default, Serialize)]
pub struct Backgrounds {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub landing: Option<Option<Background>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reading: Option<Option<Background>>,
}

/// Unset fields keep the book's own paper; `texture: Some(None)` removes the texture.
#[derive(Debug, Default, Serialize)]
pub struct Paper {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub texture: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grain: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct Background {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub src: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poster: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opacity: Option<f64>,
}

#[derive(Debug, Default, Serialize)]
pub struct Decoration {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_texture: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_frame: Option<Option<PageFrame>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chapter_ornament: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drop_caps: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paper_grain: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct PageFrame {
    pub src: String,
    pub slice: u32,
    pub width: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repeat: Option<String>,
}

#[derive(Debug, Default, Serialize)]
pub struct LandingTheme {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover: Option<ImageRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title_image: Option<ImageRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layout: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub menu: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub music: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ThemeOverride {
    pub from: String,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub tokens: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub styles: BTreeMap<String, BTreeMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backgrounds: Option<Backgrounds>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decoration: Option<Decoration>,
}
