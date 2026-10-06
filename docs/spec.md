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

- **Rust is the build tool, not the runtime.** The existing crate (`interactive_storybook`) becomes the CLI/compiler.
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

Base syntax is CommonMark. Extensions use the generic-directive style (`:inline`, `::leaf`, `:::container`) so that plain Markdown editors and pandoc degrade gracefully. **All syntax below is illustrative**; exact grammar is to be finalized during implementation.

| Purpose | Sketch |
|---|---|
| Manual page break | `::pagebreak` |
| Music | `::music{track="storm" fade=2}` · `::music{stop}` · `::music{stop fade=0}` (hard stop) · optional `delay=1.5` on any cue |
| Ambient layer | `::ambient{track="rain" id="rain"}` · `::ambient{stop="rain"}` |
| Sound effect | `::sfx{sound="thunder"}` |
| Reveal block | `:::reveal{effect="fade"}` … `:::` — each paragraph inside is one step |
| Special text | `:::style{name="handwriting"}` … `:::` or inline `:style[text]{name="whisper"}` |
| Inline illustration | standard `![alt text](images/map.png)` |
| Chapter header art | front matter `header_image = "…"` |
| Illustration track | `::illustration{src="plates/bridge.png" alt="…"}` |
| Force / suppress a reference | `:ref[the old woman]{id="elara"}` · `:noref[Elara]` |

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
| `tome preview` | Build and serve the book in a browser with live reload on file changes. |
| `tome check` | Validate everything and report friendly, file-and-line-numbered errors and warnings. |
| `tome build [--target web\|linux\|windows\|all]` | Produce outputs (§10). |

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

Music, ambience and sound effects are separate channels, each with its own volume/mute in reader settings (plus master). A voice channel is reserved for the future voice-track feature.

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

### 6.4 Web autoplay

Browsers block audio before user interaction. The landing screen's *Start* / *Continue* action serves as the required gesture; no audio is attempted before it.

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
- `backgrounds`: `landing` and `reading`, each an image, GIF or video (`kind` inferred from the extension) with optional `poster`, `fit`, `position`, `opacity`.
- `landing`: `cover`, `title_image`, `layout` (`centered` / `split`), `menu` (`stacked` / `inline`), `music`.

The runtime writes the theme into one generated stylesheet after its own CSS. Values that could escape a declaration (`; { } < >`) are rejected with a warning. Relative `url(...)` values resolve against `book.json`. Landing music starts on the first interaction with the title screen that isn't activating a button, since browsers block audio before a gesture, and fades out when the book's own audio begins.

### 9.2 Progressive themes

`[[override]] from = "chapter-id"` blocks apply partial theme changes from a chapter onward (e.g. a green accent from chapter 6). Overrides merge per token, style, background and decoration field, in chapter order; `null` clears a background or decoration. Overrides follow the reader's **current position**, so the look matches the part of the story being read. As a general rule, **every chapter-based setting except spoiler gating follows the current chapter**; spoiler gating follows the furthest chapter reached (§5.1). The landing screen uses the theme at the reader's saved position (base theme for a new reader).

### 9.3 Custom CSS

`theme/custom.css` is loaded after the generated theme. Besides the tokens, the runtime exposes state as attributes for author CSS: `data-chapter` and `data-layout` on `.tome-reading`, and `data-special-text`, `data-accents`, `data-drop-caps`, `data-chapter-ornament` and `data-page-frame` on the root element. The runtime exposes a **documented, versioned theming contract** — CSS custom properties (`--accent`, `--page-bg`, …) and stable class names — treated as public API under semver. Author CSS should use the color tokens so the reader's accent toggle keeps working. (Utility-class frameworks such as Tailwind are not used in the runtime markup, to keep the contract stable.)

### 9.4 Defaults

One polished default theme ships with the MVP. A book with no theme configuration must look good.

---

## 10. Platforms and outputs

| Target | MVP output |
|---|---|
| Web / PWA | Static folder, offline-capable via service worker. Hosting is the author's responsibility. |
| Linux | Standalone Electron executable |
| Windows | Standalone Electron executable (portable `.exe`) |
| macOS | Deferred (no test hardware) |
| Mobile | Deferred; the PWA works in mobile browsers in the meantime |

- The CLI may download Node/Electron on first desktop build; offline desktop builds are not an MVP requirement.
- Installers (`.msi`, `.deb`, AppImage, Flatpak) are post-MVP.
- Implementation risk to verify early: producing the Windows build from Linux (exe metadata/icon editing tools often need Wine).

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

Saved automatically. Electron: a JSON file in the app data directory. Web: IndexedDB/localStorage scoped to the book. The web build *should* offer export/import of the save file, since browser storage can be wiped.

### 11.2 Storage abstraction

All persistence goes through a storage-adapter interface so that a remote/sync backend (including Steam Cloud) can be added later without touching the rest of the runtime.

### 11.3 Book updates

- Saves reference stable chapter IDs, never chapter numbers.
- MVP: if a chapter's content hash changed since the save, the reader's position **resets to the start of that chapter**. Bookmarks fall back to chapter start likewise.
- Highlights in a chapter whose content changed, or whose text no longer matches, are hidden but retained in the save data.
- Highlights are painted with the CSS Custom Highlight API, so the page DOM is never modified; per-character effects have already restructured it.
- Post-MVP: robust re-anchoring of bookmarks/highlights, with orphans surfaced to the reader.

---

## 12. Reader settings & accessibility

Authors set the defaults; readers can override **only settings that serve accessibility or comfort**. There is no reader-selectable dark mode or alternative palette: this is not a generic ereader, and the author's theme is part of the work.

- **Body font** (from bundled choices including the author's default) and **font size**.
- **Color accents** on/off.
- **Special text** on/off (special typography and animations). Defaults to off when the OS reports `prefers-reduced-motion`.
- **Background video/GIF animation** follows the special-text/motion setting (falls back to a static poster frame).
- **Music, ambience, sound effects:** separate volume/mute.
- **Auto mode** on/off and interval.
- **Already read** flag.

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
| Toggle text/illustration (narrow) | I | Toggle button | Toggle button | X |
| Turn page | → / ← | — | — | RB / LB |
| Scroll a long page | ↑ / ↓ | Wheel | Drag | D-pad / left stick up and down |

For RTL books, swipe and arrow directions mirror (future, §14).

**Gamepad.** The W3C standard layout (which Steam Input provides) is polled every animation frame while a controller is connected; the left stick acts as a D-pad, and held directions repeat. In the menu, the references sidebar and on the title screen, the D-pad moves focus (left and right adjust sliders), A activates, and B backs out of a submenu or entry before closing. Start or Back/View opens and closes the menu.

**Navigation semantics:** arriving via *advance* is "forward"; via *back* is "backward". A jump (chapter list, bookmark) to a page the reader has **not** seen behaves as forward; to a page they **have** seen behaves as backward (§6.3).

**Menu:** Resume · Chapters · Bookmarks · Highlights · References · Settings · Title screen. A modal dialog opened with Esc or the Menu button in the status bar; Esc closes it. Leaving the book goes through Title screen, since Esc now opens the menu.

### 13.1 Chapter index

The chapter list works like the contents page of a paperback: all chapter titles are shown, read or not. The bundle carries it as `contents` (chapter entries with optional title overrides, plus headings); without it the runtime lists every chapter in order. It is **generated automatically** from the manuscript into `contents.toml`, which the author may then edit by hand (rename entries, hide entries, add part/section headings). Once the file exists, the CLI never overwrites it; `tome check` warns when it is out of sync with the manuscript (missing or unknown chapter IDs), and a `tome contents --regenerate` command (name provisional) rebuilds it on request.

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

Voice tracks · hidden-depth content (in-world documents, annotations) · "previously on" recaps · codex/glossary screen with "new" markers · maps, timelines, family trees · illustration zoom/interaction · highlight export · sync server / Steam Cloud · installers and distro packages · Steam integration · free samples / partial builds · GUI authoring companion · bundled pandoc · macOS · native mobile apps · multi-language books · robust bookmark/highlight migration.

---

## 17. Open questions

1. **Final name** for the framework/CLI — deferred.
2. **Exact directive grammar** (and parser choice in Rust) — to be determined incrementally during implementation. Syntax in this document is illustrative only.
