# interactive_storybook

A framework for turning a linear novel into a standalone, bespoke reading app: spoiler-aware
references, music and sound, kinetic text, an illustration track and per-book theming.
Authors write Markdown and TOML; `tome` (placeholder name) builds the book.

See [docs/spec.md](docs/spec.md) for the full specification, and
[cli/templates/GUIDE.md](cli/templates/GUIDE.md) for the author's guide.

## Layout

| Folder | What it is |
|---|---|
| `runtime/` | The reader app (Svelte + TypeScript), prebuilt once and shared by every book |
| `cli/` | `tome`, the Rust compiler and CLI; embeds the built runtime |
| `examples/the-uneven-bell/` | A sample book that uses every feature |
| `scripts/` | Tools for regenerating the sample's placeholder assets |

## Building

The CLI embeds the runtime, so build the runtime first:

```sh
cd runtime && npm install && npm run build && cd ..
cargo build --release          # produces target/release/tome
```

Then:

```sh
target/release/tome preview examples/the-uneven-bell --open   # read it with live reload
target/release/tome build examples/the-uneven-bell            # web build in examples/the-uneven-bell/dist/web
target/release/tome build examples/the-uneven-bell --target all   # plus Linux and Windows apps
target/release/tome new my-book                              # start a new book
```

## Working on the runtime

```sh
cd runtime
npm run sample   # compile the sample book into public/book (needs Rust)
npm run dev      # Vite dev server with hot reload
npm test         # unit tests
npm run check    # type check
```

Rust tests: `cargo test` (includes compiling the sample book).

## License

Apache-2.0. Books built with the framework belong to their authors; the framework's license
places no requirements on book content.
