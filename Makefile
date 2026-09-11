# Build and test the typst-exif package.

TYPST ?= typst
WASM := plugin/target/wasm32-unknown-unknown/release/typst_exif_plugin.wasm

# Read from typst.toml so the staged directory can never disagree with it.
NAME := $(shell sed -n 's/^name = "\(.*\)"/\1/p' typst.toml)
VERSION := $(shell sed -n 's/^version = "\(.*\)"/\1/p' typst.toml)
DIST := dist/preview/$(NAME)/$(VERSION)

# Where Typst looks for local packages, so `@preview/$(NAME):$(VERSION)` can
# be resolved before the package is published.
DATA_DIR ?= $(if $(XDG_DATA_HOME),$(XDG_DATA_HOME),$(HOME)/.local/share)
INSTALLED := $(DATA_DIR)/typst/packages/preview/$(NAME)/$(VERSION)

.PHONY: all plugin test test-rust test-typst test-package fixtures example dist install uninstall clean

all: exif.wasm

exif.wasm: $(shell find plugin/src -name '*.rs') plugin/Cargo.toml
	cd plugin && cargo build --release --target wasm32-unknown-unknown
	cp $(WASM) $@

test: test-rust test-typst test-package

test-rust:
	cd plugin && cargo test

test-typst: exif.wasm
	$(TYPST) compile --root . --format pdf tests/test.typ /dev/null
	$(TYPST) compile --root . --format pdf tests/test-interpret.typ /dev/null

fixtures:
	cd plugin && cargo run --example mkfixtures

example: exif.wasm
	$(TYPST) compile --root . examples/metadata.typ examples/metadata.pdf

# Stages exactly the files that belong in the typst/packages pull request:
# the package itself, plus the example the README links to. Copy the staged
# $(NAME)/$(VERSION) directory to packages/preview/$(NAME)/$(VERSION) in a
# fork of https://github.com/typst/packages and open a pull request.
dist: exif.wasm
	rm -rf $(DIST)
	mkdir -p $(DIST)/examples
	cp typst.toml lib.typ read.typ interpret.typ exif.wasm README.md LICENSE NOTICE.md $(DIST)/
	cp examples/photo.jpg examples/metadata.png $(DIST)/examples/
	sed 's|#import "/lib.typ"|#import "@preview/$(NAME):$(VERSION)"|' \
		examples/metadata.typ > $(DIST)/examples/metadata.typ
	@echo "staged $(DIST)"
	@du -sh $(DIST)

# Installs the staged package where Typst resolves @preview from, so the
# README's examples can be compiled exactly as a user would run them.
install: dist
	rm -rf $(INSTALLED)
	mkdir -p $(dir $(INSTALLED))
	cp -r $(DIST) $(INSTALLED)
	@echo "installed $(INSTALLED)"

uninstall:
	rm -rf $(INSTALLED)

# Compiles against the installed package, through @preview, with no access to
# the working tree: this is what a user gets.
test-package: install
	$(TYPST) compile --root tests/package tests/package/test.typ /dev/null --format pdf
	$(TYPST) compile --root $(DIST) $(DIST)/examples/metadata.typ /dev/null --format pdf

clean:
	cd plugin && cargo clean
	rm -rf dist
