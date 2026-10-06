//! The manuscript's extensions to Markdown, in the generic-directive style:
//!
//! - leaf, on its own line: `::music{harbour fade=2}`
//! - container: `:::reveal{effect=typewriter}` … `:::`
//! - inline: `:fx[so kind]{wave}`
//!
//! Attributes are bare words (`stop`, or a main value like `harbour`) and `key=value`
//! pairs, with values optionally in double quotes.

use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Attrs {
    /// Bare words, in order.
    pub words: Vec<String>,
    pub named: BTreeMap<String, String>,
}

impl Attrs {
    pub fn has_word(&self, word: &str) -> bool {
        self.words.iter().any(|w| w == word)
    }

    /// The first bare word other than the given keywords: the directive's main value.
    pub fn main(&self, keywords: &[&str]) -> Option<&str> {
        self.words.iter().map(String::as_str).find(|w| !keywords.contains(w))
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.named.get(key).map(String::as_str)
    }
}

/// Parse the inside of `{…}`.
pub fn parse_attrs(input: &str) -> Result<Attrs, String> {
    let mut attrs = Attrs::default();
    let mut chars = input.chars().peekable();

    loop {
        while chars.peek().is_some_and(|c| c.is_whitespace()) {
            chars.next();
        }
        let Some(&first) = chars.peek() else { break };
        if first == '"' || first == '=' {
            return Err(format!("unexpected `{first}`: attributes look like `word` or `key=value`"));
        }

        let mut key = String::new();
        while let Some(&c) = chars.peek() {
            if c.is_whitespace() || c == '=' {
                break;
            }
            key.push(c);
            chars.next();
        }

        if chars.peek() == Some(&'=') {
            chars.next();
            let value = if chars.peek() == Some(&'"') {
                chars.next();
                let mut value = String::new();
                loop {
                    match chars.next() {
                        None => return Err(format!("the value of `{key}` is missing its closing quote")),
                        Some('"') => break,
                        Some('\\') => value.extend(chars.next()),
                        Some(c) => value.push(c),
                    }
                }
                value
            } else {
                let mut value = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_whitespace() {
                        break;
                    }
                    value.push(c);
                    chars.next();
                }
                value
            };
            if attrs.named.insert(key.clone(), value).is_some() {
                return Err(format!("`{key}` is given twice"));
            }
        } else {
            attrs.words.push(key);
        }
    }
    Ok(attrs)
}

#[derive(Debug, Clone, PartialEq)]
pub enum DirectiveLine {
    /// `::name{…}`
    Leaf { name: String, attrs: Attrs },
    /// `:::name{…}`
    Open { name: String, attrs: Attrs },
    /// `:::`
    Close,
}

/// Recognise a directive line. `Ok(None)` for ordinary Markdown.
pub fn parse_line(line: &str) -> Result<Option<DirectiveLine>, String> {
    let line = line.trim_end();
    let colons = line.chars().take_while(|&c| c == ':').count();
    if colons < 2 {
        return Ok(None);
    }
    let rest = &line[colons..];
    if rest.is_empty() {
        return Ok(if colons >= 3 { Some(DirectiveLine::Close) } else { None });
    }

    let name_len = rest.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '-').count();
    if name_len == 0 || !rest.starts_with(|c: char| c.is_ascii_lowercase()) {
        return Ok(None);
    }
    let name = rest[..name_len].to_string();
    let after = rest[name_len..].trim();
    let attrs = if after.is_empty() {
        Attrs::default()
    } else if let Some(inner) = after.strip_prefix('{').and_then(|a| a.strip_suffix('}')) {
        parse_attrs(inner)?
    } else {
        return Err(format!("expected `{{…}}` after `{}{name}`, found `{after}`", ":".repeat(colons)));
    };

    Ok(Some(if colons == 2 { DirectiveLine::Leaf { name, attrs } } else { DirectiveLine::Open { name, attrs } }))
}

/// One paragraph's source with inline directives applied.
#[derive(Debug, Default, PartialEq)]
pub struct InlineResult {
    /// Markdown with inline directives turned into HTML spans, ready to render.
    pub render: String,
    /// Markdown for reference matching: `:noref` text removed, `:ref` text kept.
    pub matching: String,
    /// Reference IDs forced with `:ref[…]{id}`.
    pub forced_refs: Vec<String>,
    pub errors: Vec<String>,
}

pub const REST_EFFECTS: &[&str] = &["pulse", "breathe", "tremble", "gradient", "wave"];

/// Attributes for a resting effect: `data-tome-rest` plus parameters as custom properties.
pub fn rest_attributes(attrs: &Attrs, effect: &str) -> Result<String, String> {
    if !REST_EFFECTS.contains(&effect) {
        return Err(format!("unknown effect `{effect}`; the effects are {}", REST_EFFECTS.join(", ")));
    }
    let mut style = Vec::new();
    for (key, value) in &attrs.named {
        let property = match key.as_str() {
            "speed" | "duration" => format!("--tome-rest-duration: {}s", number(key, value)?),
            "amplitude" => format!("--tome-rest-amplitude: {}", length(key, value)?),
            "scale" => format!("--tome-rest-scale: {}", number(key, value)?),
            "min-opacity" | "min_opacity" => format!("--tome-rest-min-opacity: {}", number(key, value)?),
            "gradient" => format!("--tome-rest-gradient: var(--tome-gradient-{})", css_name(key, value)?),
            "effect" | "rest" => continue,
            other => return Err(format!("unknown option `{other}` for the {effect} effect")),
        };
        style.push(property);
    }
    let mut out = format!(" data-tome-rest=\"{effect}\"");
    if !style.is_empty() {
        out.push_str(&format!(" style=\"{}\"", style.join("; ")));
    }
    Ok(out)
}

pub fn number(key: &str, value: &str) -> Result<f64, String> {
    value.parse::<f64>().ok().filter(|n| n.is_finite() && *n >= 0.0).ok_or_else(|| format!("`{key}` must be a number, not `{value}`"))
}

/// A CSS length; bare numbers mean ems.
fn length(key: &str, value: &str) -> Result<String, String> {
    if let Ok(n) = value.parse::<f64>() {
        return Ok(format!("{n}em"));
    }
    let digits = value.trim_end_matches(|c: char| c.is_ascii_alphabetic() || c == '%');
    if digits.parse::<f64>().is_ok() && ["em", "rem", "px", "%"].contains(&&value[digits.len()..]) {
        Ok(value.to_string())
    } else {
        Err(format!("`{key}` must be a length like 0.2em or 3px, not `{value}`"))
    }
}

pub fn css_name<'a>(key: &str, value: &'a str) -> Result<&'a str, String> {
    let valid = !value.is_empty() && value.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if valid { Ok(value) } else { Err(format!("`{key}` must be a lowercase name like `dawn-light`, not `{value}`")) }
}

/// Apply `:fx[…]{…}`, `:style[…]{…}`, `:ref[…]{…}` and `:noref[…]` in a paragraph's source.
pub fn apply_inline(source: &str) -> InlineResult {
    let mut out = InlineResult::default();
    let mut rest = source;

    while let Some(found) = find_inline(rest) {
        out.render.push_str(&rest[..found.start]);
        out.matching.push_str(&rest[..found.start]);
        let text = found.text;

        let attrs = match parse_attrs(found.attrs.unwrap_or("")) {
            Ok(attrs) => attrs,
            Err(error) => {
                out.errors.push(format!(":{}: {error}", found.name));
                Attrs::default()
            }
        };

        match found.name {
            "fx" => match attrs.main(&[]).or(attrs.get("effect")) {
                Some(effect) => match rest_attributes(&attrs, effect) {
                    Ok(html_attrs) => {
                        out.render.push_str(&format!("<span{html_attrs}>{text}</span>"));
                        out.matching.push_str(text);
                    }
                    Err(error) => {
                        out.errors.push(error);
                        out.render.push_str(text);
                        out.matching.push_str(text);
                    }
                },
                None => {
                    out.errors.push("`:fx[…]` needs an effect, like `:fx[text]{wave}`".into());
                    out.render.push_str(text);
                    out.matching.push_str(text);
                }
            },
            "style" => match attrs.main(&[]).map(|s| css_name("style", s)) {
                Some(Ok(style)) => {
                    out.render.push_str(&format!("<span data-tome-style=\"{style}\">{text}</span>"));
                    out.matching.push_str(text);
                }
                Some(Err(error)) => out.errors.push(error),
                None => out.errors.push("`:style[…]` needs a style name, like `:style[text]{whisper}`".into()),
            },
            "ref" => {
                out.render.push_str(text);
                out.matching.push_str(text);
                match attrs.main(&[]) {
                    Some(id) => out.forced_refs.push(id.to_string()),
                    None => out.errors.push("`:ref[…]` needs a reference ID, like `:ref[the old woman]{elara}`".into()),
                }
            }
            "noref" => {
                out.render.push_str(text);
                // Keep word boundaries intact around the removed text.
                out.matching.push(' ');
            }
            _ => unreachable!(),
        }
        rest = &rest[found.end..];
    }
    out.render.push_str(rest);
    out.matching.push_str(rest);
    out
}

struct InlineMatch<'a> {
    start: usize,
    end: usize,
    name: &'a str,
    text: &'a str,
    attrs: Option<&'a str>,
}

const INLINE_NAMES: &[&str] = &["fx", "style", "ref", "noref"];

fn find_inline(source: &str) -> Option<InlineMatch<'_>> {
    let mut search_from = 0;
    while let Some(offset) = source[search_from..].find(':') {
        let start = search_from + offset;
        search_from = start + 1;

        // Not part of a word or a longer run of colons (times, URLs, leaf directives).
        if source[..start].chars().next_back().is_some_and(|c| c.is_alphanumeric() || c == ':') {
            continue;
        }
        let after = &source[start + 1..];
        let Some(name) = INLINE_NAMES.iter().find(|n| after.starts_with(&format!("{n}["))) else { continue };

        let text_start = start + 1 + name.len() + 1;
        let Some(text_len) = balanced(&source[text_start..], '[', ']') else { continue };
        let text_end = text_start + text_len;
        let mut end = text_end + 1;

        let attrs = if source[end..].starts_with('{') {
            let attrs_start = end + 1;
            let Some(len) = balanced(&source[attrs_start..], '{', '}') else { continue };
            end = attrs_start + len + 1;
            Some(&source[attrs_start..attrs_start + len])
        } else {
            None
        };
        return Some(InlineMatch { start, end, name, text: &source[text_start..text_end], attrs });
    }
    None
}

/// Length of the text up to the bracket that closes an already-opened one.
fn balanced(text: &str, open: char, close: char) -> Option<usize> {
    let mut depth = 0;
    for (i, c) in text.char_indices() {
        if c == open {
            depth += 1;
        } else if c == close {
            if depth == 0 {
                return Some(i);
            }
            depth -= 1;
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_words_and_named_values() {
        let attrs = parse_attrs(r#"harbour fade=2 alt="A boat, \"late\"" stop"#).unwrap();
        assert_eq!(attrs.words, ["harbour", "stop"]);
        assert_eq!(attrs.get("fade"), Some("2"));
        assert_eq!(attrs.get("alt"), Some(r#"A boat, "late""#));
        assert_eq!(attrs.main(&["stop"]), Some("harbour"));
    }

    #[test]
    fn rejects_broken_attributes() {
        assert!(parse_attrs(r#"alt="unclosed"#).is_err());
        assert!(parse_attrs("fade=1 fade=2").is_err());
    }

    #[test]
    fn recognises_directive_lines() {
        assert_eq!(
            parse_line("::music{harbour}").unwrap(),
            Some(DirectiveLine::Leaf { name: "music".into(), attrs: parse_attrs("harbour").unwrap() })
        );
        assert_eq!(parse_line("::pagebreak").unwrap(), Some(DirectiveLine::Leaf { name: "pagebreak".into(), attrs: Attrs::default() }));
        assert!(matches!(parse_line(":::reveal{effect=slide}").unwrap(), Some(DirectiveLine::Open { .. })));
        assert_eq!(parse_line(":::").unwrap(), Some(DirectiveLine::Close));
        assert_eq!(parse_line("::::").unwrap(), Some(DirectiveLine::Close));
        // Ordinary text that merely starts with colons.
        assert_eq!(parse_line(":: not a directive").unwrap(), None);
        assert_eq!(parse_line("Time: 10:30").unwrap(), None);
        assert!(parse_line("::music harbour").is_err());
    }

    #[test]
    fn applies_inline_directives() {
        let result = apply_inline(r#"“Oh, :fx[*wonderful*]{wave speed=2}.” She met :ref[the old woman]{elara}, not :noref[Elara]."#);
        assert_eq!(
            result.render,
            r#"“Oh, <span data-tome-rest="wave" style="--tome-rest-duration: 2s">*wonderful*</span>.” She met the old woman, not Elara."#
        );
        assert_eq!(result.matching, "“Oh, *wonderful*.” She met the old woman, not  .");
        assert_eq!(result.forced_refs, ["elara"]);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn leaves_ordinary_colons_alone() {
        let source = "At 10:30 she read https://example.com: nothing [here]{x}.";
        assert_eq!(apply_inline(source).render, source);
    }

    #[test]
    fn reports_bad_inline_effects() {
        let result = apply_inline(":fx[text]{sparkle}");
        assert_eq!(result.render, "text");
        assert!(result.errors[0].contains("unknown effect `sparkle`"));
    }
}
