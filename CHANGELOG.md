# Changelog

All notable changes to `mdstream` are documented in this file.

## [Unreleased]

## [0.3.6] - 2026-06-30

### Changed

- The H2 (`##`) heading color in the palette changed from yellow/orange (`RGB(255,170,80)`) to a vibrant cyan (`RGB(96,214,255)`), matching the cyan used for function names in the built-in `mdstream` code theme. All six heading levels (H1-H6) remain distinct, vibrant hues.

## [0.3.5] - 2026-06-07

### Fixed

- GFM table delimiter rows with fewer than three hyphens per cell now promote to a rendered table instead of leaking as raw `| ... |` markdown. `table_separator_cell_re` used `^:?-{3,}:?# Changelog

All notable changes to `mdstream` are documented in this file.

, which demanded at least 3 hyphens and so rejected every short alignment delimiter the GFM spec permits (`-`, `--`, `:-`, `-:`, `:-:`, `--:`, `:--`). Any table whose separator row used these, e.g. `|---|--:|--:|`, failed to enter table mode and the header, separator, and body all rendered as literal markdown source. Fix: relax the quantifier to `^:?-+:?# Changelog

All notable changes to `mdstream` are documented in this file.

 (one-or-more hyphens), which still rejects colon-only cells (`:`, `::`) that carry no hyphen. Reproduced from a financial scenario grid using `|---|--:|--:|...`; regression tests in `tests/tables.rs`.

## [0.3.4] - 2026-05-20

### Changed

- The live partial row, the in-progress paragraph that re-renders in place while a producer is still streaming, is now projected through a *tail-window* whenever the buffer outgrows the visible budget (`terminal_width - left_pad`). Pre-0.3.4 the partial was allowed to wrap onto multiple physical rows, which made the cursor visibly drop on every byte that crossed a wrap boundary and caused jitter / flicker in hosts that surface the partial inside a fixed UI region (chat panes, IDE sidebars). New behavior: as soon as the buffer would exceed the row, the renderer erases the row and repaints as `pad` + dim `…` + the rightmost `budget - 1` cells of the buffer. The window slides left in lock-step with the producer, the cursor stays anchored to the right edge, and the live partial is *always* exactly one physical row tall regardless of buffer size. The full buffer is preserved verbatim and is what `render_line` renders when the newline finally lands, so no content is lost. Append-only fast path is kept for the common case where the partial fits. The slow erase-and-repaint path only engages on overflow. Resize wider mid-stream automatically refits the buffer in full and drops the dim marker on the next token. Resize narrower auto-engages the tail-window. CJK / VS-16 / combining-mark clusters are never split at the tail boundary (`tail_by_width` walks grapheme clusters, mirroring `wrap_styled_cell`'s atomizer). No new CLI flag. This is a strict UX improvement with no downside: output is byte-identical to 0.3.3 whenever the partial fits.

## [0.3.3] - 2026-05-14

### Fixed

- Table cells containing a base codepoint followed by a Variation Selector (VS15 `U+FE0E` text presentation, VS16 `U+FE0F` emoji presentation) no longer mis-align the surrounding row by ±1 cell when `--table-fit` soft-wraps the cell. `wrap_styled_cell` measured glyph width per codepoint via `UnicodeWidthChar::width`, which can't see the following VS and therefore disagreed with the string-level `UnicodeWidthStr::width` used by `visible_width` / `align_cell`. Wrapping packed one extra cell onto the line, `align_cell`'s `saturating_sub` clamped padding to zero, and the row overflowed the column by 1 cell — pushing the closing `│` off the right edge of the terminal (visible as "the last column lost its border"). Fix: in `read_word_pieces`, bundle each base codepoint and any immediately-following `U+FE0E` / `U+FE0F` into a single glyph atom whose width is computed via `UnicodeWidthStr::width` on the combined slice. Bonus: the base+VS pair can no longer be split across a wrap boundary. Reproduces with `⚠️ Degraded`, `♣️ Black Club Suit`, `⏸️ Paused` at narrow column widths; regression tests in `wrap_styled_cell_tests`.
- Tables with ragged rows (a row whose cell count differs from the header) no longer break the rest of the table. Previously, the first mismatched row flushed the table early and every subsequent row — *including rows that DID match the header* — rendered as raw `| cell | cell |` markdown text, because re-entering table mode requires a fresh `|---|` separator that the input doesn't have. New GFM-compatible behavior: ragged rows stay inside the table; missing trailing cells render blank, extra cells are dropped. Reported via `tmp/make-a-detailed-plan-virtual-dusk.md` where row 13 of a 4-column table omitted its `| Source |` cell and torpedoed rows 13 onward. Regression test: `ragged_rows_keep_table_open` in `tests/tables.rs`.

## [0.3.2] - 2026-05-06

### Fixed

- Single-column markdown tables (`| huge |\n|------|\n| body |`) are now promoted and rendered like any other table. The candidate regex required at least one inner pipe and `split_table_row` rejected rows below 2 cells, so 1-column tables silently leaked through as raw markdown source. Bug predates 0.3.0; uncovered by the `Single mega-cell` example in `tmp/markdown-tables-mock-002.md`.
- `mdstream --help` now actually lists `--table-fit` / `--table-width-offset` and the matching `MDSTREAM_TABLE_FIT` / `MDSTREAM_TABLE_WIDTH_OFFSET` env vars. The hand-curated help screen in `src/help.rs` had been out of sync with the clap derive in `src/cli.rs` since 0.3.0 — both flags shipped without ever appearing in the rendered help.

### Changed

- The `## Flags` section of `--help` is now generated programmatically by introspecting `clap::Command::get_arguments()`. The clap derive in `src/cli.rs` is the single source of truth: edit a flag's `help`, `default_value_t`, `env`, or `value_name` there and the rendered help reflects it on next build, no `help.rs` edits needed.
- Flags are grouped under headings (`Display`, `Code highlighting`, `Tables`) via `#[arg(help_heading = "...")]` on each clap arg. Each entry is a single line with help text, default, and env var, replacing the previous separate `## Environment` section.
- New tests guarantee the pipeline can't silently drop entries: a unit test in `src/help.rs` and an integration test in `tests/cli_integration.rs` both walk every clap arg and assert each long flag and `env =` substring appears in the rendered output. CI fails if `src/cli.rs` ever drifts from what `--help` shows.

## [0.3.1] - 2026-05-06

### Fixed

- `--table-fit` is now a *max-width* constraint instead of a fill: tables whose natural width fits inside `terminal_width + table_width_offset - padding` render at their natural size (matching the no-fit default), and the fit/squeeze allocator only engages when the natural table would overflow. The 0.3.0 behavior of unconditionally expanding every table to fill the terminal — even a tiny `| A | B |` ballooning to 80 columns — was the wrong default. The `--table-fit` and `--table-width-offset` `--help` strings were updated to match.

## [0.3.0] - 2026-05-06

### Added

- Opt-in `--table-fit` flag (env: `MDSTREAM_TABLE_FIT`) for terminal-width-aware table layout. When enabled, every flushed table is sized against the live terminal width — measured fresh per table so consecutive tables can pick up resizes between flushes. Cells that exceed their allotted column width are soft-wrapped at word boundaries with a hard-break fallback for over-long tokens; ANSI styling carries across wrap boundaries so a bold or colored cell stays bold/colored on every visual row.
- Companion `--table-width-offset N` flag (env: `MDSTREAM_TABLE_WIDTH_OFFSET`, signed) for trimming or expanding the table-fit target width by a cell count. Negative values leave a right gutter; positive values over-expand.

### Changed

- Column widths under `--table-fit` are allocated by a CSS `table-layout: auto`-style hybrid: a *slack* branch when content fits with room to spare (extra distributed proportional to natural widths), a *fit* branch when content fits only after squeezing (extra distributed proportional to per-column headroom), and a *squeeze* branch for too-narrow targets (proportional to min with `>= 1` clamps and trim-from-widest reconciliation). Width detection auto-disables — falls back to the existing content-only widths — when no live width can be determined (no TTY, no `COLUMNS`, terminal query failed) or the post-offset, post-padding target is non-positive.

## [0.2.2] - 2026-05-05

### Fixed

- Partial-paragraph redraw is now resize-safe and host-width aware. The renderer tracks the wrapped row count of the in-progress partial as bytes are emitted (using the terminal width that was live at each emit), instead of recomputing it from `terminal::size()` at flush time. Previous behavior left a stranded raw-markdown wrap row in scrollback above the rendered version when the width seen at flush differed from the width in effect when the bytes were originally drawn (mid-stream resize, or `/dev/tty` returning a value that disagreed with the host's `process.stdout.columns`).

### Added

- `term_width()` honors the `COLUMNS` environment variable when no test override is set, falling back to `/dev/tty` only if `COLUMNS` is unset or unparseable. Hosts that pipe stdio to mdstream (e.g. embedding terminals) can now pin the width explicitly and have it respected by the partial-redraw math.

### Changed

- Completed task list items now render with `✔` instead of `☑` for a cleaner, more legible glyph at typical terminal font sizes.

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
