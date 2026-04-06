mod support;

use mdstream::renderer::StreamingMarkdownRenderer;
use support::strip_ansi;

#[test]
fn table_promotion_alignment_and_repaint() {
    let mut renderer = StreamingMarkdownRenderer::new(0, true, true);
    renderer.set_term_width_override_for_tests(120);

    let first = renderer.render_line("| Language | Score |\n");
    let promoted = renderer.render_line("| :--- | ---: |\n");
    let repainted = renderer.render_line("| Python | 10 |\n");
    let closed = renderer.render_line("after\n");

    assert_eq!(strip_ansi(&first), "| Language | Score |\n");
    assert!(promoted.contains("\x1b[1A\r\x1b[J"));
    assert!(repainted.contains("\x1b[2A\r\x1b[J"));
    let clean_promoted = strip_ansi(&promoted);
    let clean_repainted = strip_ansi(&repainted);
    assert!(clean_promoted.contains(" Language │ Score "));
    assert!(clean_promoted.contains("━━━━━━━━━━┿━━━━━━━"));
    assert!(clean_repainted.contains(" Python   │    10 "));
    assert_eq!(strip_ansi(&closed), "after\n");
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

    // Feed in a small flag table. Each row becomes a separate render_line
    // call; the final rendered table is the output of the last extend.
    let _ = renderer.render_line("| Country        | Region        |\n");
    let _ = renderer.render_line("|----------------|---------------|\n");
    let _ = renderer.render_line("| 🇺🇸 United States | North America |\n");
    let _ = renderer.render_line("| 🇨🇳 China         | Asia          |\n");
    let last = renderer.render_line("| 🇬🇧 United Kingdom | Europe        |\n");

    // The final rendered table is in `last`. Strip ANSI and split into
    // visible lines. Every non-empty line must have the same visible width.
    let plain = strip_ansi(&last);
    let lines: Vec<&str> = plain.split('\n').filter(|l| !l.is_empty()).collect();

    assert!(
        lines.len() >= 5,
        "expected header + sep + 3 body rows, got {}: {plain:?}",
        lines.len()
    );

    // Use the same unicode-width path mdstream itself uses.
    use unicode_width::UnicodeWidthStr;
    let widths: Vec<usize> = lines.iter().map(|l| l.width()).collect();
    let header_width = widths[0];
    for (i, &w) in widths.iter().enumerate() {
        assert_eq!(
            w, header_width,
            "row {i} has visible width {w}, expected {header_width}; row: {:?}",
            lines[i]
        );
    }
}

#[test]
fn candidate_clears_if_not_promoted() {
    let mut renderer = StreamingMarkdownRenderer::new(0, true, true);
    let first = renderer.render_line("| maybe | table |\n");
    let second = renderer.render_line("not a separator\n");

    assert_eq!(strip_ansi(&first), "| maybe | table |\n");
    assert_eq!(strip_ansi(&second), "not a separator\n");
}
