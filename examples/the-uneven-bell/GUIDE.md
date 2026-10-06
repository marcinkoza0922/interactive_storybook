# Writing your book

Your book is a folder of plain text files:

| File | What it's for |
|---|---|
| `book.toml` | Title, author, language, page length |
| `manuscript/*.md` | Chapters, one per file, in file-name order (`01-…`, `02-…`) |
| `references.toml` | Entries readers can look up, unlocked chapter by chapter |
| `theme/theme.toml` | Colors, fonts, backgrounds, the title screen |
| `theme/custom.css` | Optional extra styling |
| `contents.toml` | Optional: the chapter list, as you want readers to see it |
| `assets/` | Sound, images, video and fonts |

Commands, run from this folder:

- `tome preview` — read the book in your browser; it updates as you save.
- `tome check` — list problems without building.
- `tome build` — make the finished book for the web, in `dist/web/`.
- `tome build --target linux` (or `windows`, or `all`) — make a desktop app. The first time,
  this downloads Electron (about 115 MB), which is then reused.
- `tome contents` — write `contents.toml` to edit the chapter list.

## Chapters

Each chapter starts with its title as a heading. Its ID (used by references and saved
progress) comes from the file name: `01-the-harbour.md` is `the-harbour`. Don't rename
a chapter's file once readers have it, or give it a fixed `id` first:

```
+++
id = "the-harbour"
title = "The Harbour"            # instead of a # heading
header_image = "ornament.svg"    # art above the title
header_image_alt = ""
pagination = { max_words = 150 } # this chapter only
+++
```

Write in ordinary Markdown. Quotes and dashes become typographic (`"` → “ ”, `--` → –).
A line of `***` is a scene break.

## Pages

Pages fill up to the limits in `book.toml` and break between paragraphs. Force a break with:

```
::pagebreak
```

A page that doesn't fit the screen scrolls.

## Sound

Cues go on their own line. They play when the text after them appears. Name a file in
`assets/audio/` without its extension:

```
::music{harbour}                  start (or crossfade to) music; it keeps playing
::music{harbour volume=0.6 fade=3}
::music{stop}                     fade out (fade=0 for a hard stop)
::ambient{rain}                   a looping layer; add as many as you like
::ambient{stop rain}              stop one layer
::ambient{stop}                   stop them all
::sfx{thunder delay=1.5}          a one-off sound, only when reading forward
```

Times are in seconds. Volume runs from 0 to 1.

## Illustrations

```
![A ferry in a grey harbour](harbour.png)            in the text
![The old letter](letter.svg "The letter")           with a caption
::illustration{bridge alt="A stone bridge in fog"}   beside the text, until changed
::illustration{none}                                 back to text only
```

Always describe images for readers who can't see them.

## Revealing text and effects

Paragraphs inside a reveal appear one at a time as the reader advances:

```
:::reveal
Inside, the air was cold and still.

On the table sat a single candle.
:::

:::reveal{effect=typewriter delay=0.3}
You came after all.
:::
```

Effects: `fade` (the default), `slide`, `typewriter`. Options: `duration`, `delay`,
`easing`, and `together` to reveal the whole block at once.

Effects that keep moving while the text is on screen:

```
“Oh, :fx[how wonderful]{wave},” she said.
:fx[something moved]{tremble}
:::fx{pulse speed=3}
A whole paragraph.
:::
```

Effects: `pulse`, `breathe`, `tremble`, `gradient`, `wave`. Options: `speed` (seconds),
`amplitude`, `scale`, `min-opacity`, and `gradient=name` for a gradient defined in the theme.

Special styles: `:style[a coat like her own]{whisper}`, or a `:::style{handwriting}` block.
`handwriting` and `whisper` are built in; the theme can change them or add more.

Readers can turn all of this off in Settings.

## References

In `references.toml`, each entry has names to look for and sections that unlock as the
reader reaches each chapter. Later sections replace earlier ones unless `mode = "append"`:

```toml
[[reference]]
id = "elara"
match = ["the Witch of Varn"]

[[reference.section]]
from = "the-harbour"
title = "The Witch of Varn"
text = "A healer from the northern villages."
image = "witch.png"
image_alt = "A woman in a grey cloak"

[[reference.section]]
from = "the-crown"
title = "Elara"                 # the name changes with the reveal
text = "The exiled queen."
match = ["Elara"]               # only looked for from this chapter on
```

Names match whole words, case-sensitively. To link something the names don't catch, or to
stop a false match: `:ref[the old woman]{elara}`, `:noref[Elara]`.

## Theme

Everything in `theme/theme.toml` is optional:

```toml
[tokens]                        # any of the runtime's --tome-* settings
accent = "#8a3b2e"
page-bg = "#f6f1e7"
font-body = "Lora, Georgia, serif"
gradient-dawn = "linear-gradient(90deg, #f6c453, #c0533a, #f6c453)"

[[fonts]]
family = "Lora"
src = "lora-regular.woff2"

[styles.handwriting]
font-family = "Caveat, cursive"
font-size = "1.4em"

[backgrounds.landing]           # also [backgrounds.reading]
src = "fog.webm"                # image, GIF or video
poster = "fog.jpg"              # shown when readers turn motion off
opacity = 0.5

[decoration]
page_texture = "paper.png"
chapter_ornament = "ornament.svg"
drop_caps = true
page_frame = { src = "frame.svg", slice = 30, width = "18px" }

[landing]
cover = "cover.jpg"
cover_alt = "…"
layout = "split"                # or "centered"
music = "theme"

[[override]]                    # changes from a chapter onward
from = "the-keeper"
tokens = { page-bg = "#1d2125", text = "#e6dfd0" }
decoration = { page_texture = "none" }
```
