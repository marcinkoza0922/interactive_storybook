//! `theme/theme.toml`: the book's look, into the bundle's `theme`. TOML has no null, so a
//! field an override clears is written as `"none"`.

use crate::assets::{AssetKind, Assets};
use crate::bundle::{Background, Backgrounds, Decoration, FontFile, ImageRef, LandingTheme, PageFrame, Theme, ThemeOverride};
use crate::diagnostics::{Diagnostics, line_of};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use toml::Spanned;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ThemeSource {
    #[serde(default)]
    tokens: BTreeMap<String, String>,
    #[serde(default)]
    fonts: Vec<FontSource>,
    #[serde(default)]
    styles: BTreeMap<String, BTreeMap<String, String>>,
    backgrounds: Option<BackgroundsSource>,
    decoration: Option<DecorationSource>,
    landing: Option<LandingSource>,
    custom_css: Option<Spanned<String>>,
    #[serde(default, rename = "override")]
    overrides: Vec<OverrideSource>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FontSource {
    family: String,
    src: Spanned<String>,
    weight: Option<String>,
    style: Option<String>,
}

/// A table, or `"none"` to clear it in an override. Read as a raw value so the whole
/// field keeps its position for error messages.
type Clearable = Spanned<toml::Value>;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BackgroundsSource {
    landing: Option<Clearable>,
    reading: Option<Clearable>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BackgroundSource {
    src: String,
    poster: Option<String>,
    fit: Option<String>,
    position: Option<String>,
    opacity: Option<f64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DecorationSource {
    page_texture: Option<Spanned<String>>,
    page_frame: Option<Clearable>,
    chapter_ornament: Option<Spanned<String>>,
    drop_caps: Option<bool>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FrameSource {
    src: String,
    slice: u32,
    width: String,
    repeat: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LandingSource {
    cover: Option<Spanned<String>>,
    cover_alt: Option<String>,
    title_image: Option<Spanned<String>>,
    title_image_alt: Option<String>,
    layout: Option<Spanned<String>>,
    menu: Option<Spanned<String>>,
    music: Option<Spanned<String>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OverrideSource {
    from: Spanned<String>,
    #[serde(default)]
    tokens: BTreeMap<String, String>,
    #[serde(default)]
    styles: BTreeMap<String, BTreeMap<String, String>>,
    backgrounds: Option<BackgroundsSource>,
    decoration: Option<DecorationSource>,
}

struct Loader<'a> {
    file: PathBuf,
    text: String,
    assets: &'a mut Assets,
    diagnostics: &'a mut Diagnostics,
}

/// The theme, and the names of the special styles it defines. None if there's no theme file.
pub fn load(root: &Path, chapter_ids: &[String], assets: &mut Assets, diagnostics: &mut Diagnostics) -> Option<(Theme, Vec<String>)> {
    let file = root.join("theme/theme.toml");
    let custom_css = root.join("theme/custom.css");
    if !file.is_file() {
        // A custom stylesheet works without a theme file too.
        return custom_css.is_file().then(|| {
            let css = assets.resolve("./custom.css", &file, AssetKind::Css).ok();
            (Theme { custom_css: css, ..Theme::default() }, vec![])
        });
    }
    let text = match std::fs::read_to_string(&file) {
        Ok(text) => text,
        Err(error) => {
            diagnostics.error(Some(&file), None, format!("couldn't read the file: {error}"));
            return None;
        }
    };
    let source: ThemeSource = match toml::from_str(&text) {
        Ok(source) => source,
        Err(error) => {
            diagnostics.error(Some(&file), error.span().map(|s| line_of(&text, s.start)), error.message().to_string());
            return None;
        }
    };

    let mut loader = Loader { file: file.clone(), text, assets, diagnostics };
    let mut theme = Theme {
        tokens: loader.tokens(source.tokens),
        styles: loader.styles(source.styles),
        fonts: source
            .fonts
            .into_iter()
            .filter_map(|font| {
                let src = loader.asset(&font.src, AssetKind::Font)?;
                Some(FontFile { family: font.family, src, weight: font.weight, style: font.style })
            })
            .collect(),
        backgrounds: source.backgrounds.map(|b| loader.backgrounds(b)),
        decoration: source.decoration.map(|d| loader.decoration(d)),
        landing: source.landing.map(|l| loader.landing(l)),
        custom_css: match &source.custom_css {
            Some(css) => loader.asset(css, AssetKind::Css),
            None => custom_css.is_file().then(|| loader.assets.resolve("./custom.css", &file, AssetKind::Css).ok()).flatten(),
        },
        overrides: vec![],
    };

    for source in source.overrides {
        let from = source.from.get_ref().clone();
        if !chapter_ids.contains(&from) {
            let line = loader.line(&source.from);
            loader.diagnostics.error(Some(&file), Some(line), format!("there's no chapter with the ID `{from}`")).help =
                Some(format!("chapter IDs are: {}", chapter_ids.join(", ")));
            continue;
        }
        theme.overrides.push(ThemeOverride {
            from,
            tokens: loader.tokens(source.tokens),
            styles: loader.styles(source.styles),
            backgrounds: source.backgrounds.map(|b| loader.backgrounds(b)),
            decoration: source.decoration.map(|d| loader.decoration(d)),
        });
    }

    let mut styles: Vec<String> = theme.styles.keys().cloned().collect();
    styles.extend(theme.overrides.iter().flat_map(|o| o.styles.keys().cloned()));
    Some((theme, styles))
}

const UNSAFE: &[char] = &[';', '{', '}', '<', '>'];

fn valid_name(name: &str) -> bool {
    !name.is_empty() && name.split('-').all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()))
}

impl Loader<'_> {
    fn line<T>(&self, spanned: &Spanned<T>) -> usize {
        line_of(&self.text, spanned.span().start)
    }

    fn asset(&mut self, reference: &Spanned<String>, kind: AssetKind) -> Option<String> {
        let line = self.line(reference);
        self.asset_at(reference.get_ref(), line, kind)
    }

    fn asset_at(&mut self, reference: &str, line: usize, kind: AssetKind) -> Option<String> {
        match self.assets.resolve(reference, &self.file, kind) {
            Ok(path) => Some(path),
            Err(message) => {
                self.diagnostics.error(Some(&self.file), Some(line), message);
                None
            }
        }
    }

    /// `"none"` clears (Some(None)); a table becomes `T`; anything else is reported.
    fn clearable<T: serde::de::DeserializeOwned>(&mut self, value: &Clearable) -> Option<(Option<T>, usize)> {
        let line = self.line(value);
        match value.get_ref() {
            toml::Value::String(word) if word == "none" => Some((None, line)),
            toml::Value::Table(_) => match value.get_ref().clone().try_into::<T>() {
                Ok(parsed) => Some((Some(parsed), line)),
                Err(error) => {
                    self.diagnostics.error(Some(&self.file), Some(line), error.message().to_string());
                    None
                }
            },
            other => {
                self.diagnostics.error(Some(&self.file), Some(line), format!("expected a table or \"none\", not {other}"));
                None
            }
        }
    }

    /// An asset, or `"none"` to clear it in an override.
    fn clearable_asset(&mut self, reference: &Spanned<String>, kind: AssetKind) -> Option<Option<String>> {
        if reference.get_ref() == "none" { Some(None) } else { self.asset(reference, kind).map(Some) }
    }

    fn tokens(&mut self, tokens: BTreeMap<String, String>) -> BTreeMap<String, String> {
        tokens
            .into_iter()
            .filter(|(name, value)| {
                if !valid_name(name) {
                    self.diagnostics.error(Some(&self.file), None, format!("the token name `{name}` must be lowercase words joined by dashes, like `page-bg`"));
                    false
                } else if value.contains(UNSAFE) {
                    self.diagnostics.error(Some(&self.file), None, format!("the value of `{name}` can't contain ; {{ }} < or >"));
                    false
                } else {
                    true
                }
            })
            .collect()
    }

    fn styles(&mut self, styles: BTreeMap<String, BTreeMap<String, String>>) -> BTreeMap<String, BTreeMap<String, String>> {
        styles
            .into_iter()
            .filter_map(|(name, declarations)| {
                if !valid_name(&name) {
                    self.diagnostics.error(Some(&self.file), None, format!("the style name `{name}` must be lowercase words joined by dashes"));
                    return None;
                }
                let declarations = declarations
                    .into_iter()
                    .filter(|(property, value)| {
                        let ok = property.chars().all(|c| c.is_ascii_lowercase() || c == '-') && !value.contains(UNSAFE);
                        if !ok {
                            self.diagnostics.error(Some(&self.file), None, format!("style `{name}`: `{property} = \"{value}\"` isn't a valid CSS declaration"));
                        }
                        ok
                    })
                    .collect();
                Some((name, declarations))
            })
            .collect()
    }

    fn backgrounds(&mut self, source: BackgroundsSource) -> Backgrounds {
        Backgrounds { landing: source.landing.and_then(|b| self.background(b)), reading: source.reading.and_then(|b| self.background(b)) }
    }

    fn background(&mut self, source: Clearable) -> Option<Option<Background>> {
        let (background, line) = self.clearable::<BackgroundSource>(&source)?;
        let Some(background) = background else { return Some(None) };
        let src = self.asset_at(&background.src, line, AssetKind::Media)?;
        let poster = background.poster.as_deref().and_then(|p| self.asset_at(p, line, AssetKind::Image));
        if let Some(opacity) = background.opacity.filter(|o| !(0.0..=1.0).contains(o)) {
            self.diagnostics.error(Some(&self.file), Some(line), format!("`opacity` must be between 0 and 1, not {opacity}"));
        }
        Some(Some(Background { src, poster, fit: background.fit, position: background.position, opacity: background.opacity }))
    }

    fn decoration(&mut self, source: DecorationSource) -> Decoration {
        Decoration {
            page_texture: source.page_texture.as_ref().and_then(|t| self.clearable_asset(t, AssetKind::Image)),
            chapter_ornament: source.chapter_ornament.as_ref().and_then(|o| self.clearable_asset(o, AssetKind::Image)),
            page_frame: source.page_frame.as_ref().and_then(|frame| {
                let (frame, line) = self.clearable::<FrameSource>(frame)?;
                let Some(frame) = frame else { return Some(None) };
                let src = self.asset_at(&frame.src, line, AssetKind::Image)?;
                if frame.width.contains(UNSAFE) {
                    self.diagnostics.error(Some(&self.file), Some(line), "the frame `width` must be a CSS length like 18px");
                    return None;
                }
                Some(Some(PageFrame { src, slice: frame.slice, width: frame.width, repeat: frame.repeat }))
            }),
            drop_caps: source.drop_caps,
        }
    }

    fn landing(&mut self, source: LandingSource) -> LandingTheme {
        let image = |loader: &mut Self, src: &Option<Spanned<String>>, alt: &Option<String>, what: &str| {
            let src = src.as_ref()?;
            let path = loader.asset(src, AssetKind::Image)?;
            if alt.as_deref().is_none_or(|a| a.trim().is_empty()) {
                let line = loader.line(src);
                loader.diagnostics.warning(Some(&loader.file), Some(line), format!("the landing {what} has no alt text")).help =
                    Some(format!("add `{what}_alt = \"…\"`"));
            }
            Some(ImageRef { src: path, alt: alt.clone().unwrap_or_default() })
        };
        let cover = image(self, &source.cover, &source.cover_alt, "cover");
        let title_image = image(self, &source.title_image, &source.title_image_alt, "title_image");
        let layout = self.choice(source.layout, &["centered", "split"]);
        let menu = self.choice(source.menu, &["stacked", "inline"]);
        let music = source.music.as_ref().and_then(|m| self.asset(m, AssetKind::Audio));
        LandingTheme { cover, title_image, layout, menu, music }
    }

    fn choice(&mut self, value: Option<Spanned<String>>, allowed: &[&str]) -> Option<String> {
        let value = value?;
        if allowed.contains(&value.get_ref().as_str()) {
            return Some(value.into_inner());
        }
        let line = self.line(&value);
        self.diagnostics.error(Some(&self.file), Some(line), format!("expected one of {}, not \"{}\"", allowed.join(", "), value.get_ref()));
        None
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn load_theme(toml: &str) -> (Option<(Theme, Vec<String>)>, Diagnostics) {
        let dir = tempfile::tempdir().unwrap();
        for file in ["assets/images/paper.svg", "assets/video/fog.webm", "assets/images/fog.jpg", "assets/fonts/caveat.woff2", "assets/audio/keeper.ogg", "theme/custom.css"] {
            let path = dir.path().join(file);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, file).unwrap();
        }
        fs::write(dir.path().join("theme/theme.toml"), toml).unwrap();
        let mut assets = Assets::new(dir.path());
        let mut diagnostics = Diagnostics::default();
        let theme = load(dir.path(), &["one".to_string(), "two".to_string()], &mut assets, &mut diagnostics);
        (theme, diagnostics)
    }

    #[test]
    fn loads_a_full_theme() {
        let (theme, diagnostics) = load_theme(
            r##"
[tokens]
accent = "#8a3b2e"

[[fonts]]
family = "Caveat"
src = "caveat"

[styles.handwriting]
font-family = "Caveat, cursive"

[backgrounds.landing]
src = "fog"
poster = "fog.jpg"
opacity = 0.4

[decoration]
page_texture = "paper"
drop_caps = true

[landing]
layout = "split"
music = "keeper"

[[override]]
from = "two"
tokens = { accent = "gold" }
decoration = { page_texture = "none" }
backgrounds = { reading = "none" }
"##,
        );
        assert!(diagnostics.items.is_empty(), "{diagnostics:?}");
        let (theme, styles) = theme.unwrap();
        assert_eq!(styles, ["handwriting"]);
        assert!(theme.fonts[0].src.ends_with(".woff2"));
        assert!(theme.custom_css.as_deref().is_some_and(|c| c.ends_with(".css")), "custom.css is picked up automatically");
        let json = serde_json::to_value(&theme).unwrap();
        assert_eq!(json["overrides"][0]["decoration"]["page_texture"], serde_json::Value::Null);
        assert_eq!(json["overrides"][0]["backgrounds"]["reading"], serde_json::Value::Null);
        assert!(json["overrides"][0]["backgrounds"].get("landing").is_none(), "absent stays absent");
        assert_eq!(json["landing"]["layout"], "split");
    }

    #[test]
    fn reports_mistakes() {
        let (_, diagnostics) = load_theme(
            "[tokens]\naccent = \"red; } body { display: none\"\n\n[landing]\nlayout = \"diagonal\"\n\n[[override]]\nfrom = \"nine\"\n",
        );
        let found: Vec<_> = diagnostics.items.iter().map(|d| (d.line, d.message.clone())).collect();
        assert!(found.iter().any(|(_, m)| m.contains("can't contain")), "{found:?}");
        assert!(found.iter().any(|(l, m)| *l == Some(5) && m.contains("centered, split")), "{found:?}");
        assert!(found.iter().any(|(l, m)| *l == Some(8) && m.contains("`nine`")), "{found:?}");
    }
}
