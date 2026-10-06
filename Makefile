# Builds the runtime first, then the CLI that embeds it.
#
#   make            build everything (release): target/release/tome
#   make debug      the same, with a debug CLI: target/debug/tome
#   make test       run all tests
#   make check      type-check the runtime and lint-check the CLI
#   make sample     compile the sample book into the runtime's dev fixture
#   make preview    preview the sample book with live reload
#   make clean      remove build output

.PHONY: all release debug runtime test check sample preview clean

all: release

release: runtime
	cargo build --release

debug: runtime
	cargo build

runtime: runtime/node_modules
	cd runtime && npm run build

# Install packages on first use, or when the lockfile changes.
runtime/node_modules: runtime/package-lock.json
	cd runtime && npm ci
	@touch $@

test: runtime
	cd runtime && npm test
	cargo test

check: runtime/node_modules
	cd runtime && npm run check
	cargo check

sample: runtime
	cd runtime && npm run sample

preview: debug
	target/debug/tome preview examples/the-uneven-bell --open

clean:
	rm -rf runtime/dist runtime/public/book
	cargo clean
