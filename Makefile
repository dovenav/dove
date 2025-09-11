# Simple convenience Makefile for dove

CARGO ?= cargo
FEATURES ?=
CARGO_FLAGS ?=
ARGS ?=

.PHONY: help build build-remote build-site preview preview-remote init fmt check-fmt clippy lint clean

help:
	@echo "Targets:"
	@echo "  make build                 # cargo build $(FEATURES)"
	@echo "  make build-remote          # cargo build --features remote"
	@echo "  make build-site            # cargo run $(FEATURES) -- build $(ARGS)"
	@echo "  make preview               # cargo run $(FEATURES) -- preview --build-first $(ARGS)"
	@echo "  make preview-remote        # cargo run --features remote -- preview --build-first $(ARGS)"
	@echo "  make init                  # cargo run $(FEATURES) -- init $(ARGS)"
	@echo "  make fmt                   # cargo fmt --all"
	@echo "  make check-fmt             # cargo fmt --all -- --check"
	@echo "  make clippy                # cargo clippy $(FEATURES) -- -D warnings"
	@echo "  make lint                  # = check-fmt + clippy"
	@echo "  make clean                 # cargo clean"

build:
	$(CARGO) build $(FEATURES) $(CARGO_FLAGS)

build-remote:
	$(CARGO) build --features remote $(CARGO_FLAGS)

build-site:
	$(CARGO) run $(FEATURES) -- build $(ARGS)

preview:
	$(CARGO) run $(FEATURES) -- preview --build-first $(ARGS)

preview-remote:
	$(CARGO) run --features remote -- preview --build-first $(ARGS)

init:
	$(CARGO) run $(FEATURES) -- init $(ARGS)

fmt:
	$(CARGO) fmt --all

check-fmt:
	$(CARGO) fmt --all -- --check

clippy:
	$(CARGO) clippy $(FEATURES) -- -D warnings

lint: check-fmt clippy

clean:
	$(CARGO) clean

