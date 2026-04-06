# mdstream

`mdstream` is a Rust rewrite of the original Python streaming Markdown renderer used by `llm`.

It is built for terminal output that feels immediate while still becoming readable as soon as a line completes:

- raw chunks stream to the terminal as they arrive
- completed lines are re-rendered with Markdown styling
- fenced code blocks are syntax-highlighted
- the active table block can be repainted in place
- the runtime is Rust-only

This project is intentionally small and terminal-focused. It does not try to be a full Markdown parser. It is a line-oriented streaming renderer with a small amount of state for the cases that matter in a TTY.

## What It Does

The current Rust implementation covers the core hybrid renderer behavior:

- partial-line streaming
- EOF flush for unterminated input
- UTF-8 chunk handling
- headings from `#` through `######`
- blockquotes
- unordered lists
- ordered lists
- task lists
- horizontal rules
- inline Markdown such as bold, italic, strike, code spans, links, images, autolinks, and bare URLs
- pipe-table promotion, alignment, and bottom-block repaint
- fenced code blocks with language labels and line numbers
- `syntect`-backed syntax highlighting for fenced code blocks
- environment-driven padding and line-number control
- standalone binary integration tests and ANSI snapshot coverage

## Architecture

The renderer is split into three layers:

1. `src/lib.rs` wires stdin and stdout together.
2. `src/renderer.rs` owns the streaming state machine and all terminal rendering.
3. `src/cli.rs` defines the CLI surface and options.

The renderer keeps just enough state to make streaming work well in a terminal:

- `partial` for the current incomplete line
- fenced-code state for incremental highlighting
- heading spacing state
- table state for promotion and repaint

That structure is deliberate. The goal is to keep latency low without waiting for a full document parse.

## Why `syntect`

The Python implementation used `pygments` for fenced code blocks. In Rust, the first choice is `syntect`.

Reasons:

- it is a proven terminal-highlighting library
- it ships with broad syntax coverage
- it already exposes ANSI terminal output helpers
- it fits the line-oriented streaming design without bringing in a full parser stack
- it can run in a pure-Rust configuration with the `default-fancy` feature set

`tree-sitter-highlight` was considered as well, but it is a better fit for editor-grade parsing than for the bootstrap version of this renderer. It also requires language grammar plumbing that is unnecessary for the first Rust pass.

If profiling later shows that code-block highlighting needs a different backend, that decision can be revisited. `syntect` is the right starting point.

### Syntax theme

Code blocks are rendered with `syntect`'s bundled `base16-ocean.dark` theme. This is intentionally not a pixel match for the Python implementation, which used Pygments' `monokai` style. Two consequences:

- Token colors in fenced code blocks will differ from Python output. The structural layout (line numbers, language label, frame characters) is identical.
- The theme is currently hard-coded in `src/renderer.rs`. A configurable theme is a future improvement; if that lands, the default will stay `base16-ocean.dark` for backwards compatibility.

## Usage

Build and run the renderer from stdin:

```bash
cargo run -- < input.md
```

Environment variables:

```bash
MDSTREAM_PADDING=2 cargo run -- < input.md
MDSTREAM_NO_LINENO=true cargo run -- < input.md
```

If you are working on the CLI and want to inspect the current flags:

```bash
cargo run -- --help
```

Typical development flow:

```bash
cargo fmt
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

## Development

The project uses Rust 2024 and a pinned toolchain in `rust-toolchain.toml`.

Recommended commands:

```bash
cargo fmt
cargo clippy --all-targets --all-features -- -D warnings
cargo test
rust-analyzer diagnostics .
```

The test suite is expected to grow into a full parity suite with the Python implementation. The current plan includes:

- streaming-state tests
- inline Markdown tests
- structured-line tests
- code-fence tests
- table-state tests
- CLI integration tests
- snapshot tests for ANSI-heavy output

## Project Status

This repository is under active porting work.

What is in place now:

- the Rust crate is initialized
- the toolchain is pinned
- the dependency set is chosen
- the streaming core is implemented
- table promotion and repaint are implemented
- fenced code blocks are highlighted through `syntect`
- the test suite is split across focused integration files under `tests/`
- snapshot coverage is in place for ANSI-heavy mixed output
- the current porting status is tracked in [PROGRESS.md](./PROGRESS.md)

What remains:

- deeper inline Markdown edge cases
- final CLI polish

## Validation

Use these commands to validate the project locally:

```bash
cargo fmt
cargo clippy --all-targets --all-features -- -D warnings
cargo test
rust-analyzer diagnostics .
```

If you only want a quick smoke test:

```bash
cargo test
```
