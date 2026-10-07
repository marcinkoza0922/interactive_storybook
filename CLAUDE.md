# Working in this repo

`cli/` is the Rust CLI (`tome`); `runtime/` is the Svelte reader it embeds. The `Makefile` lists every build step.
For now, breaking changes are acceptable.

## Rust commands

Use these, which keep output short. Don't run plain `cargo build` or `cargo test`.

- Type-check: `cargo check --message-format=short 2>&1 | head -40` (usually only the first error matters)
- Lint: `cargo clippy --all-targets --message-format=short`. Keep it at zero warnings; `make lint` fails on any.
- Test: `cargo nextest run --profile agent --hide-progress-bar --cargo-quiet [filter]`, with a filter for the module you changed
- Without nextest: `RUST_BACKTRACE=0 cargo test -q --bins [filter]`

A post-edit hook (`.claude/settings.json`) runs `cargo check` after each `.rs` edit and prints only on errors.

Print to the terminal only in user-facing CLI code, marked with `#[allow(clippy::print_stdout, clippy::print_stderr, reason = "...")]`. Keep functions under 60 lines (`too_many_lines`) and files under about 400 (`make sizes`).

## Runtime commands

Use npm, from `runtime/`. Use these instead of `npm run check`, `npm test` and `npm run build`:

- Type-check: `npm run --silent check:ai`
- Test: `npm run --silent test:ai [file]`
- Build: `npm run --silent build:ai`

Write Svelte 5 runes (`$state`, `$derived`, `$props`, `$effect`). Don't use `export let` or `$:`.

## Don't read

`Cargo.lock`, `runtime/package-lock.json`, `target/`, `runtime/dist/`, `runtime/node_modules/`, binary assets under `examples/` and `cli/templates/`, or the SVGs under `examples/*/assets/images/`.
