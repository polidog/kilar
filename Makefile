# Makefile for kilar

.PHONY: help build test clean release check fmt clippy audit install

# Default target
help:
	@echo "Available targets:"
	@echo "  build      - Build the project in debug mode"
	@echo "  test       - Run all tests"
	@echo "  clean      - Clean build artifacts"
	@echo "  release    - Build optimized release binary"
	@echo "  check      - Run cargo check"
	@echo "  fmt        - Format code"
	@echo "  clippy     - Run clippy lints"
	@echo "  audit      - Run security audit"
	@echo "  install    - Install the binary locally"
	@echo "  ci         - Run all CI checks"

# Build targets
build:
	cargo build

release:
	cargo build --release

# Test targets
test:
	cargo test

# Code quality targets
check:
	cargo check

fmt:
	cargo fmt --all

clippy:
	cargo clippy --all-targets --all-features -- -D warnings

audit:
	cargo audit

# Utility targets
clean:
	cargo clean

install:
	cargo install --path .

# CI target that runs all quality checks
ci: fmt clippy test audit check
	@echo "All CI checks passed!"

# Package and publish
package:
	cargo package

publish-dry-run:
	cargo publish --dry-run

publish:
	cargo publish

# Development helpers
dev-deps:
	cargo install cargo-audit
	cargo install cargo-watch

watch:
	cargo watch -x check -x test

# Cross-compilation targets
build-linux:
	cargo build --release --target x86_64-unknown-linux-gnu

build-windows:
	cargo build --release --target x86_64-pc-windows-msvc

build-macos-intel:
	cargo build --release --target x86_64-apple-darwin

build-macos-arm:
	cargo build --release --target aarch64-apple-darwin

# Build all release targets (requires cross-compilation setup)
build-all: release build-linux build-windows build-macos-intel build-macos-arm
	@echo "All release binaries built!"

