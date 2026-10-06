//! Word timings for narration: reading a timing file (WebVTT, SRT or JSON) and aligning its
//! words to the paragraph a voice cue narrates, so the runtime can highlight each word as
//! it's spoken.

use serde::Deserialize;

/// A stretch of the recording and the words spoken in it: a word, a phrase or a sentence.
#[derive(Debug, Clone, PartialEq)]
pub struct Segment {
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
}

/// Read a timing file, choosing the format by its extension.
pub fn parse(text: &str, extension: &str) -> Result<Vec<Segment>, String> {
    let segments = match extension {
        "vtt" | "srt" => parse_cues(text)?,
        "json" => parse_json(text)?,
        other => return Err(format!("timing files must be .vtt, .srt or .json, not .{other}")),
    };
    if segments.is_empty() {
        return Err("the timing file has no timed text".into());
    }
    Ok(segments)
}

/// WebVTT and SRT: blocks with a `start --> end` line followed by text.
fn parse_cues(text: &str) -> Result<Vec<Segment>, String> {
    let mut segments = Vec::new();
    for block in text.replace("\r\n", "\n").split("\n\n") {
        let mut lines = block.lines().skip_while(|l| !l.contains("-->"));
        let Some(times) = lines.next() else { continue };
        let mut parts = times.split("-->");
        let (Some(start), Some(end)) = (parts.next(), parts.next()) else { continue };
        let end = end.split_whitespace().next().unwrap_or("");
        let start_ms = timestamp(start.trim()).ok_or_else(|| format!("can't read the time `{}`", start.trim()))?;
        let end_ms = timestamp(end).ok_or_else(|| format!("can't read the time `{end}`"))?;
        let words: Vec<String> = lines.map(strip_tags).collect();
        segments.push(Segment { start_ms, end_ms, text: words.join(" ") });
    }
    Ok(segments)
}

/// `hh:mm:ss.mmm`, `mm:ss.mmm`, with `.` or `,` before the milliseconds.
fn timestamp(text: &str) -> Option<u64> {
    let text = text.replace(',', ".");
    let parts: Vec<&str> = text.split(':').collect();
    let (hours, minutes, seconds) = match parts[..] {
        [h, m, s] => (h.parse::<u64>().ok()?, m.parse::<u64>().ok()?, s),
        [m, s] => (0, m.parse::<u64>().ok()?, s),
        _ => return None,
    };
    let seconds: f64 = seconds.parse().ok()?;
    Some(hours * 3_600_000 + minutes * 60_000 + (seconds * 1000.0).round() as u64)
}

fn strip_tags(line: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in line.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out
}

#[derive(Deserialize)]
#[serde(untagged)]
enum JsonTiming {
    /// `[{"start": 0.0, "end": 1.2, "text": "…"}]`, seconds.
    Segments(Vec<JsonSegment>),
    /// Whisper-style `{"segments": [...]}` or `{"words": [{"word": "…", "start", "end"}]}`.
    Object { segments: Option<Vec<JsonSegment>>, words: Option<Vec<JsonWord>> },
}

#[derive(Deserialize)]
struct JsonSegment {
    start: f64,
    end: f64,
    text: String,
}

#[derive(Deserialize)]
struct JsonWord {
    word: String,
    start: f64,
    end: f64,
}

fn parse_json(text: &str) -> Result<Vec<Segment>, String> {
    let timing: JsonTiming = serde_json::from_str(text).map_err(|e| format!("the timing file isn't valid JSON timing: {e}"))?;
    let ms = |seconds: f64| (seconds.max(0.0) * 1000.0).round() as u64;
    Ok(match timing {
        JsonTiming::Segments(segments) | JsonTiming::Object { segments: Some(segments), .. } => {
            segments.into_iter().map(|s| Segment { start_ms: ms(s.start), end_ms: ms(s.end), text: s.text }).collect()
        }
        JsonTiming::Object { words: Some(words), .. } => {
            words.into_iter().map(|w| Segment { start_ms: ms(w.start), end_ms: ms(w.end), text: w.word }).collect()
        }
        JsonTiming::Object { .. } => vec![],
    })
}

/// Lowercase letters and digits only, so punctuation and typography don't block a match.
fn normalize(word: &str) -> String {
    word.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).collect()
}

/// How far ahead to look for a timing word that doesn't match the next word of the text.
const RESYNC_WINDOW: usize = 4;

/// When each of `words` (the paragraph's words, in order) starts, in milliseconds, and the
/// share of the timing's words found in the text. Words within a phrase share its time in
/// proportion to their length. A word the narration skips takes the next spoken word's
/// start, so the highlight passes over it; words after the narration ends get None and are
/// never highlighted (a line may voice only part of a paragraph, such as its dialogue).
pub fn align(words: &[&str], segments: &[Segment]) -> (Vec<Option<u64>>, f64) {
    let targets: Vec<String> = words.iter().map(|w| normalize(w)).collect();
    let mut starts: Vec<Option<u64>> = vec![None; words.len()];
    let mut next = 0;
    let (mut spoken_total, mut spoken_found) = (0usize, 0usize);

    for segment in segments {
        // This segment's words, matched to the text where possible.
        let spoken: Vec<String> = segment.text.split_whitespace().map(normalize).filter(|w| !w.is_empty()).collect();
        let mut matched: Vec<usize> = Vec::new();
        spoken_total += spoken.len();
        for word in &spoken {
            let found = (next..(next + RESYNC_WINDOW).min(targets.len())).find(|&i| &targets[i] == word);
            if let Some(index) = found {
                matched.push(index);
                spoken_found += 1;
                next = index + 1;
            }
        }
        // Spread the segment's time over its matched words by length.
        let total: usize = matched.iter().map(|&i| targets[i].chars().count().max(1)).sum();
        let span = segment.end_ms.saturating_sub(segment.start_ms) as f64;
        let mut elapsed = 0usize;
        for index in matched {
            starts[index] = Some(segment.start_ms + (span * elapsed as f64 / total.max(1) as f64).round() as u64);
            elapsed += targets[index].chars().count().max(1);
        }
    }

    // Fill gaps from the right: skipped words take the next spoken word's start.
    let mut following: Option<u64> = None;
    let mut filled: Vec<Option<u64>> = starts
        .into_iter()
        .rev()
        .map(|start| {
            if start.is_some() {
                following = start;
            }
            following
        })
        .collect();
    filled.reverse();
    let share = if spoken_total == 0 { 0.0 } else { spoken_found as f64 / spoken_total as f64 };
    (filled, share)
}

/// The words of a block's HTML as the reader sees them (tags removed, entities decoded).
pub fn html_words(html: &str) -> Vec<String> {
    let text = strip_tags(html).replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&#39;", "'").replace("&amp;", "&");
    text.split_whitespace().map(String::from).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_webvtt_and_srt() {
        let vtt = "WEBVTT\n\nNOTE made by hand\n\n1\n00:00:00.000 --> 00:00:01.500 align:start\nShe found <b>the</b> address\n\n00:01.500 --> 00:03.250\non the letter.\n";
        let segments = parse(vtt, "vtt").unwrap();
        assert_eq!(segments, [
            Segment { start_ms: 0, end_ms: 1500, text: "She found the address".into() },
            Segment { start_ms: 1500, end_ms: 3250, text: "on the letter.".into() },
        ]);
        let srt = "1\n00:00:00,000 --> 00:00:01,500\nShe found\nthe address\n\n2\n00:00:01,500 --> 00:00:02,000\nhere\n";
        assert_eq!(parse(srt, "srt").unwrap()[0].text, "She found the address");
    }

    #[test]
    fn reads_json_segments_and_words() {
        let plain = r#"[{"start": 0, "end": 1.25, "text": "Which daughter?"}]"#;
        assert_eq!(parse(plain, "json").unwrap()[0].end_ms, 1250);
        let whisper = r#"{"text": "…", "segments": [{"start": 0.5, "end": 1.0, "text": " Hello"}]}"#;
        assert_eq!(parse(whisper, "json").unwrap()[0].start_ms, 500);
        let words = r#"{"words": [{"word": "You", "start": 0.0, "end": 0.3}, {"word": "came", "start": 0.3, "end": 0.7}]}"#;
        assert_eq!(parse(words, "json").unwrap().len(), 2);
        assert!(parse("{}", "json").is_err());
        assert!(parse("", "txt").is_err());
    }

    #[test]
    fn aligns_phrases_to_words_by_length_ignoring_punctuation() {
        let words = ["“If", "you’re", "waiting,”", "he", "said."];
        let segments = [
            Segment { start_ms: 0, end_ms: 1200, text: "If you're waiting".into() },
            Segment { start_ms: 1500, end_ms: 2000, text: "he said".into() },
        ];
        let (starts, share) = align(&words, &segments);
        // "if"(2) "youre"(5) "waiting"(7) share 1200ms: 0, 2/14, 7/14 of it; "he"(2) "said"(4) share 500.
        assert_eq!(starts, [Some(0), Some(171), Some(600), Some(1500), Some(1667)]);
        assert_eq!(share, 1.0);
    }

    #[test]
    fn resyncs_over_words_the_timing_skips_or_adds() {
        let words = ["The", "old", "keeper", "was", "younger"];
        // The recording drops "old" and adds "um".
        let segments = [
            Segment { start_ms: 0, end_ms: 100, text: "The".into() },
            Segment { start_ms: 100, end_ms: 200, text: "um".into() },
            Segment { start_ms: 200, end_ms: 300, text: "keeper".into() },
            Segment { start_ms: 300, end_ms: 400, text: "was younger".into() },
        ];
        let (starts, share) = align(&words, &segments);
        // "old" is skipped: it takes "keeper"'s start, so the highlight passes over it.
        assert_eq!(starts, [Some(0), Some(200), Some(200), Some(300), Some(330)]);
        assert_eq!(share, 0.8, "4 of the 5 spoken words are in the text");
    }

    #[test]
    fn leaves_words_after_the_narration_unlit() {
        let words = ["“You’ll", "be", "the", "daughter,”", "he", "said."];
        let segments = [Segment { start_ms: 0, end_ms: 900, text: "You'll be the daughter.".into() }];
        let (starts, share) = align(&words, &segments);
        assert_eq!(&starts[4..], [None, None]);
        assert_eq!(share, 1.0, "voicing only the dialogue is a full match");
    }

    #[test]
    fn reads_words_from_html() {
        assert_eq!(html_words(r#"<p>Tar &amp; <span data-tome-rest="wave">rope</span>.</p>"#), ["Tar", "&", "rope."]);
    }
}
