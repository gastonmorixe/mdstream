.DEFAULT_GOAL := help

.PHONY: help build release test fmt lint check install

help:
	@printf "Available targets:\n"
	@printf "  build: cargo build\n"
	@printf "  release: cargo build --release\n"
	@printf "  test: cargo test\n"
	@printf "  fmt: cargo fmt\n"
	@printf "  lint: cargo clippy --all-targets --all-features -- -D warnings\n"
	@printf "  check: cargo fmt --check, cargo clippy, cargo test\n"
	@printf "  install: cargo install --path .\n"

build:
	cargo build

release:
	cargo build --release

test:
	cargo test

fmt:
	cargo fmt

lint:
	cargo clippy --all-targets --all-features -- -D warnings

check:
	cargo fmt --check
	cargo clippy --all-targets --all-features -- -D warnings
	cargo test

install:
	cargo install --path .
