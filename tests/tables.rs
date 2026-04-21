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
