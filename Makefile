# Build and test the typst-exif package.

TYPST ?= typst
WASM := plugin/target/wasm32-unknown-unknown/release/typst_exif_plugin.wasm

.PHONY: all plugin test test-rust test-typst fixtures example clean

all: exif.wasm

exif.wasm: $(shell find plugin/src -name '*.rs') plugin/Cargo.toml
	cd plugin && cargo build --release --target wasm32-unknown-unknown
	cp $(WASM) $@

test: test-rust test-typst

test-rust:
	cd plugin && cargo test

test-typst: exif.wasm
	$(TYPST) compile --root . --format pdf tests/test.typ /dev/null
	$(TYPST) compile --root . --format pdf tests/test-interpret.typ /dev/null

fixtures:
	cd plugin && cargo run --example mkfixtures

example: exif.wasm
	$(TYPST) compile --root . examples/metadata.typ examples/metadata.pdf

clean:
	cd plugin && cargo clean
