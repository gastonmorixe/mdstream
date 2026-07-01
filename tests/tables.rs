mod support;

use mdstream::renderer::StreamingMarkdownRenderer;
use support::strip_ansi;

#[test]
fn table_is_buffered_until_closed() {
    let mut renderer = StreamingMarkdownRenderer::new(0, true, true);
    renderer.set_term_width_override_for_tests(120);

    let first = renderer.render_line("| Language | Score |\n");
    let promoted = renderer.render_line("| :--- | ---: |\n");
    let buffered_row = renderer.render_line("| Python | 10 |\n");
    let closed = renderer.render_line("after\n");

    assert_eq!(first, "");
    assert_eq!(promoted, "");
    assert_eq!(buffered_row, "");

    let plain = strip_ansi(&closed);
    assert!(plain.contains(" Language │ Score "));
    assert!(plain.contains("━━━━━━━━━━┿━━━━━━━"));
    assert!(plain.contains(" Python   │    10 "));
    assert!(plain.ends_with("after\n"));
    assert!(!closed.contains("\r\x1b[J"));
}

#[test]
fn flag_emoji_cells_keep_table_columns_aligned() {
    // Regression: a table containing flag emoji (regional-indicator pairs)
    // in one column must render every row — header, separator, and all
    // body rows — at the same visible width, measured consistently so that
    // the vertical dividers line up on any terminal that respects the
    // unicode-width 0.2.x emoji widths (flag = 2 cells).
    let mut renderer = StreamingMarkdownRenderer::new(0, true, true);
    renderer.set_term_width_override_for_tests(120);

    // Feed in a small flag table. Buffered rendering emits the completed
    // table only when the block closes.
    let _ = renderer.render_line("| Country        | Region        |\n");
    let _ = renderer.render_line("|----------------|---------------|\n");
    let _ = renderer.render_line("| 🇺🇸 United States | North America |\n");
    let _ = renderer.render_line("| 🇨🇳 China         | Asia          |\n");
    let _ = renderer.render_line("| 🇬🇧 United Kingdom | Europe        |\n");
    let flushed = renderer.render_line("after\n");

    // The flushed output includes the full table followed by the closing line.
    // Every non-empty table line must have the same visible width.
    let plain = strip_ansi(&flushed);
    let lines: Vec<&str> = plain.split('\n').filter(|l| !l.is_empty()).collect();

    assert!(
        lines.len() >= 8,
        "expected header + sep + 3 body rows + trailing line, got {}: {plain:?}",
        lines.len()
    );

    // Use the same unicode-width path mdstream itself uses.
    use unicode_width::UnicodeWidthStr;
    let table_lines = &lines[..lines.len() - 1];
    let widths: Vec<usize> = table_lines.iter().map(|l| l.width()).collect();
    let header_width = widths[0];
    for (i, &w) in widths.iter().enumerate() {
        assert_eq!(
            w, header_width,
            "row {i} has visible width {w}, expected {header_width}; row: {:?}",
            table_lines[i]
        );
    }
}

#[test]
fn candidate_clears_if_not_promoted() {
    let mut renderer = StreamingMarkdownRenderer::new(0, true, true);
    let first = renderer.render_line("| maybe | table |\n");
    let second = renderer.render_line("not a separator\n");

    assert_eq!(first, "");
    assert_eq!(strip_ansi(&second), "| maybe | table |\nnot a separator\n");
}

#[test]
fn ragged_rows_keep_table_open() {
    // Regression for `tmp/make-a-detailed-plan-virtual-dusk.md`: when the
    // user authored row 13 of a 4-column table with only 3 cells (a
    // missing trailing `| Source |`), pre-fix mdstream flushed the
    // table on that row AND every later 4-cell row rendered as raw
    // markdown text — because once we exit table mode, the only way to
    // re-enter is via a fresh `|---|` separator, which the input
    // doesn't have.
    //
    // GFM behavior: ragged rows are normalized to the header column
    // count — missing cells render blank, extras are dropped. The
    // table stays open until a non-row line (blank line, prose, etc.)
    // closes it.
    let mut renderer = StreamingMarkdownRenderer::new(0, true, true);
    renderer.set_term_width_override_for_tests(120);

    let _ = renderer.render_line("| # | Q | Interpretation | Source |\n");
    let _ = renderer.render_line("|---|---|---|---|\n");
    let _ = renderer.render_line("| 1 | a | aye | user |\n");
    // Row missing trailing cell — only 3 cells.
    let _ = renderer.render_line("| 2 | b | bee |\n");
    // Row with extra cell — 5 cells; extra is dropped.
    let _ = renderer.render_line("| 3 | c | cee | mine | extra |\n");
    let _ = renderer.render_line("| 4 | d | dee | user |\n");
    let closed = strip_ansi(&renderer.render_line("after\n"));

    // No raw markdown should leak.
    for marker in ["| 2 |", "| 3 |", "| 4 |"] {
        assert!(
            !closed.contains(marker),
            "ragged row leaked as raw markdown ({marker:?}): {closed:?}"
        );
    }
    // All four data rows must appear inside the rendered table.
    for token in ["aye", "bee", "cee", "dee"] {
        assert!(
            closed.contains(token),
            "missing row content {token:?}: {closed:?}"
        );
    }
    // The "extra" cell from row 3 is truncated.
    assert!(
        !closed.contains("extra"),
        "extra cell should be dropped, not rendered: {closed:?}"
    );
    // The dropped-trailing-cell on row 2 leaves the Source column blank
    // but the column dividers still align — i.e. we still have a
    // single rendered table, not two flushes.
    let table_starts = closed.matches('━').count();
    assert!(
        table_starts > 0,
        "expected a single rendered table separator: {closed:?}"
    );
    assert!(closed.ends_with("after\n"));
}

#[test]
fn table_accepts_escaped_pipes_inside_code_spans() {
    let mut renderer = StreamingMarkdownRenderer::new(0, true, true);
    renderer.set_term_width_override_for_tests(160);

    let _ = renderer.render_line("| Element | Markdown | Rendered as |\n");
    let _ = renderer.render_line("|---|---|---|\n");
    let _ = renderer.render_line(
        "| Pipe table | `\\| h \\| ... \\|` | bold centered header, `━`/`─` separators, `│` columns |\n",
    );
    let buffered = renderer.render_line(
        "| Code fence | ```` ```rust ```` | colored language label, line numbers, syntect theme colors and backgrounds |\n",
    );
    let after = renderer.render_line("after\n");
    let plain = strip_ansi(&after);

    assert_eq!(buffered, "");
    assert!(
        plain.contains("Pipe table"),
        "missing pipe-table row: {plain:?}"
    );
    assert!(
        plain.contains("Code fence"),
        "missing code-fence row: {plain:?}"
    );
    assert!(
        plain.contains('│'),
        "expected rendered table columns: {plain:?}"
    );
    assert!(plain.ends_with("after\n"));
}

#[test]
fn table_flushes_at_eof() {
    let mut renderer = StreamingMarkdownRenderer::new(0, true, true);

    let _ = renderer.render_line("| A | B |\n");
    let _ = renderer.render_line("|---|---|\n");
    let _ = renderer.render_line("| one | two |\n");

    let mut out = Vec::new();
    renderer.finish(&mut out).unwrap();
    let plain = strip_ansi(std::str::from_utf8(&out).unwrap());

    assert!(plain.contains(" A "));
    assert!(plain.contains(" one │ two "));
}

#[test]
fn single_column_table_is_promoted_and_rendered() {
    // Regression for `tmp/markdown-tables-mock-002.md` "Single mega-cell":
    // 1-column tables ARE valid markdown (GFM, CommonMark Tables ext.,
    // pandoc) but the candidate regex required >= 1 inner pipe and
    // `split_table_row` rejected anything below 2 cells, so `| huge |`
    // never promoted and the raw markdown leaked into output.
    let mut renderer = StreamingMarkdownRenderer::new(0, true, true);
    renderer.set_term_width_override_for_tests(120);

    let _ = renderer.render_line("| huge |\n");
    let _ = renderer.render_line("|------|\n");
    let _ = renderer.render_line("| short cell |\n");
    let closed = strip_ansi(&renderer.render_line("after\n"));

    assert!(
        closed.contains('━'),
        "expected horizontal rule from a promoted table: {closed:?}"
    );
    assert!(closed.contains("huge"), "header missing: {closed:?}");
    assert!(closed.contains("short cell"), "body missing: {closed:?}");
    // Raw markdown must NOT survive promotion.
    assert!(
        !closed.contains("| huge |"),
        "raw markdown leaked: {closed:?}"
    );
    assert!(
        !closed.contains("|------|"),
        "raw separator leaked: {closed:?}"
    );
}

#[test]
fn single_column_mega_cell_caps_at_terminal_width_with_fit() {
    // 1-col table with one very long body cell. Under --table-fit, the
    // cell soft-wraps to the terminal cap and every visual row equals
    // the target width — same contract as multi-column fit-mode.
    let body = "alpha bravo charlie delta echo foxtrot golf hotel india juliet kilo lima mike november oscar papa quebec romeo sierra tango uniform victor whiskey xray yankee zulu alpha bravo charlie delta echo";
    let mut renderer = StreamingMarkdownRenderer::new(0, true, true);
    renderer.set_term_width_override_for_tests(60);
    renderer.set_table_fit(true);

    let _ = renderer.render_line("| huge |\n");
    let _ = renderer.render_line("|------|\n");
    let _ = renderer.render_line(&format!("| {body} |\n"));
    let mut out = Vec::new();
    renderer.finish(&mut out).unwrap();
    let rendered = strip_ansi(std::str::from_utf8(&out).unwrap());

    let table_rows: Vec<&str> = rendered.split('\n').filter(|l| !l.is_empty()).collect();
    assert!(
        table_rows.len() >= 3,
        "expected header + rule + >=1 wrap row: {rendered:?}"
    );
    use unicode_width::UnicodeWidthStr;
    for row in &table_rows {
        assert_eq!(row.width(), 60, "row not 60 cells: {row:?}");
    }
}

// ---------------------------------------------------------------------------
// Terminal-width-aware ("table-fit") rendering. Opt-in via
// `set_table_fit(true)` (mirrors the `--table-fit` / `MDSTREAM_TABLE_FIT`
// flag). When on, every flushed table is allocated against the live
// terminal width, with cells soft-wrapped to keep columns aligned.
// ---------------------------------------------------------------------------

/// Helper: feed a single table and return the rendered output as plain
/// text (ANSI stripped, trailing newline trimmed for asserts).
fn render_table_fit(rows: &[&str], cols: usize, offset: i32) -> String {
    let mut renderer = StreamingMarkdownRenderer::new(0, true, true);
    renderer.set_term_width_override_for_tests(cols);
    renderer.set_table_fit(true);
    renderer.set_table_width_offset(offset);
    let mut last = String::new();
    for row in rows {
        last = renderer.render_line(row);
    }
    let mut out = Vec::new();
    renderer.finish(&mut out).unwrap();
    let tail = String::from_utf8(out).unwrap();
    strip_ansi(&format!("{last}{tail}"))
}

/// Visible width of `line` measured the same way mdstream measures it
/// (after ANSI strip + unicode-width). Used by table-fit asserts to
/// verify each rendered row exactly hits the target width.
fn visible_width(line: &str) -> usize {
    use unicode_width::UnicodeWidthStr;
    line.width()
}

#[test]
fn table_fit_disabled_keeps_content_width() {
    // Without `set_table_fit(true)`, the table must still size to its
    // content even on a wide terminal — preserves the pre-feature
    // default and matches the existing `table_is_buffered_until_closed`
    // contract.
    let mut renderer = StreamingMarkdownRenderer::new(0, true, true);
    renderer.set_term_width_override_for_tests(200);

    let _ = renderer.render_line("| A | B |\n");
    let _ = renderer.render_line("|---|---|\n");
    let _ = renderer.render_line("| one | two |\n");
    let closed = strip_ansi(&renderer.render_line("after\n"));

    let widths: Vec<usize> = closed
        .split('\n')
        .filter(|l| l.contains('│') || l.contains('┿') || l.contains('┼'))
        .map(visible_width)
        .collect();
    assert!(!widths.is_empty(), "expected table rows: {closed:?}");
    // Content-only widths are well below 200.
    for w in &widths {
        assert!(*w < 50, "row width {w} should be content-bounded, not 200");
    }
}

#[test]
fn table_fit_keeps_small_tables_at_natural_width() {
    // `--table-fit` is a *max* constraint, not a fill: tables whose
    // natural width is below the terminal width render at their
    // natural size, just like if fit-mode were off.
    let rendered = render_table_fit(
        &[
            "| A | B |\n",
            "|---|---|\n",
            "| one | two |\n",
            "| three | four |\n",
        ],
        60,
        0,
    );

    let table_lines: Vec<&str> = rendered
        .split('\n')
        .filter(|l| l.contains('│') || l.contains('┿') || l.contains('┼'))
        .collect();

    assert!(!table_lines.is_empty(), "no table lines: {rendered:?}");
    for line in &table_lines {
        assert!(
            visible_width(line) < 60,
            "small table should NOT expand to fill: {line:?}"
        );
    }
}

#[test]
fn table_fit_caps_overflowing_table_at_terminal_width() {
    // When the natural table would overflow, fit-mode caps it at
    // exactly the terminal width and soft-wraps cells.
    let rendered = render_table_fit(
        &[
            "| Tag | Description |\n",
            "|---|---|\n",
            "| ok | one two three four five six seven eight nine ten eleven twelve thirteen fourteen fifteen |\n",
        ],
        40,
        0,
    );
    let table_lines: Vec<&str> = rendered
        .split('\n')
        .filter(|l| l.contains('│') || l.contains('┿') || l.contains('┼'))
        .collect();
    assert!(!table_lines.is_empty(), "no table lines: {rendered:?}");
    for line in &table_lines {
        assert_eq!(
            visible_width(line),
            40,
            "overflowing table should be capped at 40 cols: {line:?}"
        );
    }
}

#[test]
fn table_fit_offset_subtracts_columns() {
    // Negative offset leaves a right gutter; an overflowing table
    // is capped at (term_width + offset) cells.
    let rendered = render_table_fit(
        &[
            "| A | B | C |\n",
            "|---|---|---|\n",
            "| 1 | aaaa bbbb cccc dddd eeee ffff gggg hhhh iiii jjjj kkkk llll mmmm nnnn oooo pppp | 3 |\n",
        ],
        80,
        -10,
    );
    let mut saw_row = false;
    for line in rendered.split('\n').filter(|l| l.contains('│')) {
        saw_row = true;
        assert_eq!(visible_width(line), 70);
    }
    assert!(saw_row, "no body row: {rendered:?}");
}

#[test]
fn table_fit_offset_can_be_positive() {
    // Positive offset over-expands beyond the detected width — symmetric
    // with the negative path. Triggered via overflowing content so the
    // max-mode allocator actually engages.
    let rendered = render_table_fit(
        &[
            "| A | B |\n",
            "|---|---|\n",
            "| x | aaaa bbbb cccc dddd eeee ffff gggg hhhh iiii jjjj kkkk llll mmmm nnnn |\n",
        ],
        50,
        5,
    );
    for line in rendered.split('\n').filter(|l| l.contains('│')) {
        assert_eq!(visible_width(line), 55);
    }
}

#[test]
fn table_fit_soft_wraps_long_cell_content() {
    // The natural width exceeds the target, so the wide column must
    // soft-wrap — header row spans 1 visual line, body row spans
    // multiple, and every visual line lines up at exactly `target`
    // visible cells (so the column dividers stay vertically stacked).
    let rendered = render_table_fit(
        &[
            "| Tag | Description |\n",
            "|---|---|\n",
            "| ok | one two three four five six seven eight nine ten eleven twelve |\n",
        ],
        40,
        0,
    );

    let lines: Vec<&str> = rendered
        .split('\n')
        .filter(|l| l.contains('│') || l.contains('┿'))
        .collect();
    assert!(
        lines.len() >= 3,
        "expected wrapping to add visual rows: {rendered:?}"
    );
    for line in &lines {
        assert_eq!(visible_width(line), 40, "row not 40 cells wide: {line:?}");
    }

    // No mid-word breaks: every word from the source must appear
    // intact somewhere in the rendered text (post strip).
    for word in [
        "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten", "eleven",
        "twelve",
    ] {
        assert!(
            rendered.contains(word),
            "lost word `{word}` to a hard break: {rendered:?}"
        );
    }
}

#[test]
fn table_fit_respects_alignment_per_wrapped_line() {
    // A right-aligned column with a wrapped cell must right-align
    // *each* wrapped visual line; left-aligned similarly. We verify by
    // checking that left-aligned wrapped lines start with content (no
    // leading pad beyond the row's single-cell padding) and
    // right-aligned wrapped lines end with content.
    let rendered = render_table_fit(
        &[
            "| Left | Right |\n",
            "| :--- | ---: |\n",
            "| aaaa bbbb cccc dddd | wwww xxxx yyyy zzzz |\n",
        ],
        30,
        0,
    );
    let body: Vec<&str> = rendered
        .split('\n')
        .filter(|l| l.contains('│') && !l.contains('━'))
        .collect();
    // Header (1 line) + at least 2 wrapped body lines.
    assert!(body.len() >= 3, "expected wrap rows: {rendered:?}");

    // Inspect just the body wrap rows (skip header).
    for line in body.iter().skip(1) {
        let cells: Vec<&str> = line.split('│').collect();
        assert_eq!(cells.len(), 2, "expected 2 cells: {line:?}");
        let left = cells[0];
        let right = cells[1];
        // Left cell: " " (cell pad) then content, then trailing pad of
        // spaces from the left-align padding. The leading pad is
        // exactly one space (the cell's intrinsic left padding).
        assert!(left.starts_with(' '), "left cell missing pad: {left:?}");
        if !left.trim().is_empty() {
            // Exactly one leading space — content begins on cell's
            // left edge under left alignment.
            assert_eq!(
                left.chars().take_while(|c| *c == ' ').count(),
                1,
                "left-aligned cell has unexpected leading whitespace: {left:?}"
            );
        }
        // Right cell: leading pad spaces (right alignment) + content +
        // single trailing space (the cell's intrinsic right padding).
        assert!(right.ends_with(' '), "right cell missing pad: {right:?}");
        if !right.trim().is_empty() {
            // Exactly one trailing space — right-aligned content hugs
            // the right edge of the cell.
            assert_eq!(
                right.chars().rev().take_while(|c| *c == ' ').count(),
                1,
                "right-aligned cell has unexpected trailing whitespace: {right:?}"
            );
        }
    }
}

#[test]
fn table_fit_auto_disables_when_no_width_detectable() {
    // No override, no COLUMNS env var, stdin/stdout aren't TTYs in the
    // test process. Fit-mode must silently fall back to content widths
    // instead of panicking or producing an 80-col fallback table.
    // SAFETY: ensure COLUMNS isn't set by the harness; restored after.
    let prev = std::env::var("COLUMNS").ok();
    unsafe { std::env::remove_var("COLUMNS") };

    let mut renderer = StreamingMarkdownRenderer::new(0, true, true);
    renderer.set_table_fit(true);
    // Deliberately no `set_term_width_override_for_tests` and no
    // COLUMNS — `detect_live_columns` should hit the
    // `!is_terminal()` branch and return None.

    let _ = renderer.render_line("| A | B |\n");
    let _ = renderer.render_line("|---|---|\n");
    let _ = renderer.render_line("| one | two |\n");
    let closed = strip_ansi(&renderer.render_line("after\n"));

    let widths: Vec<usize> = closed
        .split('\n')
        .filter(|l| l.contains('│'))
        .map(visible_width)
        .collect();
    assert!(!widths.is_empty(), "table should still render: {closed:?}");
    // Content-only widths — well under any terminal we'd see.
    for w in widths {
        assert!(w < 30, "expected content-bounded width, got {w}");
    }

    if let Some(v) = prev {
        unsafe { std::env::set_var("COLUMNS", v) };
    }
}

#[test]
fn table_fit_target_zero_or_negative_falls_back() {
    // If `terminal_width + offset - padding <= 0`, the renderer must
    // not divide-by-zero or panic. It auto-disables for that table.
    let mut renderer = StreamingMarkdownRenderer::new(0, true, true);
    renderer.set_term_width_override_for_tests(20);
    renderer.set_table_fit(true);
    // Offset that drives the target to zero.
    renderer.set_table_width_offset(-20);

    let _ = renderer.render_line("| A | B |\n");
    let _ = renderer.render_line("|---|---|\n");
    let _ = renderer.render_line("| one | two |\n");
    let closed = strip_ansi(&renderer.render_line("after\n"));

    // Should fall back to natural widths (small, not panic, not zero).
    let widths: Vec<usize> = closed
        .split('\n')
        .filter(|l| l.contains('│'))
        .map(visible_width)
        .collect();
    assert!(!widths.is_empty(), "table missing: {closed:?}");
    for w in widths {
        assert!((5..30).contains(&w), "width {w} outside fallback range");
    }
}

#[test]
fn table_fit_re_detects_width_each_render() {
    // Spec: "no resizing, this is just a one off calculation at the
    // time each table is rendered." Two consecutive overflowing
    // tables with different `term_width_override` values must be
    // sized independently — i.e. the second table picks up the new
    // width.
    let mut renderer = StreamingMarkdownRenderer::new(0, true, true);
    renderer.set_table_fit(true);

    // First overflowing table at 30 cols.
    renderer.set_term_width_override_for_tests(30);
    let _ = renderer.render_line("| A | B |\n");
    let _ = renderer.render_line("|---|---|\n");
    let _ = renderer.render_line("| one | aaaa bbbb cccc dddd eeee ffff gggg |\n");
    let first = strip_ansi(&renderer.render_line("\n"));

    // Second overflowing table at 50 cols.
    renderer.set_term_width_override_for_tests(50);
    let _ = renderer.render_line("| C | D |\n");
    let _ = renderer.render_line("|---|---|\n");
    let _ = renderer.render_line("| three | wwww xxxx yyyy zzzz aaaa bbbb cccc dddd eeee ffff |\n");
    let second = strip_ansi(&renderer.render_line("after\n"));

    let first_w = first
        .split('\n')
        .filter(|l| l.contains('│'))
        .map(visible_width)
        .next()
        .unwrap();
    let second_w = second
        .split('\n')
        .filter(|l| l.contains('│'))
        .map(visible_width)
        .next()
        .unwrap();
    assert_eq!(first_w, 30);
    assert_eq!(second_w, 50);
}

#[test]
fn table_fit_propagates_styling_across_wrapped_lines() {
    // A bold header that wraps must remain bold on its continuation
    // line. Verify by checking the rendered output (with ANSI intact)
    // contains a `\x1b[1m` marker on the second visual line of the
    // wrapped header.
    let mut renderer = StreamingMarkdownRenderer::new(0, true, true);
    renderer.set_term_width_override_for_tests(24);
    renderer.set_table_fit(true);

    let _ = renderer.render_line("| Population (millions) | X |\n");
    let _ = renderer.render_line("|---|---|\n");
    let _ = renderer.render_line("| 100 | y |\n");
    let mut out = Vec::new();
    renderer.finish(&mut out).unwrap();
    let raw = String::from_utf8(out).unwrap();

    // Find the position of the closing `(millions)` text — it's on a
    // continuation line of the wrapped header. The bold open `\x1b[1m`
    // must appear before it (re-emitted at line start).
    let idx = raw
        .find("(millions)")
        .expect("expected wrapped header continuation");
    let before = &raw[..idx];
    let last_reset = before.rfind("\x1b[0m").unwrap_or(0);
    let after_reset = &raw[last_reset..idx];
    assert!(
        after_reset.contains("\x1b[1m"),
        "bold style not re-emitted on wrapped header line: {after_reset:?}"
    );
}

#[test]
fn table_fit_wraps_cjk_wide_glyphs_correctly() {
    // CJK characters are 2 cells each. Wrapping must respect their
    // width — the rendered row width must still equal the target.
    let rendered = render_table_fit(
        &[
            "| Name | Note |\n",
            "|---|---|\n",
            "| 日本 | 日本語のテキスト 日本語のテキスト 日本語のテキスト |\n",
        ],
        40,
        0,
    );
    for line in rendered.split('\n').filter(|l| l.contains('│')) {
        assert_eq!(visible_width(line), 40, "cjk row not 40 cells: {line:?}");
    }
}

#[test]
fn table_fit_squeeze_branch_handles_too_narrow_target() {
    // Target < sum of column min widths. We don't panic; every
    // column gets at least 1 cell of content; row width still
    // equals target.
    let rendered = render_table_fit(
        &[
            "| Veryverylongheader | Anotherlongheader | Third |\n",
            "|---|---|---|\n",
            "| supercalifragilistic | a | b |\n",
        ],
        20,
        0,
    );
    for line in rendered.split('\n').filter(|l| l.contains('│')) {
        assert_eq!(visible_width(line), 20, "squeeze row off: {line:?}");
    }
}

// --- column-allocation algorithm: white-box checks via the public render path ---

#[test]
fn table_fit_max_mode_does_not_widen_short_tables() {
    // Under the max-not-fill semantic, a small two-column table never
    // expands beyond its natural width even with ample terminal
    // slack. Both columns stay at their content-only sizes.
    let rendered = render_table_fit(
        &["| S | LongerColumn |\n", "|---|---|\n", "| 1 | text |\n"],
        60,
        0,
    );
    let body = rendered
        .split('\n')
        .find(|l| l.contains('│') && l.contains('1'))
        .unwrap();
    assert!(
        visible_width(body) < 60,
        "max-mode widened a fitting table: {body:?}"
    );
    // The wider column is naturally wider than the short one — no
    // expansion happened, but the natural shape is preserved.
    let cells: Vec<&str> = body.split('│').collect();
    assert_eq!(cells.len(), 2);
    let short_len = visible_width(cells[0]);
    let long_len = visible_width(cells[1]);
    assert!(
        long_len > short_len,
        "natural shape lost: short={short_len} long={long_len}"
    );
}

// ---------------------------------------------------------------------------
// Regression: GFM delimiter rows allow ONE hyphen per cell, optionally
// wrapped in colons (`-`, `:-`, `-:`, `:-:`, `--:`, `:--`). mdstream's
// `table_separator_cell_re` used `^:?-{3,}:?$`, demanding >= 3 hyphens, so
// any alignment delimiter with 1-2 dashes failed to promote and the raw
// `| ... |` markdown leaked into the terminal. Reproduced from session
// 517eb994 (a financial scenario grid using `|---|--:|--:|...`).
// ---------------------------------------------------------------------------

/// Helper: render a 3-column table whose delimiter row uses `delim` in
/// each cell and return the closed (flushed) plain-text output.
fn render_with_delim(delim: &str) -> String {
    let mut renderer = StreamingMarkdownRenderer::new(0, true, true);
    renderer.set_term_width_override_for_tests(120);
    let _ = renderer.render_line("| A | B | C |\n");
    let _ = renderer.render_line(&format!("|{delim}|{delim}|{delim}|\n"));
    let _ = renderer.render_line("| 1 | 2 | 3 |\n");
    strip_ansi(&renderer.render_line("after\n"))
}

#[test]
fn short_alignment_delimiters_promote_to_table() {
    for delim in [
        "-", "--", "---", ":-", "-:", ":-:", ":--", "--:", ":---", "---:", ":---:", ":--:",
    ] {
        let out = render_with_delim(delim);
        assert!(
            out.contains('\u{2502}') || out.contains('\u{2500}') || out.contains('\u{2501}'),
            "delimiter {delim:?} failed to promote to a table: {out:?}"
        );
        assert!(
            !out.contains("| A | B | C |"),
            "delimiter {delim:?} leaked raw markdown header: {out:?}"
        );
        assert!(
            !out.contains(&format!("|{delim}|")),
            "delimiter {delim:?} leaked raw separator: {out:?}"
        );
    }
}

#[test]
fn two_dash_alignment_delimiter_preserves_alignment() {
    let mut renderer = StreamingMarkdownRenderer::new(0, true, true);
    renderer.set_term_width_override_for_tests(120);
    let _ = renderer.render_line("| Left | Mid | Right |\n");
    let _ = renderer.render_line("| :-- | :-: | --: |\n");
    let _ = renderer.render_line("| a | b | ccccc |\n");
    let closed = strip_ansi(&renderer.render_line("after\n"));

    assert!(
        closed.contains('\u{2502}'),
        "alignment delimiter row did not promote: {closed:?}"
    );
    assert!(
        !closed.contains("| :-- |"),
        "raw separator leaked: {closed:?}"
    );
    assert!(closed.contains("ccccc"), "body cell missing: {closed:?}");
}

#[test]
fn session_517eb994_scenario_grid_renders_as_table() {
    let mut renderer = StreamingMarkdownRenderer::new(0, true, true);
    renderer.set_term_width_override_for_tests(146);
    let _ = renderer.render_line("| Rate move | IEF | Bond P&L | Collar payoff | **Net P&L** |\n");
    let _ = renderer.render_line("|---|--:|--:|--:|--:|\n");
    let _ = renderer.render_line(
        "| -200bp (rates fall hard) | $106.73 | +$48,553 | -$40,244 | **+$8,309** |\n",
    );
    let _ = renderer.render_line("| 0bp (flat) | $93.62 | $0 | -$555 | **-$555** |\n");
    let closed = strip_ansi(&renderer.render_line("after\n"));

    assert!(
        closed.contains('\u{2502}') && closed.contains('\u{2501}'),
        "scenario grid failed to render as a table: {closed:?}"
    );
    assert!(
        !closed.contains("| Rate move |"),
        "raw markdown header leaked: {closed:?}"
    );
    assert!(
        !closed.contains("|---|--:|"),
        "raw separator leaked: {closed:?}"
    );
    assert!(
        closed.contains("Bond P&L"),
        "header content missing: {closed:?}"
    );
    assert!(
        closed.contains("$106.73"),
        "body content missing: {closed:?}"
    );
}
