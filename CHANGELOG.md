# Changelog

All notable changes to `mdstream` are documented in this file.

## [0.2.1] - 2026-04-23

### Fixed

- Normalize TypeScript-style fenced code tokens so `typescript`, `ts`, `mts`, `cts`, and `tsx` fences use JavaScript highlighting instead of falling back to plain text.

### Changed

- Release automation now starts from pushing a semantic version tag, reruns `cargo fmt --check`, `cargo clippy --locked --all-targets --all-features -- -D warnings`, `cargo test --locked`, and only then builds release artifacts.
- Release publishing now packages and uploads native tarballs for Linux x86_64, Linux arm64, and macOS arm64 from the verified tag build.
- README and renderer docs now describe the tag-driven release flow, prebuilt binary installs, and the syntax-token normalization behavior.

## [0.2.0] - 2026-04-22

### Added

- Bundled the `mdstream` house theme plus vendored `Catppuccin Mocha`, `Sublime Snazzy`, and `Dracula` code themes.
- Added rendered `--help` output, configurable code block backgrounds, inline code color selection, and richer CLI documentation.

### Changed

- Improved table rendering width calculations and release build tooling with a project `Makefile`.

## [0.1.0] - 2026-04-06

### Added

- Initial public release of the Rust `mdstream` streaming Markdown renderer.
