# Interactive Novel Framework — Specification

> **Status:** Draft v0.1 (2026-10-05), produced from a requirements interview.
> **Name:** The project has no name yet. `tome` is used throughout as a **placeholder** for the CLI and framework name. (`storybook` was rejected because it collides with Storybook.js, a well-known tool in the same npm/JS ecosystem the runtime uses.)

---

## 1. Overview

`tome` is an open-source framework for turning a **linear (non-branching) novel** into a **standalone, bespoke reading app**. It is *not* an ebook reader or document viewer: every book built with it ships as its own application with its own look, sound and atmosphere.

Authors work entirely in **data** — an extended-Markdown manuscript plus TOML configuration and asset files. A non-programmer should never need to read or write a line of code. Because the project is open source, authors who want something the framework doesn't support can extend it themselves.

The first real-world use is the project author's own novel; the framework must nonetheless stay general and reusable.

### 1.1 Core enhancements

| Enhancement | Summary |
|---|---|
| **Spoiler-aware references** | Characters, places and terms get reference entries that only reveal what the reader knows as of the chapters they've reached. |
| **Atmosphere** | Music, one-shot sound effects, unlimited layered ambient tracks; themes that can shift as the book progresses; image/video/GIF backgrounds. |
| **Kinetic & special text** | Step-by-step reveals (fade-ins etc.), special typography (handwriting and similar), all optional for the reader. |
| **Illustration track** | A parallel track of major illustrations shown as a facing page beside the text. |

### 1.2 Goals

- Make a single novel feel distinctive and crafted, not generic.
- Fully data-driven authoring; friendly errors; sensible defaults everywhere so a minimally configured book still looks good.
- Desktop first (Linux, Windows), web/PWA from the same build, mobile possible later.
- Accessible by default — real HTML text, reader overrides for fonts, color and motion.

### 1.3 Non-goals

- Branching or choice-driven fiction.
- A generic reader that opens arbitrary EPUB/PDF files.
- DRM or content protection of any kind.
- Accounts, servers or cloud sync (for now — see §11.2).

---

## 2. Users

| User | Needs |
|---|---|
| **Author** (primary: the project author; later: other writers) | Writes in Markdown, configures in TOML, runs a single CLI. May be a non-programmer. May optionally write custom CSS. |
| **Reader** | Reads the finished book as a desktop app or in a browser. Expects book-like paging, sensible controls (keyboard, mouse, touch, gamepad), and control over fonts, motion and audio. |
| **Contributor** | Developer extending the framework (open source). Needs a clean separation between compiler, bundle format and runtime. |

---

## 3. Architecture

```
 Author project (Markdown + TOML + assets)
            │
            ▼
 ┌─────────────────────────┐
 │  tome CLI  (Rust)       │  parse · validate · paginate · compute cue state · package
 └─────────────────────────┘
            │  book bundle (book.json + hashed assets)
            ▼
 ┌─────────────────────────┐
 │  Prebuilt runtime       │  Svelte web app, generic across all books
 │  (shipped with the CLI) │  loads the bundle; renders HTML; plays audio
 └─────────────────────────┘
            │
     ┌──────┴────────┐
     ▼               ▼
  Web / PWA      Electron app (Linux, Windows)
```

### 3.1 Key decisions

- **Rust is the build tool, not the runtime.** The CLI/compiler lives in `cli/` (a Cargo workspace member) and embeds the prebuilt runtime.
- **Runtime is Svelte**, chosen for its built-in transitions/animation primitives, small bundles (good for PWA) and approachability for contributors.
- **Text is rendered as HTML** for screen-reader support, native text selection, bidi/CJK shaping and browser accessibility features.
- **Electron** for desktop (not Tauri): consistent Chromium rendering across platforms is preferred over smaller binaries, as OS webviews (notably WebKitGTK on Linux) are known to cause audio/animation inconsistencies.
- **Prebuilt runtime + compiled bundle**, not per-book generated Svelte source. Authors never need a JS toolchain for writing or previewing; runtime fixes benefit every book. Per-book customization happens via theme data and custom CSS (§9).
- **Pagination happens at build time** in Rust, so every reader sees the same pages and audio/reveal cues behave deterministically.

### 3.2 Book bundle

The CLI emits a versioned bundle consumed by the runtime:

- `book.json` — metadata, chapters (each with a stable ID and content hash), pages, blocks, reveal steps, cues, references, theme and per-page precomputed audio state.
- `assets/` — content-hashed images, audio, video, fonts, custom CSS.
- All asset paths in the bundle, including URLs inside block and reference HTML, are relative to `book.json`.
- A `bundle_schema_version` field; the runtime refuses bundles with an incompatible major version, with a clear message.

### 3.3 Visual regression tests (post-MVP)

Pagination, kinetic text and theming are hard to cover with unit tests, so the sample book doubles as a visual test suite. A headless Chromium (Playwright) opens a build of the sample at fixed viewports (a wide spread and a narrow single page), with animations settled to their final frame and audio off. It screenshots a fixed list of pages, the menu, the references sidebar and the title screen, and compares them against stored images within a small tolerance. It runs as its own `make` target, not as part of `make test`, and reviewed differences are accepted by regenerating the stored images.

---

## 4. Authoring

### 4.1 Project layout (proposed)

```
mybook/
├── book.toml            # title, author, language, pagination limits, defaults
├── manuscript/
│   ├── 01-the-flood.md  # one file per chapter, ordered by filename
│   └── 02-ashes.md
├── contents.toml        # chapter index; generated, then author-editable (§13.1)
├── references.toml      # or references/*.toml for large books
├── theme/
│   ├── theme.toml
│   └── custom.css       # optional
└── assets/
    ├── audio/  images/  video/  fonts/
```

Chapter files may carry TOML front matter (`+++ … +++`) with `id`, `title`, and per-chapter overrides. If `id` is omitted it is derived from the filename slug (`01-the-flood.md` → `the-flood`). **Chapter IDs are stable identifiers** used by saves, reference gating and theme overrides; renaming one is a breaking change and `tome check` should say so where it can detect it.

### 4.2 Manuscript: extended Markdown

Base syntax is CommonMark with smart punctuation (`"…"` → “…”, `--` → –) and strikethrough. Extensions use the generic-directive style: leaf directives on their own line (`::name{…}`), containers (`:::name{…}` … `:::`) and inline directives (`:name[text]{…}`). Attributes are bare words (a keyword like `stop`, or the directive's main value) and `key=value` pairs, with optional double quotes. Times are in seconds. Directives inside fenced code blocks are left as text. The author-facing guide (`GUIDE.md`, written into every new project) is the reference; in summary:

| Purpose | Syntax |
|---|---|
| Manual page break | `::pagebreak` |
| Music | `::music{harbour volume=0.8 fade=2 delay=1}` · `::music{stop}` · `::music{stop fade=0}` (hard stop) |
| Ambient layer | `::ambient{rain}` (ID defaults to the name; `id=` to override) · `::ambient{stop rain}` · `::ambient{stop}` (all layers) |
| Sound effect | `::sfx{thunder volume=0.6 delay=1.5}` |
| Sound caption (post-MVP) | `caption="a bell tolls, uneven"` on any `::music`, `::ambient` or `::sfx` cue (§6.7) |
| Footnote (post-MVP) | `text[^ash]` with `[^ash]: The definition.` anywhere in the same chapter file |
| Reveal steps | `:::reveal{effect=typewriter duration=2 delay=0.3 easing=ease-in}` … `:::`; each block is a step unless `together`; `rest=wave` adds a resting effect |
| Resting effect | `:fx[so kind]{wave speed=2}` inline, or a `:::fx{pulse}` block; options `speed`, `amplitude`, `scale`, `min-opacity`, `gradient=name` |
| Special style | `:style[a coat like her own]{whisper}` or `:::style{handwriting}` |
| Inline illustration | `![alt](letter.svg)`; with a title (`![alt](letter.svg "Caption")`) alone in a paragraph it becomes a captioned figure |
| Illustration track | `::illustration{bridge alt="…"}` · `::illustration{none}` |
| Paper for one page | `::paper{letter}` (a named paper from the theme); front matter `paper = "letter"` for a whole chapter |
| Force / suppress a reference | `:ref[the old woman]{elara}` · `:noref[Elara]` |

A cue fires with the first block after it; cues after a chapter's last block attach to an empty anchor block at the same step as the last block. `::ambient{stop}` is expanded by the compiler into a stop for every layer playing at that point, following the audio state across chapters.

**Asset references.** A bare name like `harbour` is looked up in `assets/` (first in the folder for its kind: `audio/`, `images/` (and `video/`, for posters), `video/`, `fonts/`), and the extension may be omitted when exactly one file of the right kind matches. A path starting with `./` or `../` is relative to the file it's written in, so Markdown editors can preview images. Assets are copied into the bundle with content-hashed names.

**Footnotes (post-MVP).** Standard Markdown footnotes (`[^label]` and `[^label]: …`, as pulldown-cmark parses them). A footnote's definition never appears in the page flow and doesn't count against pagination limits. The marker renders as a small superscript button; activating it (click, tap, or focus and the advance key) opens the footnote in a popover anchored to the marker, which closes with Esc, a click outside, or a page turn. Activating a marker never advances the page. Footnotes are numbered per chapter. The bundle carries each page's footnotes as rendered HTML beside its blocks, so references inside footnote text are matched like any other text.

**Chapter front matter** (TOML between `+++` lines) may set `id`, `title`, `header_image`, `header_image_alt` and `pagination`. Without a `title`, a leading `# Heading` is the title.

Conversion from DOCX and other formats is out of scope; authors are pointed to **pandoc** (bundling pandoc is a possible future addition).

### 4.3 Pagination

- The compiler splits each chapter into pages automatically, honouring an author-set maximum per page (`max_words`, `max_characters`, `max_lines`) in `book.toml`, overridable per chapter. `max_lines` counts **author-made line breaks in the source** (each paragraph and each hard line break is one line), not rendered lines, so it is independent of the reader's font and window size.
- Breaks fall only between blocks (never mid-paragraph).
- `::pagebreak` forces a break.
- **A chapter always starts on a new page.**
- Inline illustrations do **not** count against the limit. Track illustrations are on a separate track (§7) and also don't count.
- If a page's content does not fit the viewport (large fonts, small window), the page **scrolls vertically**. Pages are never re-split at runtime.

### 4.4 CLI

| Command | Purpose |
|---|---|
| `tome new <dir>` | Scaffold a project with a sample chapter, references and the default theme. |
| `tome preview [--open]` | Serve the book locally (bound to localhost) and rebuild on every change. The page reloads after a successful rebuild and returns to the author's page even if the chapter changed; a failed build keeps serving the last good one and shows the errors over the page. |
| `tome check` | Validate everything and report friendly, file-and-line-numbered errors and warnings. |
| `tome build [--target web\|linux\|windows] [--out dir] [--bundle-only]` | Produce outputs (§10). Refuses to replace an output folder it didn't create. |
| `tome contents [--regenerate]` | Write `contents.toml` from the chapters, for the author to edit. |

`tome check` should at minimum catch:

- Malformed TOML / unknown keys (with "did you mean" suggestions).
- Missing asset files; unsupported asset formats.
- References or theme overrides gated on a chapter ID that doesn't exist.
- Reference aliases that never appear in the manuscript (warning).
- Overlapping/ambiguous aliases between references (warning).
- Missing alt text on any image (warning).
- `::music{stop}` / `::ambient{stop}` with nothing playing (warning).
- Unknown directive names, effect names or style names.
- Pages that exceed the limit because a single block is larger than the limit (warning).

Post-MVP, it should also catch:

- Files in `assets/` that nothing uses (warning), so they don't bloat the build.
- References that no page links to, counting `:ref` (warning).
- Cues that do nothing: music for the track already playing, an ambient layer started while it's already playing or stopped while it isn't, and the same sound effect cued twice on one step (warning).
- Reveal steps with no visible content, such as a step holding only cues (warning).
- Music, ambience and sound-effect cues without a `caption` (warning, like missing alt text; §6.7).
- Footnote markers without a definition, and definitions never referenced.
- A build over the size budget (§4.5).

### 4.5 Asset processing (post-MVP)

`tome build` prepares assets so every book gets consistent sound and small downloads without the author doing anything:

- **Loudness.** Each music, ambience and narration file is measured (EBU R128 integrated loudness) and the bundle records a gain that brings it to the channel's target (`[audio] loudness` in `book.toml`, with defaults per channel; sound effects are left as recorded). The runtime applies the gain before the cue's `volume` and the reader's volumes. Files aren't re-encoded, so this is lossless. `normalize = false` on a cue's asset entry, or in `[audio]`, opts out.
- **Images.** Raster images larger than they will ever be shown are downscaled (to a `max_image_size` in `book.toml`), and re-encoded to a smaller modern format at a quality the author can set. Several widths are emitted for large illustrations so the runtime picks one for the viewport (`srcset`). SVG and GIF are copied as they are, and video is untouched (no ffmpeg dependency).
- **Caching.** Processed results are cached by the source's content hash, so `tome preview` stays fast. Preview may serve originals until processing finishes.
- **Size report.** Every build prints the bundle's total size, its largest assets, and how much each chapter adds (assets first used in that chapter), so authors can see what a web reader downloads. A `[build] size_budget_mb` in `book.toml` turns an oversized build into a warning.

### 4.6 Preview tools (post-MVP)

`tome preview` adds author-only tools, absent from every built output:

- **Debug overlay**, toggled with F2: the music track and ambient layers playing, delayed cues still pending, the narration queue, the reveal step (`3 / 7`), why the page broke where it did (`limit: 212 / 220 words`, `::pagebreak`, `chapter start`), the theme overrides and paper in effect, and the chapter's ID.
- **Open in editor.** Clicking a block with a modifier (Ctrl/Cmd+click), or a button in the overlay, opens its source file at the right line. Preview builds carry a source map (block → file and line) that shipped bundles don't. The preview server runs the editor with `$VISUAL` or `$EDITOR`, or a command template from the author's `tome` config (`code -g {file}:{line}`); it only accepts the request from the page it served.
- **Follow edits.** An overlay toggle that, after a rebuild caused by a manuscript edit in another chapter, jumps to the first page that changed instead of staying on the current one.

### 4.7 Editor support (post-MVP)

`tome lsp` runs a language server over stdio for the manuscript, `book.toml`, `references.toml`, `contents.toml` and `theme.toml`:

- Diagnostics from `tome check` as the author types.
- Completion of directive names and options, effect and style names, asset names (per kind, as §4.2 resolves them), reference IDs in `:ref`, and chapter IDs in `from =` and `contents.toml`.
- Hover showing a reference's sections and their gating, an asset's details (size, duration, dimensions), or a style's definition; go-to-definition from `:ref` and `from =` to what they name.

A thin VS Code extension (published separately) starts the server and adds syntax highlighting for directives. It finds `tome` on the `PATH` or at a configured path, and tells the author how to install it if it's missing, since non-programmers are the authors who benefit most.

---

## 5. Spoiler-aware references

### 5.1 Gating model

- Gating is **per chapter only**. A section with `from = "ch-id"` becomes visible once the reader has **reached** that chapter (arrived at its first page).
- A reveal that happens mid-chapter is not exposed until the next chapter — the author simply gates it on the following chapter. This is deliberate: safer and simpler.
- Visibility is based on the reader's **furthest chapter reached**, not their current position. Re-reading chapter 3 after finishing chapter 20 shows what they actually know.
- A global **"I've already read this book"** flag unlocks everything.

### 5.2 Skipping ahead

Readers may jump to any chapter, but jumping beyond the next unread chapter shows a confirmation: *"Jumping to Chapter 14 will unlock references up to Chapter 14, which may contain spoilers."* Confirming advances the furthest-reached chapter.

### 5.3 Reference entries (TOML sketch)

```toml
[[reference]]
id    = "elara"
match = ["Elara", "the Witch of Varn"]      # aliases auto-detected in the text

  [[reference.section]]
  from  = "the-flood"
  title = "The Witch of Varn"                # displayed name is gated too
  text  = "A healer from the northern villages."
  image = "images/refs/witch.png"

  [[reference.section]]
  from = "ashes"
  mode = "append"                            # adds to the previous visible section
  text = "Keeps ravens."

  [[reference.section]]
  from  = "the-crown"
  mode  = "replace"                          # default; supersedes everything before
  title = "Elara"
  text  = "Revealed to be the exiled queen."
  match = ["the queen"]                      # aliases can also be added from a chapter onward
```

- Section `mode` is `replace` (default) or `append`. The visible entry is computed by walking unlocked sections in order: `replace` resets, `append` extends.
- **Title is gated** with the section, so the reference panel never leaks a name the reader hasn't learned.
- Text is Markdown. Each section may have **one image**, displayed in a fixed position.
- A reference appears only if at least one of its sections is unlocked.

### 5.4 Reader experience

- The page text stays **clean** — no inline links or underlines.
- A **collapsible sidebar** lists every reference matched on the current page; selecting one shows its current (gated) entry.
- Matching is automatic from aliases (whole-word, case-sensitive by default), with `:ref` / `:noref` for exceptions.
- Matching happens in the **compiler**: each page in the bundle lists the IDs of references it mentions, in order of first mention. Aliases added by a section match only in text from that section's chapter onward. The runtime only applies gating and displays the result.
- On narrow viewports the sidebar overlays the page; tapping outside it closes it rather than turning the page. Esc closes the sidebar before it opens the menu. An open entry stays open across page turns; closing the sidebar returns it to the page's list.

---

## 6. Atmosphere: audio

### 6.1 Channels

Music, ambience, sound effects and narration are separate channels, each with its own volume/mute in reader settings (plus master).

### 6.2 Cue rules

- **The text appears, the cue plays.** A cue fires immediately when the content that follows it becomes visible: a cue at the top of a page fires on arrival; a cue inside a reveal block fires when the next step is revealed. The author controls timing by placing cues after deliberate page breaks or before reveal steps.
- An author may add an explicit `delay` (seconds) to any cue. Without one, playback is immediate.
- **Music** plays until the author stops it — across pages and chapters. Changing tracks crossfades. Stopping fades out by default; `fade=0` is a hard stop. If a cue requests the track already playing, playback continues uninterrupted.
- **Ambient layers:** any number may play simultaneously, each started and stopped independently by ID.
- **Sound effects** are one-shots.

### 6.3 Direction-dependent behaviour

| Event | Moving forward | Moving back |
|---|---|---|
| Sound effects | Play | Don't play |
| Reveals | Animate step by step | Page shown fully revealed |
| Music / ambience | Applied as cued | Restored to the page's state **after a short delay** (default ~3 s, author-configurable), so a reader glancing back for a detail isn't interrupted |

The runtime derives the music/ambient state at the end of every page from the cues when the book loads (a single cheap pass), so it can restore the correct state from any position. The compiler doesn't need to emit it.

Resuming from the landing screen restores the page's audio immediately; the restore delay applies only when navigating back within a reading session.

Delayed cues that haven't fired yet when the reader leaves the page are dropped. Their lasting effect on music and ambience is already captured in the page states, so only pending sound effects are lost.

### 6.4 Narration

Voice cues (`::voice{name}`, options `volume` and `delay`) are lines of narration with their own rules:

- **One line at a time, strictly in order.** Lines never overlap. Lines revealed on the same page queue behind the one playing, in the order they appear in the text; revealing never interrupts narration. A voice cue's `delay` is a pause before the line once its turn comes (after the previous line ends), not a timer from when it was revealed, so it can't reorder lines.
- **A page turn cuts it off.** Turning the page (forward, back, or a jump) stops the current line with a short fade (150 ms) and drops the queue; moving forward, the new page's lines then start.
- **Forward, and when resuming.** Like sound effects, narration doesn't play going back or jumping to a page already read. Resuming a page (Continue on the title screen, or a `tome preview` reload) reads its narration from the first line, since that's where the reader picks the book up.
- **Turn guard.** While a line the reader can hear plays, is queued, or is about to start (a pending `delay`), the first attempt to turn the page (advance on a fully revealed page, or back) only shows "Narration is still playing — press again to turn the page." A second attempt within 4 seconds turns it. Revealing a step never asks, nor do deliberate jumps from the menu.
- **Readers can turn it off.** With the Narration channel (or all sound) muted or at zero, nothing is heard and the guard is off. Muting mid-line silences the line without ending it.
- **Highlighting.** While a line is heard, the paragraph it narrates (the block after its cue) is tinted, or, given word timings, the word being spoken is highlighted. Timings come from a timing file beside the recording (same name; `.vtt`, `.srt` or `.json`: plain segments, Whisper segments, or a word list) or named with `timing=`. The compiler aligns the timing's words to the paragraph's (normalized, with a short look-ahead to re-sync over small differences), spreads phrase times across their words by length, and emits a start time per word, with null for words after the narration ends; it warns when under 60% of the timed words are found in the paragraph. The runtime paints the word with a CSS Custom Highlight, polling playback each frame. Readers can turn it off (Settings → Reading); silent narration isn't highlighted.
- **Ducking.** While narration is heard, music and ambience dip to the book's `ducking` level (`[audio]` in `book.toml`, default 0.35, `1` for none) over 300 ms, and recover over 800 ms when narration ends. They stay down through pauses between queued lines and across a page turn onto more narration, so they don't pump. Sound effects aren't ducked. The dip is a separate gain after the reader's own volumes, so it never changes their settings. Silent narration doesn't duck, and muting narration mid-line brings the music straight back.
- **Auto mode follows the narration**, heard or not: it waits for each line before advancing. With narration off, lines pass silently, each lasting as long as its recording (read from the file's metadata), since a line's length is a good guide to how long its text takes to read.
- A line that fails to load counts as finished, so the queue and the guard never get stuck.

**Read aloud (post-MVP).** For readers who want the book spoken but have no recorded narration, a *Read aloud* setting (off by default) speaks the text with the system's speech synthesis (Web Speech API). It reads each block as it's revealed, and follows the narration rules above: one block at a time in order, cut off by a page turn, forward only, with the turn guard, ducking, auto-mode waiting and word highlighting (from the synthesizer's word boundary events). Recorded narration always wins: a page with voice cues plays those, and Read aloud covers only pages without them. Hidden text such as the plain copies of per-character effects is read once, and footnotes aren't read. The reader picks a voice and rate from what the system offers; with no voices available, the setting explains that and stays off.

### 6.5 Playback

All sound (music, ambience, effects and narration) plays from decoded buffers through one Web Audio graph, never from `<audio>` elements. Browsers that block autoplay (Brave does by default) refuse an element's `play()` unless a click comes right before it, which a queued narration line, a page's music or a preview reload never has; a resumed audio context plays freely. Buffers also loop music and ambience without gaps, and give narration highlighting an exact clock. Decoded music is large (a few minutes of stereo is tens of megabytes), so decoded audio is cached within a 256 MB budget, least recently used first, never evicting a track that's playing; the current and next pages' sounds are decoded ahead.

### 6.6 Web autoplay

Browsers block audio before user interaction. The landing screen's *Start* / *Continue* action serves as the required gesture; no audio is attempted before it.

### 6.7 Sound captions (post-MVP)

Deaf and hard-of-hearing readers otherwise miss part of the book, so music, ambience and sound-effect cues may carry a `caption` (§4.2), written in the book's voice: *a bell tolls, uneven*. With *Captions* on (Settings → Sound, off by default), the runtime shows them in a small caption strip at the bottom of the page, in an `aria-live="polite"` region:

- A **sound effect** caption appears when the effect plays and stays for the effect's length, at least 3 seconds.
- A **music** or **ambience** caption appears when the track or layer starts and fades after a few seconds. The strip keeps a compact, dim line of what is still sounding (*rain · harbour music*), so a reader arriving on a page knows the soundscape; it's restored with the audio state when going back or resuming.
- Captions follow the direction rules: sound effects aren't captioned going back, because they don't play.
- Captions show whether or not the channel is muted, since a reader who can't hear it may well have muted it.
- Narration isn't captioned: its text is already the page, and narration highlighting shows where it is.

Theme tokens style the strip (`caption-bg`, `caption-text`), and the strip never covers the text column on a spread.

---

## 7. Illustrations

Three kinds:

1. **Inline** — between paragraphs in the text flow.
2. **Chapter header art.**
3. **Illustration track** — major illustrations running parallel to the text.

### 7.1 Illustration track

- `::illustration` sets the current track image; it persists across pages and chapters until changed. The author can also clear the track (e.g. `::illustration{none}`), returning to a single text column.
- **Wide viewport:** shown side-by-side with the text as a two-page spread.
- **Narrow viewport** (mobile or a small window): the reader toggles between text and image. When moving forward onto a page that introduces a new track illustration, the app **lingers on the image** for a few seconds (author-configurable default) before fading in the text. The advance action skips the linger.
- Layout is decided by **viewport size/aspect ratio**, never by device type. The runtime exposes the current layout as `data-layout="spread" | "single"` for themes and custom CSS.
- While the illustration is showing on a narrow viewport (lingering, or toggled with the Illustration button / `I`), the advance action returns to the text rather than revealing a step or turning the page.
- Audio cues on a page fire on arrival, even while its illustration lingers; the linger is not a reveal step.

### 7.2 Accessibility

Alt text is expected on every image; `tome check` warns when missing.

Zoom, full-screen view and timed illustration reveals are post-MVP.

---

## 8. Kinetic & special text

There are two distinct kinds of animation, which can be combined on the same text:

| Kind | When it runs | Purpose |
|---|---|---|
| **Entrance** | Once, as the text is revealed (forward only) | Pacing: controls *when* text appears |
| **Resting** | Continuously while the text is on screen | Emphasis/tone: e.g. a sarcastic line of dialogue that stays wavy on the page |

Both apply to a block (paragraph) or an inline span. Every effect has sensible defaults and author-tunable parameters (duration, speed, amplitude, easing, delay).

### 8.1 Entrance animations (reveals)

- MVP effects: **fade-in** (the essential effect and the default), **slide** and **typewriter**.
- Entrance effects are forward-only. **There are no fade-outs**: text never disappears once shown.
- Revisiting a page (going back) shows it fully revealed, with no entrance animation.

### 8.2 Resting animations

MVP set:

- **Pulse** — rhythmic opacity/intensity pulsing.
- **Breathe** — gentle growing and shrinking.
- **Tremble** — small, jittery shaking.
- **Gradient shift** — color cycling along an **author-defined gradient** (declared in the theme).
- **Wave** — characters rise and fall in a travelling wave.

Resting animations persist while the text is on screen and are present regardless of navigation direction, since they are part of how the text looks rather than when it appears. Illustrative syntax: `:fx[Oh, *wonderful*.]{rest="wave"}`, or `:::reveal{effect="fade" rest="tremble"}` on a block.

Per-character effects (wave, tremble) split text into spans for rendering; the runtime must keep the underlying text intact for screen readers and text selection (e.g. a visually hidden plain copy, with the animated spans `aria-hidden`).

**Bundle contract.** Entrance effects are the block's `reveal` (`effect`, `duration_ms`, `delay_ms`, `easing`). For typewriter, `duration_ms` is the time to type the whole block, defaulting to 35 ms per character. Resting effects and named styles are attributes in the block HTML (`data-tome-rest="wave"`, `data-tome-style="handwriting"`), on a paragraph for a whole block or on a span inline. Parameters are CSS custom properties on the element (`--tome-rest-duration`, `--tome-rest-amplitude`, `--tome-rest-scale`, `--tome-rest-min-opacity`, `--tome-rest-gradient`). The runtime does the character splitting when the book loads.

### 8.3 The advance action

There is **one "advance" action**. Each press reveals the next step; when the page is fully revealed, the same action turns the page. If an animation is in progress, advance completes it instantly rather than skipping further.

**Auto mode** (optional, off by default): advance fires on a fixed timer, regardless of text length. Deliberately simple. Any manual advance restarts the timer; it pauses while the menu or references are open.

### 8.4 Special text

Named styles (e.g. `handwriting`, `whisper`) are defined in the theme and applied by directives. A named style may bundle typography with a resting animation.

Readers can disable all special text, which renders it in the normal body style and disables **both** entrance and resting animations (content appears static, in its final state). Gradient shift is also disabled when the reader turns off color accents. The runtime applies these settings as `data-special-text="off"` and `data-accents="off"` on the root element, so themes and custom CSS can respond to them too.

---

## 9. Theming

### 9.1 Theme TOML

The base theme defines:

- Colors (as named tokens, including accent tokens), fonts (bundled font files) and default body typography.
- **Landing screen:** cover art, title treatment, background, menu arrangement, optional music.
- **Backgrounds:** image, GIF or pre-rendered video, per screen (landing, reading). Real-time 3D is out of scope; pre-rendered video is the recommended substitute.
- **Decoration:** page frames, textures, chapter-heading ornaments, drop caps.
- **UI chrome:** buttons, sidebar, menu, icons.
- Special-text style definitions.

**Bundle contract.** The compiler turns `theme.toml` into the bundle's `theme`:

- `tokens`: values for any `--tome-*` custom property, named without the prefix (`accent`, `page-bg`, `font-body`, …). Named gradients for the gradient effect are tokens too (`gradient-dawn`), referenced as `var(--tome-gradient-dawn)`.
- `fonts`: font files (`family`, `src`, optional `weight` / `style`), declared as `@font-face`.
- `styles`: named special styles as CSS declarations for `[data-tome-style="name"]`.
- `decoration`: `page_texture`, `page_frame` (`src`, `slice`, `width`, `repeat`, drawn as a border image), `chapter_ornament` (above titles of chapters without their own header art), `drop_caps`.
- `backgrounds`: `landing` and `reading`, each an image, GIF or video (`kind` inferred from the extension), a plain `color`, or both, with optional `poster`, `fit`, `position`, `opacity`.
- `papers`: named papers (`color`, `texture` or `none`, `grain` 0–1) that pages select with `::paper{name}`; unset fields keep the book's paper.
- The book's own paper is the `paper` token (defaulting to `page-bg`), `decoration.page_texture` and `decoration.paper_grain`. Grain is generated by the runtime (SVG noise), so it needs no image.
- `landing`: `cover`, `title_image`, `layout` (`centered` / `split`), `menu` (`stacked` / `inline`), `music`.

The runtime writes the theme into one generated stylesheet after its own CSS. Values that could escape a declaration (`; { } < >`) are rejected with a warning. Relative `url(...)` values resolve against `book.json`. Landing music starts on the first interaction with the title screen that isn't activating a button, since browsers block audio before a gesture, and fades out when the book's own audio begins.

**Book frame.** With a reading background showing, the reading layout becomes the book as an object: the text page (and, in a spread, the illustration page facing it) are opaque sheets of paper a page wide, with a soft shadow and a fold shadow at the spine, lying on the background, which shows around them. The runtime flags this with `data-backdrop="on"` on the root element. Without a reading background, the page fills the window as before. Sheet size, margin, shadow and corner radius are tokens (`sheet-width`, `book-margin`, `sheet-shadow`, `sheet-radius`).

### 9.2 Progressive themes

`[[override]] from = "chapter-id"` blocks apply partial theme changes from a chapter onward (e.g. a green accent from chapter 6). Overrides merge per token, style, background and decoration field, in chapter order; `null` clears a background or decoration. Overrides follow the reader's **current position**, so the look matches the part of the story being read. As a general rule, **every chapter-based setting except spoiler gating follows the current chapter**; spoiler gating follows the furthest chapter reached (§5.1). The landing screen uses the theme at the reader's saved position (base theme for a new reader).

### 9.3 Custom CSS

`theme/custom.css` is loaded after the generated theme. Besides the tokens, the runtime exposes state as attributes for author CSS: `data-chapter`, `data-layout` and `data-paper` on `.tome-reading`, and `data-special-text`, `data-accents`, `data-drop-caps`, `data-chapter-ornament`, `data-page-frame` and `data-backdrop` on the root element. The runtime exposes a **documented, versioned theming contract** — CSS custom properties (`--accent`, `--page-bg`, …) and stable class names — treated as public API under semver. Author CSS should use the color tokens so the reader's accent toggle keeps working. (Utility-class frameworks such as Tailwind are not used in the runtime markup, to keep the contract stable.)

### 9.4 Defaults

One polished default theme ships with the MVP. A book with no theme configuration must look good.

---

## 10. Platforms and outputs

| Target | MVP output |
|---|---|
| Web / PWA | `dist/web/`: a static folder. Hosting is the author's responsibility. (Offline support via a service worker is still to do.) |
| Linux | `dist/linux-x64/`: the app folder, ready to run, plus a `.tar.gz` of it to distribute |
| Windows | `dist/windows-x64/`: the app folder with `<Title>.exe`, plus a `.zip` of it to distribute |
| macOS | Deferred (no test hardware) |
| Mobile | Deferred; the PWA works in mobile browsers in the meantime |

- **No Node needed.** Desktop builds use Electron's official prebuilt release (pinned version, SHA-256 checksums built into `tome`), downloaded once and cached in the user's cache folder. The book's web build, a small main process and a `package.json` replace Electron's default app, and the executable is renamed after the book. Because this is pure file assembly, Windows builds can be made on Linux. `--target all` builds every target; `--arch arm64` builds for ARM.
- **The desktop shell** serves the book from a custom secure origin (`app://book/`) rather than `file://`, so `fetch`, `localStorage` and media byte ranges work as on the web. Each book stores its data in its own folder (`tome-books/<book id>` in the platform's app-data directory). Audio may start without a click (so a controller alone can start the book). It **opens full screen by default** and remembers if the reader switches to a window (stored by the desktop shell in `window.json`, since the window needs it before the page loads). Readers switch with the *Full screen* setting under Settings → Display, or F11 / Alt+Enter; the setting follows however it changes. The title screen and the menu have an **Exit** button. These desktop-only controls reach the shell through a preload bridge exposing only quit and full screen, and don't appear in the web build. There's no menu bar. External links open in the reader's browser. One instance runs at a time.
- Installers (`.msi`, `.deb`, AppImage, Flatpak) are post-MVP.
- Not yet done: a custom icon and version information on the Windows `.exe` (setting them needs `rcedit`, which runs under Wine on Linux), and code signing.
- Known risk: on Linux distributions that restrict unprivileged user namespaces (e.g. Ubuntu 24.04+ with AppArmor), Electron's sandbox can't start from a portable folder unless `chrome-sandbox` is made setuid root. AppImage or `.deb` packaging is the usual fix.

---

## 11. Reader state

### 11.1 Saved state

One state per install (no profiles):

- Current position: chapter ID + page index + the chapter's content hash.
- Furthest chapter reached.
- "Already read" flag.
- Settings (§12).
- Bookmarks (page-level, optional label).
- Highlights (text ranges anchored by chapter ID, block ID, offsets and a text snippet), each with an optional note.
- Reading pace for the time-left estimate (post-MVP, §13.3).

Saved automatically. Electron: a JSON file in the app data directory. Web: IndexedDB/localStorage scoped to the book. The web build *should* offer export/import of the save file, since browser storage can be wiped.

### 11.2 Storage abstraction

All persistence goes through a storage-adapter interface so that a remote/sync backend (including Steam Cloud) can be added later without touching the rest of the runtime.

### 11.3 Book updates

- Saves reference stable chapter IDs, never chapter numbers.
- MVP: if a chapter's content hash changed since the save, the reader's position **resets to the start of that chapter**. Bookmarks fall back to chapter start likewise.
- Highlights in a chapter whose content changed, or whose text no longer matches, are hidden but retained in the save data.
- Highlights are painted with the CSS Custom Highlight API, so the page DOM is never modified; per-character effects have already restructured it.
- Post-MVP: robust re-anchoring of bookmarks/highlights, with orphans surfaced to the reader.

### 11.4 Quote cards (post-MVP)

A highlight (from the Highlights list, or right after making one) can be saved as a **quote card**: an image of the quote with the book's title, author and chapter title, drawn in the book's fonts, colors and paper at the reader's current theme. The runtime renders it to a canvas and saves a PNG (a download on the web; a save dialog on desktop, which adds a save-file call to the preload bridge). The highlight's note isn't included. It's how readers share the book, and authors control it in `book.toml`: `[sharing] quote_cards = false` turns it off, and `max_quote_length` (default 280 characters) caps how much text one card can carry, so cards can't be used to copy out whole pages. The theme may set a card background and layout (`quote_card` in `theme.toml`); by default, cards use the reading paper and the chapter ornament.

---

## 12. Reader settings & accessibility

Authors set the defaults; readers can override **only settings that serve accessibility or comfort**. There is no reader-selectable dark mode or alternative palette: this is not a generic ereader, and the author's theme is part of the work.

- **Body font** (from bundled choices including the author's default) and **font size**. Post-MVP, the bundled choices include a typeface designed for legibility (such as Atkinson Hyperlegible or OpenDyslexic, both under the SIL Open Font License).
- **Line spacing** and **text width** (post-MVP): spacing from tight to loose around the theme's value, and width as narrow, theme default or wide. Both apply to body text only; pagination doesn't change, and longer pages scroll as usual (§4.3).
- **High contrast** (post-MVP): replaces the text and paper colors with a maximum-contrast pair that keeps the theme's polarity (dark on light for a light paper, light on dark for a dark one), drops paper textures and grain, turns accents and backgrounds into solid colors, and thickens focus outlines. It defaults to on when the OS reports `prefers-contrast: more`. This is an accessibility override, not an alternative palette, so it doesn't conflict with the rule above. The runtime sets `data-contrast="high"` on the root element for themes and custom CSS.
- **Color accents** on/off.
- **Special text** on/off (special typography and animations). Defaults to off when the OS reports `prefers-reduced-motion`.
- **Background video/GIF animation** follows the special-text/motion setting (falls back to a static poster frame).
- **Music, ambience, sound effects:** separate volume/mute.
- **Captions** for sound (post-MVP, §6.7) and **Read aloud** (post-MVP, §6.4).
- **Time left** in the status bar on/off (post-MVP, §13.3).
- **Auto mode** on/off and interval.
- **Already read** flag.
- **Reset reading progress** (with a confirmation): forgets the position, the furthest chapter (relocking references) and the already-read flag, and returns to the title screen. Bookmarks and highlights are deleted only if the reader ticks *Also delete bookmarks and highlights*; other settings stay. Jumping to a kept bookmark or highlight past the next unread chapter shows the same spoiler warning as the chapter list.

Text is semantic HTML; all controls are keyboard- and screen-reader-accessible.

---

## 13. Input & navigation

| Action | Keyboard | Mouse | Touch | Gamepad |
|---|---|---|---|---|
| Advance | → / Space / Enter | Click (advance zone) | Tap / swipe left | A |
| Back | ← / Backspace | Click (back zone) | Swipe right | B |
| Menu | Esc | Menu button (status bar) | Menu button (status bar) | Start |
| Bookmark page | B | Menu → Bookmarks | Menu → Bookmarks | — |
| Highlight selection | H | Highlight button | Highlight button | — |
| Toggle reference sidebar | R | References button (status bar) | References button (status bar) | Y |
| Search (post-MVP) | / or Ctrl+F | Menu → Search | Menu → Search | — |
| Toggle text/illustration (narrow) | I | Toggle button | Toggle button | X |
| Turn page | → / ← | — | — | RB / LB |
| Scroll a long page | ↑ / ↓ | Wheel | Drag | D-pad / left stick up and down |

For RTL books, swipe and arrow directions mirror (future, §14).

**Gamepad.** The W3C standard layout (which Steam Input provides) is polled every animation frame while a controller is connected; the left stick acts as a D-pad, and held directions repeat. In the menu, the references sidebar and on the title screen, the D-pad moves focus (left and right adjust sliders), A activates, and B backs out of a submenu or entry before closing. Start or Back/View opens and closes the menu.

**Navigation semantics:** arriving via *advance* is "forward"; via *back* is "backward". A jump (chapter list, bookmark) to a page the reader has **not** seen behaves as forward; to a page they **have** seen behaves as backward (§6.3).

**Menu:** Resume · Chapters · Bookmarks · Highlights · References · Search (post-MVP) · Settings · Title screen. A modal dialog opened with Esc or the Menu button in the status bar; Esc closes it. Leaving the book goes through Title screen, since Esc now opens the menu.

### 13.1 Chapter index

The chapter list works like the contents page of a paperback: all chapter titles are shown, read or not. The bundle carries it as `contents` (chapter entries with optional title overrides, plus headings); without it the runtime lists every chapter in order. It is **generated automatically** from the manuscript into `contents.toml`, which the author may then edit by hand (rename entries, hide entries, add part/section headings). Once the file exists, the CLI never overwrites it; `tome check` warns when it is out of sync with the manuscript (missing or unknown chapter IDs), and a `tome contents --regenerate` command (name provisional) rebuilds it on request.

### 13.2 Search (post-MVP)

Readers can search the text they've read, to find a half-remembered line. Search covers only chapters up to the **furthest chapter reached** (everything with the already-read flag), so it gates by the same rule as references (§5.1) and can never reveal later text. It is case- and accent-insensitive and matches whole words or phrases. Results are grouped by chapter, each with a short snippet around the match. Choosing one jumps to that page, with the jump rules from §13 (the page has been seen, so it behaves as backward), and briefly marks the match with a CSS Custom Highlight. Footnotes are searched too, and reference entries are not, since they have their own list. The runtime builds the index from the bundle's text when the book loads, or the first time Search opens on a long book, so the bundle doesn't grow. Opened from the menu, or with `/` or Ctrl+F.

### 13.3 Time left (post-MVP)

The status bar can show how long the rest of the chapter will take: *About 9 minutes left in this chapter*. The bundle carries each page's word count. The estimate divides the remaining words by the reader's own pace, measured from time spent on fully revealed pages (ignoring very short and very long ones, such as a skipped page or a reader who walked away), and starts from 230 words per minute until there's enough data. Pace is stored with the reader's saved state. The display rounds to whole minutes, and says *Less than a minute* at the end. It's on by default and can be turned off in Settings → Reading.

---

## 14. Internationalization

English-only MVP, but **i18n-ready**:

- Book language declared in `book.toml` (sets HTML `lang`/`dir`).
- All runtime UI strings come from a translatable string table; authors can override strings to match the book's voice (e.g. "References" → "Lore").
- The runtime uses CSS logical properties throughout, so RTL layout is possible later without a rewrite.
- Text shaping, bidi and CJK line-breaking are left to the browser.

Future: multi-language books, per-language references/aliases, RTL page direction.

---

## 15. Licensing & distribution

- Framework (CLI and runtime): **Apache-2.0** (intended).
- **Book content remains entirely the author's**, under whatever copyright/license they choose. The framework's license places no requirements on books built with it, and this is stated explicitly in the project README and LICENSE notes. Bundled default fonts/assets must use licenses compatible with commercial redistribution.
- No DRM, ever. Authors may sell books (Steam is a future target) or give them away.

---

## 16. Scope

### 16.1 MVP

- Extended-Markdown manuscript, TOML config, CLI (`new`, `preview`, `check`, `build`).
- Build-time pagination with author limits, manual breaks, chapter-start pages, in-page scrolling.
- Chapter-gated references: replace/append sections, gated titles, one image per section, auto-detected links in a collapsible sidebar, already-read flag, skip-ahead warning.
- Audio: music with fades/crossfades, one-shot SFX, unlimited ambient layers, direction-aware behaviour.
- Reveal steps with fade-in, slide and typewriter entrances; resting animations (pulse, breathe, tremble, gradient shift, wave); unified advance action; optional auto mode; special text styles.
- Illustration track plus inline and chapter-header art.
- Theme TOML with landing screen, image/GIF/video backgrounds, chapter-gated overrides, custom CSS, one default theme.
- Reader settings as in §12; bookmarks and highlights (with optional notes); local saves.
- Keyboard, mouse, touch and gamepad input.
- Web/PWA build; standalone Electron builds for Linux and Windows.

### 16.2 Later

Sound captions (§6.7) · read aloud with speech synthesis (§6.4) · footnotes (§4.2) · spoiler-safe search (§13.2) · time left in chapter (§13.3) · line spacing, text width, a legibility font and high contrast (§12) · quote cards (§11.4) · loudness normalization, image optimization and a size report (§4.5) · stricter `tome check` (§4.4) · preview debug overlay, open in editor and follow edits (§4.6) · language server and VS Code extension (§4.7) · visual regression tests (§3.3).

Also: hidden-depth content (in-world documents, annotations) · "previously on" recaps · codex/glossary screen with "new" markers · maps, timelines, family trees · illustration zoom/interaction · highlight export · sync server / Steam Cloud · installers and distro packages · Steam integration · free samples / partial builds · GUI authoring companion · bundled pandoc · macOS · native mobile apps · multi-language books · robust bookmark/highlight migration.

---

## 17. Open questions

1. **Final name** for the framework/CLI — deferred.
2. **Image encoder** for asset processing (§4.5): AVIF (smallest; pure-Rust encoders are slow) or WebP (fast; good lossy encoding needs libwebp, a C dependency). Either way, every output's Chromium supports it.
3. **Speech synthesis on Linux** (§6.4): Electron on Linux often has no voices unless speech-dispatcher is installed. Should Read aloud be hidden there when no voices are found, or should the desktop build bundle an offline voice?
4. **Captions default** (§6.7): off by default, or on for books whose cues all carry captions?
