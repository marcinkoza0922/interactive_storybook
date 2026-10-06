//! `tome` (placeholder name): builds interactive novels from Markdown and TOML.

mod assets;
mod bundle;
mod compile;
mod desktop;
mod diagnostics;
mod markdown;
mod output;
mod paginate;
mod preview;
mod references;
mod scaffold;
mod theme;

use anyhow::{Result, bail};
use clap::{Parser, Subcommand, ValueEnum};
use diagnostics::Severity;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "tome", version, about = "Build interactive novels from Markdown and TOML")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Start a new book project
    New {
        /// Folder to create
        dir: PathBuf,
        /// The book's title (default: from the folder name)
        #[arg(long)]
        title: Option<String>,
    },
    /// Look for problems without building
    Check {
        #[arg(default_value = ".")]
        dir: PathBuf,
    },
    /// Build the finished book
    Build {
        #[arg(default_value = ".")]
        dir: PathBuf,
        /// Output folder (default: dist inside the project); each target gets a folder in it
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long, value_enum, default_value_t = Target::Web)]
        target: Target,
        /// Processor for desktop builds
        #[arg(long, value_enum, default_value_t = Arch::X64)]
        arch: Arch,
        /// Write only book.json and its assets, without the runtime
        #[arg(long)]
        bundle_only: bool,
    },
    /// Read the book in a browser, rebuilding on every change
    Preview {
        #[arg(default_value = ".")]
        dir: PathBuf,
        #[arg(long, default_value_t = 4321)]
        port: u16,
        /// Open the browser
        #[arg(long)]
        open: bool,
    },
    /// Write contents.toml, the chapter list readers see, for editing
    Contents {
        #[arg(default_value = ".")]
        dir: PathBuf,
        /// Replace an existing contents.toml
        #[arg(long)]
        regenerate: bool,
    },
}

#[derive(Clone, Copy, PartialEq, ValueEnum)]
enum Target {
    /// A folder to host on any web server
    Web,
    /// A Linux desktop app
    Linux,
    /// A Windows desktop app
    Windows,
    /// All of the above
    All,
}

#[derive(Clone, Copy, ValueEnum)]
enum Arch {
    X64,
    Arm64,
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<ExitCode> {
    match cli.command {
        Command::New { dir, title } => {
            let title = title.unwrap_or_else(|| title_from_folder(&dir));
            scaffold::create(&dir, &title)?;
            println!("Created {}. Next:\n\n    cd {}\n    tome preview --open\n\nGUIDE.md explains how everything works.", dir.display(), dir.display());
            Ok(ExitCode::SUCCESS)
        }
        Command::Check { dir } => {
            let compiled = compile::compile(&dir);
            Ok(report(&compiled.diagnostics, &dir, "Checked"))
        }
        Command::Build { dir, out, target, arch, bundle_only } => {
            let compiled = compile::compile(&dir);
            let code = report(&compiled.diagnostics, &dir, "Built");
            if compiled.diagnostics.has_errors() {
                eprintln!("Nothing was written; fix the errors above first.");
                return Ok(code);
            }
            let out = out.unwrap_or_else(|| dir.join("dist"));
            if bundle_only {
                if target != Target::Web {
                    bail!("--bundle-only only applies to --target web");
                }
                output::write_bundle_only(&compiled, &out)?;
                println!("Wrote {}", out.display());
                return Ok(code);
            }

            let arch = match arch {
                Arch::X64 => desktop::Arch::X64,
                Arch::Arm64 => desktop::Arch::Arm64,
            };
            if matches!(target, Target::Web | Target::All) {
                let web = out.join("web");
                output::write_web(&compiled, &web)?;
                println!("Wrote {} (host it on any web server)", web.display());
            }
            for (wanted, platform) in [(Target::Linux, desktop::Platform::Linux), (Target::Windows, desktop::Platform::Windows)] {
                if target == wanted || target == Target::All {
                    let release = desktop::electron_release(platform, arch)?;
                    let folder = out.join(format!("{}-{}", platform.name(), arch.name()));
                    let archive = desktop::build(&compiled, &folder, platform, arch, &release)?;
                    println!(
                        "Wrote {} (the app, ready to run) and {} (to distribute)",
                        folder.join(desktop::app_names(&compiled).product).display(),
                        archive.display()
                    );
                }
            }
            Ok(code)
        }
        Command::Preview { dir, port, open } => {
            preview::serve(&dir, port, open)?;
            Ok(ExitCode::SUCCESS)
        }
        Command::Contents { dir, regenerate } => {
            let file = dir.join("contents.toml");
            if file.exists() && !regenerate {
                bail!("{} already exists; pass --regenerate to replace it", file.display());
            }
            std::fs::write(&file, compile::generate_contents(&dir))?;
            println!("Wrote {}. Edit it to rename, hide or group chapters.", file.display());
            Ok(ExitCode::SUCCESS)
        }
    }
}

/// Print diagnostics and a one-line summary; failure if there were errors.
fn report(diagnostics: &diagnostics::Diagnostics, dir: &Path, verb: &str) -> ExitCode {
    let root = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
    let rendered = diagnostics.render(&root);
    if !rendered.is_empty() {
        eprintln!("{rendered}\n");
    }
    let errors = diagnostics.count(Severity::Error);
    let warnings = diagnostics.count(Severity::Warning);
    let plural = |n: usize, word: &str| format!("{n} {word}{}", if n == 1 { "" } else { "s" });
    if errors > 0 {
        eprintln!("{} found.", plural(errors, "error") + &if warnings > 0 { format!(" and {}", plural(warnings, "warning")) } else { String::new() });
        return ExitCode::FAILURE;
    }
    match warnings {
        0 if verb == "Checked" => println!("No problems found."),
        0 => {}
        _ => eprintln!("{}.", plural(warnings, "warning")),
    }
    ExitCode::SUCCESS
}

fn title_from_folder(dir: &Path) -> String {
    let name = dir.file_name().and_then(|n| n.to_str()).unwrap_or("Untitled");
    let words: Vec<String> = name
        .split(['-', '_', ' '])
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut chars = w.chars();
            chars.next().map(|c| c.to_uppercase().collect::<String>() + chars.as_str()).unwrap_or_default()
        })
        .collect();
    if words.is_empty() { "Untitled".into() } else { words.join(" ") }
}
