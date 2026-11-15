# Spec: opt-in word wrap

- **Date:** 2026-11-15
- **Status:** Implemented in 0.3.8
- **Fixes:** BUGS.md [064]
- **Dependencies added:** none

## 1. Summary

Add an opt-in `--wrap` flag. When `--wrap` is on, mdstream breaks long text lines at spaces. Each row then fits the wrap width. Continuation rows repeat the prefix of the line.

- `--wrap` with no value uses the full terminal width.
- `--wrap N` uses N cells as the wrap width. N replaces the terminal width.

Without `--wrap`, the output does not change. It is byte-identical to the current output.

## 2. Terms

| Term | Meaning |
|---|---|
| **Line** | One source line of Markdown, ended by a newline. |
| **Row** | One physical row of output on the terminal. |
| **Width** | A number of terminal cells, as `unicode-width` measures them. |
| **Wrap width** | The maximum width of a row, prefix included. |
| **Prefix** | The text that mdstream writes before the content of a line: the pad, the list marker, the blockquote bar. |
| **Budget** | The width that is available for content on one row. Budget = wrap width − prefix width. |
| **Word** | A sequence of characters that has no whitespace in it. |
| **Auto mode** | `--wrap` with no value. The wrap width comes from the terminal. |
| **Fixed mode** | `--wrap N`. The wrap width is N. |

## 3. Behavior

### 3.1 Flag forms

| Input | Mode | Wrap width |
|---|---|---|
| (no flag) | off | none. No wrap. |
| `--wrap` | auto | terminal width + offset |
| `--wrap auto` | auto | terminal width + offset |
| `--wrap 80` | fixed | 80 |
| `--wrap=80` | fixed | 80 |
| `MDSTREAM_WRAP=auto` | auto | terminal width + offset |
| `MDSTREAM_WRAP=80` | fixed | 80 |

Rules:

1. N must be an integer from 10 to 1000. For all other values, clap shows an error and mdstream stops with exit code 2.
2. The flag on the command line overrides the env var.
3. In fixed mode, mdstream does not read the terminal width. Fixed mode also works when the output is not a TTY, for example in a pipe or a file.
4. In fixed mode, mdstream ignores `--wrap-width-offset`.
5. If the terminal is narrower than N in fixed mode, the terminal also wraps the rows. This is expected. The user selected N.

### 3.2 Offset flag

| Flag | Env | Default | Description |
|---|---|---|---|
| `--wrap-width-offset N` | `MDSTREAM_WRAP_WIDTH_OFFSET` | `0` | Cells to add (+) or remove (−) from the terminal width in auto mode. |

### 3.3 Width in auto mode

1. Measure the terminal width when mdstream renders each line. Do not keep the width from an earlier line.
2. Get the terminal width from `COLUMNS` first. If `COLUMNS` is not set, get it from `/dev/tty`.
3. If neither source gives a width, do not wrap.
4. Do not use the 80-column fallback in `term_width()` for wrapping.

### 3.4 Scope

Wrap these blocks:

- Paragraph lines.
- List items: bullet, ordered and task.
- Blockquote lines, including blockquotes that contain lists.

Do not wrap these blocks:

- Fenced code blocks.
- Tables. `--table-fit` controls tables.
- Headings.
- Thematic breaks.
- The live partial row. It keeps the tail-window behavior from 0.3.4.

### 3.5 Algorithm

Use greedy (first-fit) wrapping. Greedy wrapping does not need the full paragraph, so it works with streamed input.

1. Split the content into words and whitespace runs. Treat ANSI escape sequences as zero width.
2. Add words to the current row while they fit in the budget.
3. If the next word does not fit, start a new row.
4. At a break, remove the whitespace. Do not write trailing spaces on a row.
5. If one word is wider than the budget, break the word at the budget. This rule also applies to URLs and inline code.
6. Do not split a base character from its variation selector (U+FE0E, U+FE0F).
7. Do not split a combining mark from its base character.
8. Wrap each line by itself. Do not join two source lines into one row.
9. If the budget is less than 10 cells, do not wrap that line.

### 3.6 Styles

1. If a style is active at a break, close the style at the end of the row.
2. Open the same style again after the prefix on the next row.
3. Each wrapped row ends with a reset (`\x1b[0m`).
4. Use the style logic that `wrap_styled_cell` already has.

### 3.7 Prefixes

| Block | First row | Continuation rows |
|---|---|---|
| Paragraph | pad | pad |
| Bullet item | indent + `• ` | indent + spaces, same width as `• ` |
| Ordered item | indent + `12. ` | indent + spaces, same width as `12. ` |
| Task item | indent + `☐ ` | indent + spaces, same width as `☐ ` |
| Blockquote | `│ ` | `│ ` (same style) |
| Blockquote + list | `│ ` + item prefix | `│ ` + spaces |
| List with guides | guides + marker | guides + spaces |

1. Use the real marker that mdstream writes.
2. Measure the marker width with `unicode-width`.
3. Keep list indent guides (`--no-list-guides` off) on continuation rows.

## 4. Tasks

Do the tasks in this order. Each task has acceptance criteria (AC). A task is complete when all of its AC pass.

### T1. Add the `WrapMode` type

Add this type in `src/cli.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WrapMode {
    Auto,
    Fixed(usize),
}
```

Add a `FromStr` implementation. It accepts `auto` or an integer from 10 to 1000.

- **T1-AC1.** `"auto"` parses to `WrapMode::Auto`.
- **T1-AC2.** `"80"` parses to `WrapMode::Fixed(80)`.
- **T1-AC3.** `"10"` and `"1000"` parse. `"9"`, `"1001"`, `"0"`, `"-5"`, `""` and `"abc"` return an error.
- **T1-AC4.** The error message shows the permitted values.

### T2. Add the flags

Add these fields to `Cli` in `src/cli.rs`, under a new help heading, "Text":

```rust
#[arg(
    long,
    env = "MDSTREAM_WRAP",
    num_args = 0..=1,
    default_missing_value = "auto",
    value_name = "auto|COLS",
    help_heading = "Text",
    help = "Wrap long text lines. No value uses the terminal width. COLS sets a fixed width."
)]
pub wrap: Option<WrapMode>,

#[arg(
    long,
    env = "MDSTREAM_WRAP_WIDTH_OFFSET",
    default_value_t = 0,
    allow_negative_numbers = true,
    value_name = "CELLS",
    help_heading = "Text",
    help = "Cells to add (+) or remove (-) from the terminal width when --wrap has no value."
)]
pub wrap_width_offset: i32,
```

- **T2-AC1.** With no flag and no env var, `cli.wrap` is `None`.
- **T2-AC2.** `--wrap` gives `Some(WrapMode::Auto)`.
- **T2-AC3.** `--wrap 80` and `--wrap=80` give `Some(WrapMode::Fixed(80))`.
- **T2-AC4.** `--wrap --padding 2` gives `Some(WrapMode::Auto)` and padding 2. The next flag is not read as a value.
- **T2-AC5.** `MDSTREAM_WRAP=80` gives `Some(WrapMode::Fixed(80))`.
- **T2-AC6.** `MDSTREAM_WRAP=80` with `--wrap` on the command line gives `Some(WrapMode::Auto)`.
- **T2-AC7.** `--wrap 5` stops with exit code 2 and an error message.
- **T2-AC8.** `--wrap-width-offset -10` gives -10.

### T3. Send the values to the renderer

In `src/lib.rs`, send `cli.wrap` and `cli.wrap_width_offset` to the renderer. Use the same pattern as `set_table_width_offset`.

- **T3-AC1.** The renderer has `set_wrap(Option<WrapMode>)` and `set_wrap_width_offset(i32)`.
- **T3-AC2.** The renderer default is `None` and `0`.

### T4. Add the wrap width function

In `src/renderer.rs`, add `fn wrap_width(&self) -> Option<usize>`:

| `wrap` | Result |
|---|---|
| `None` | `None` |
| `Some(Fixed(n))` | `Some(n)` |
| `Some(Auto)` | detected width + offset. `None` if no width is detected or the result is 0 or less. |

Add `fn detected_term_width(&self) -> Option<usize>`. It uses the test override, then `COLUMNS`, then `terminal::size()`. It returns `None` if all three fail. It does not return 80. Change `term_width()` to call `detected_term_width().unwrap_or(80)`, so that its behavior does not change.

- **T4-AC1.** `Fixed(80)` returns `Some(80)` when `COLUMNS=200`.
- **T4-AC2.** `Fixed(80)` returns `Some(80)` when no width is detected.
- **T4-AC3.** `Auto` with `COLUMNS=120` and offset -10 returns `Some(110)`.
- **T4-AC4.** `Auto` with no detected width returns `None`.
- **T4-AC5.** `Auto` with offset -500 returns `None`.
- **T4-AC6.** All existing callers of `term_width()` get the same values as before.

### T5. Make `wrap_styled` general

In `src/renderer.rs`, change `wrap_styled_cell(text, width)` into:

```rust
fn wrap_styled(text: &str, first_width: usize, next_width: usize) -> Vec<String>
```

`first_width` is the budget of the first row. `next_width` is the budget of all other rows. Keep `wrap_styled_cell(text, w)` as a call to `wrap_styled(text, w, w)`.

- **T5-AC1.** All existing tests in `wrap_styled_cell_tests` pass with no changes.
- **T5-AC2.** All existing table snapshots pass with no changes.
- **T5-AC3.** A new unit test with different `first_width` and `next_width` gives rows with those widths.
- **T5-AC4.** No row has trailing spaces.

### T6. Apply wrapping in `render_line`

In `render_line`, for each block in section 3.4:

1. Get `wrap_width()`. If it is `None`, do the current rendering.
2. Calculate the first-row prefix and the continuation prefix (section 3.7).
3. Calculate each budget: wrap width − prefix width.
4. If a budget is less than 10, do the current rendering.
5. Call `wrap_styled` on the styled content.
6. Join the rows with `\n` and the continuation prefix.

- **T6-AC1.** With `wrap` set to `None`, all existing snapshots pass with no changes.
- **T6-AC2.** Code blocks, tables, headings and thematic breaks give the same output with `--wrap` on and off.
- **T6-AC3.** The live partial row gives the same output with `--wrap` on and off.

### T7. Update help, README and CHANGELOG

1. **`src/help.rs`:** add the two flags under "Text". This file is maintained by hand.
2. **README:** add the two flags to the flags table.
3. **CHANGELOG:** add an entry for the next version.
4. **BUGS.md:** set [064] to "fixed (opt-in, `--wrap`)". In the "by-design" section, write that the default does not change and that `--wrap` is available.

- **T7-AC1.** `mdstream --help` shows `--wrap [auto|COLS]` and `--wrap-width-offset`.
- **T7-AC2.** The help guard tests pass.
- **T7-AC3.** README shows both flags with their env vars and defaults.

### T8. Add integration tests

Add `tests/wrap.rs`. Use `set_term_width_override_for_tests` for auto mode. Use the binary for flag and env tests.

- **T8-AC1.** Off: input with long lines gives output that is byte-identical to the current output.
- **T8-AC2.** Auto, width 40: a 106-character paragraph breaks at spaces. No row is wider than 40.
- **T8-AC3.** Fixed 60, override width 200: no row is wider than 60.
- **T8-AC4.** Fixed 60, override width 30: rows are up to 60 wide. The fixed width wins.
- **T8-AC5.** Fixed 60, output to a pipe with no `COLUMNS`: the text wraps at 60.
- **T8-AC6.** Auto, output to a pipe with no `COLUMNS`: the text does not wrap.
- **T8-AC7.** Auto, width 120, offset -20: no row is wider than 100.
- **T8-AC8.** Fixed 60, offset -20: no row is wider than 60. The offset has no effect.
- **T8-AC9.** No row has trailing spaces.
- **T8-AC10.** A word wider than the budget breaks at the budget.
- **T8-AC11.** A bold phrase across a break is bold on both rows. Each wrapped row ends with `\x1b[0m`.
- **T8-AC12.** Bullet, ordered and task items: continuation rows start at the column of the item text.
- **T8-AC13.** Nested list, 3 levels: continuation rows start at the correct column at each level. Guides show on continuation rows.
- **T8-AC14.** Blockquote: each row starts with `│ `.
- **T8-AC15.** Blockquote that contains a list: each row starts with `│ `, then the correct indent.
- **T8-AC16.** CJK text: no row is wider than the wrap width.
- **T8-AC17.** `⚠️` (with VS16) is not split across rows.
- **T8-AC18.** `--padding 4`: the first row and continuation rows start with 4 spaces. No row is wider than the wrap width.
- **T8-AC19.** Budget less than 10 (deep nesting, fixed 20): the line does not wrap.

## 5. Final acceptance

The feature is complete when all of these pass:

1. **AC-1.** `cargo test` passes. No existing snapshot changes.
2. **AC-2.** `cargo clippy --all-targets -- -D warnings` passes.
3. **AC-3.** All AC in T1 to T8 pass.
4. **AC-4.** `Cargo.toml` has no new dependencies.
5. **AC-5.** Without `--wrap`, output is byte-identical to the previous release for all test inputs.
6. **AC-6.** With `--wrap N`, no row in paragraphs, lists or blockquotes is wider than N.
7. **AC-7.** With `--wrap` and a detected width W, no row in those blocks is wider than W + offset.

## 6. Limits

1. **CJK break points.** Text breaks only at spaces. CJK text with no spaces gets a hard break at the budget. A later change can add UAX #14 rules.
2. **Resize.** In auto mode, rows that mdstream already wrote do not change when the terminal width changes. Only new lines use the new width.
3. **Copy and paste.** With `--wrap`, copied text contains the inserted newlines and prefixes. This is the reason that `--wrap` is opt-in.
4. **Fixed mode on a narrow terminal.** If N is wider than the terminal, the terminal also wraps the rows.
