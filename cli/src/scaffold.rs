//! `tome new`: a starter project that builds cleanly and shows the main features.

use anyhow::{Context, Result, bail};
use std::fs;
use std::path::Path;

const GUIDE: &str = include_str!("../templates/GUIDE.md");
const PARCHMENT: &str = include_str!("../templates/parchment.svg");

pub fn create(dir: &Path, title: &str) -> Result<()> {
    if dir.exists() && fs::read_dir(dir)?.next().is_some() {
        bail!("{} already exists and isn't empty", dir.display());
    }
    let files: Vec<(&str, String)> = vec![
        (
            "book.toml",
            format!(
                "title = {title:?}\nauthor = \"Your Name\"\nlanguage = \"en\"\n\n\
                 # Pages break automatically before a paragraph that would go over these limits.\n\
                 # max_lines counts paragraphs and line breaks you write, not lines on screen.\n\
                 [pagination]\nmax_words = 220\n\n\
                 # [audio]\n# restore_delay = 3      # seconds before music returns after going back a page\n\
                 # ducking = 0.35         # music and ambience volume under narration (1: off)\n\n\
                 # [illustrations]\n# linger = 3            # seconds a new illustration shows on small screens\n"
            ),
        ),
        (
            "manuscript/01-the-beginning.md",
            "# The Beginning\n\n\
             This is your first chapter. Write in ordinary Markdown: *italics*, **bold**, and a blank\n\
             line between paragraphs. Pages break by themselves; see book.toml.\n\n\
             :::reveal\n\
             This paragraph fades in when the reader advances.\n\n\
             And this one after it.\n\
             :::\n\n\
             “Oh, :fx[how wonderful]{wave},” she said, not meaning it at all.\n\n\
             ::pagebreak\n\n\
             ::paper{parchment}\n\
             A new page, printed on parchment: the `::paper{parchment}` line above picks the\n\
             paper defined under [papers.parchment] in theme/theme.toml, for this page only.\n\n\
             GUIDE.md shows everything else: music and sound, illustrations, references,\n\
             special text and theming.\n"
                .to_string(),
        ),
        (
            "references.toml",
            "# Entries readers can look up. Each section unlocks once they reach its chapter.\n\
             #\n\
             # [[reference]]\n# id = \"elara\"\n# match = [\"Elara\", \"the Witch of Varn\"]\n#\n\
             # [[reference.section]]\n# from = \"the-beginning\"\n# title = \"The Witch of Varn\"\n# text = \"A healer from the northern villages.\"\n"
                .to_string(),
        ),
        (
            "theme/theme.toml",
            "# The book's look. Every setting is optional; see GUIDE.md for the full list.\n\n\
             [tokens]\n# accent = \"#8a3b2e\"\n# page-bg = \"#f6f1e7\"\n# font-body = \"Georgia, serif\"\n\n\
             [decoration]\n# drop_caps = true\n# paper_grain = 0.3      # grain on every page, 0 to 1\n\n\
             # A paper pages can ask for with ::paper{parchment}.\n\
             [papers.parchment]\ncolor = \"#eadbb8\"\ntexture = \"parchment.svg\"\ngrain = 0.45\n"
                .to_string(),
        ),
        ("theme/custom.css", "/* Optional extra CSS, loaded after the theme. Style the tome-* classes and tokens. */\n".to_string()),
        ("GUIDE.md", GUIDE.to_string()),
        (".gitignore", "dist/\n".to_string()),
        ("assets/audio/.gitkeep", String::new()),
        ("assets/images/parchment.svg", PARCHMENT.to_string()),
        ("assets/video/.gitkeep", String::new()),
        ("assets/fonts/.gitkeep", String::new()),
    ];
    for (path, contents) in files {
        let path = dir.join(path);
        fs::create_dir_all(path.parent().unwrap())?;
        fs::write(&path, contents).with_context(|| format!("couldn't write {}", path.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_starter_project_builds_without_problems() {
        let dir = tempfile::tempdir().unwrap();
        let book = dir.path().join("my-book");
        create(&book, "My Book").unwrap();
        let compiled = crate::compile::compile(&book);
        assert!(compiled.diagnostics.items.is_empty(), "{}", compiled.diagnostics.render(&book));
        assert_eq!(compiled.bundle.book.id, "my-book");
        assert_eq!(compiled.bundle.chapters[0].pages.len(), 2);
        let papers: Vec<_> = compiled.bundle.chapters[0].pages.iter().map(|p| p.paper.as_deref()).collect();
        assert_eq!(papers, [None, Some("parchment")], "the second page shows off a textured paper");
        assert!(create(&book, "Again").is_err(), "won't overwrite");
    }
}
