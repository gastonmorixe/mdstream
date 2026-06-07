mod support;

use mdstream::renderer::StreamingMarkdownRenderer;
use support::strip_ansi;

#[test]
fn renders_basic_structures() {
    let mut renderer = StreamingMarkdownRenderer::new(0, true, true);

    assert_eq!(
        strip_ansi(&renderer.render_line("# Title\n")),
        "Title\n━━━━━\n"
    );
    assert_eq!(
        strip_ansi(&renderer.render_line("- [x] done\n")),
        "  ✔ done\n"
    );
    assert_eq!(strip_ansi(&renderer.render_line("- item\n")), "  • item\n");
    assert_eq!(
        strip_ansi(&renderer.render_line("1. item\n")),
        "  1. item\n"
    );
}

#[test]
fn renders_blockquotes_and_heading_spacing() {
    let mut renderer = StreamingMarkdownRenderer::new(0, true, true);

    assert_eq!(strip_ansi(&renderer.render_line("> tip\n")), "  │ tip\n");
    assert_eq!(strip_ansi(&renderer.render_line(">\n")), "  │\n");
    assert_eq!(
        strip_ansi(&renderer.render_line("> - item\n")),
        "  │   • item\n"
    );
    assert_eq!(
        strip_ansi(&renderer.render_line("> > 1. nested\n")),
        "  │ │   1. nested\n"
    );

    let paragraph = strip_ansi(&renderer.render_line("Paragraph\n"));
    let heading = strip_ansi(&renderer.render_line("## Next Section\n"));
    assert_eq!(paragraph, "Paragraph\n");
    assert!(heading.starts_with("\nNext Section\n"));
}

#[test]
fn nested_unordered_lists_use_depth_specific_bullets() {
    let mut renderer = StreamingMarkdownRenderer::new(0, true, true);

    let l0 = strip_ansi(&renderer.render_line("- a\n"));
    let l1 = strip_ansi(&renderer.render_line("  - b\n"));
    let l2 = strip_ansi(&renderer.render_line("    - c\n"));
    let l3 = strip_ansi(&renderer.render_line("      - d\n"));
    let l1b = strip_ansi(&renderer.render_line("  - e\n"));

    assert_eq!(l0, "  • a\n");
    assert_eq!(l1, "  │ ◦ b\n");
    assert_eq!(l2, "  │ │ ▪ c\n");
    assert_eq!(l3, "  │ │ │ ‣ d\n");
    assert_eq!(l1b, "  │ ◦ e\n");
}

#[test]
fn nested_ordered_lists_render_hierarchical_markers() {
    let mut renderer = StreamingMarkdownRenderer::new(0, true, true);

    let l0 = strip_ansi(&renderer.render_line("1. one\n"));
    let l1 = strip_ansi(&renderer.render_line("  1. two\n"));
    let l2 = strip_ansi(&renderer.render_line("    1. three\n"));

    assert_eq!(l0, "  1. one\n");
    assert_eq!(l1, "  │ 1.1 two\n");
    assert_eq!(l2, "  │ │ 1.1.1 three\n");
}

#[test]
fn list_guides_disabled_via_constructor() {
    let mut renderer = StreamingMarkdownRenderer::new(0, true, false);

    let l0 = strip_ansi(&renderer.render_line("- a\n"));
    let l1 = strip_ansi(&renderer.render_line("  - b\n"));
    let l2 = strip_ansi(&renderer.render_line("    - c\n"));

    assert_eq!(l0, "  • a\n");
    assert_eq!(l1, "    ◦ b\n");
    assert_eq!(l2, "      ▪ c\n");
    assert!(!l1.contains('│'));
    assert!(!l2.contains('│'));
}

#[test]
fn list_continuation_lines_align_under_item_text() {
    let mut renderer = StreamingMarkdownRenderer::new(0, true, true);

    let item = strip_ansi(&renderer.render_line("- one\n"));
    let cont = strip_ansi(&renderer.render_line("  continuation\n"));
    let cont2 = strip_ansi(&renderer.render_line("  still same\n"));
    let next = strip_ansi(&renderer.render_line("- two\n"));

    assert_eq!(item, "  • one\n");
    assert_eq!(cont, "    continuation\n");
    assert_eq!(cont2, "    still same\n");
    assert_eq!(next, "  • two\n");
}

#[test]
fn blockquote_tolerates_tab_and_mixed_whitespace_indent() {
    // Python `_split_blockquote` (mdstream.py:236) uses `^(\s*(?:>\s*)+)`,
    // which accepts tabs, spaces, and other whitespace before `>`. The Rust
    // port previously stripped only ASCII spaces, so tab-indented blockquotes
    // misclassified as plain paragraphs.
    let mut renderer = StreamingMarkdownRenderer::new(0, true, true);

    let tab_indented = strip_ansi(&renderer.render_line("\t> tip\n"));
    assert_eq!(tab_indented, "  │ tip\n");

    let mixed_indented = strip_ansi(&renderer.render_line("  \t  > deeper\n"));
    assert_eq!(mixed_indented, "  │ deeper\n");
}

#[test]
fn ordered_list_accepts_multi_segment_source_marker() {
    // Python `_ORDERED_RE` accepts `1.2.3.` as an explicit hierarchical marker
    // (mdstream.py:153 — `((?:\d+\.)*\d+)`). The Rust port previously matched
    // only a single integer, so `1.2.3. Section` fell through to a paragraph.
    let mut renderer = StreamingMarkdownRenderer::new(0, true, true);

    let line = strip_ansi(&renderer.render_line("1.2.3. Section\n"));
    assert_eq!(line, "  1.2.3 Section\n");
}

#[test]
fn ordered_list_consecutive_multi_segment_markers() {
    let mut renderer = StreamingMarkdownRenderer::new(0, true, true);

    let first = strip_ansi(&renderer.render_line("1.2.3. First\n"));
    let second = strip_ansi(&renderer.render_line("1.2.4. Second\n"));

    assert_eq!(first, "  1.2.3 First\n");
    assert_eq!(second, "  1.2.4 Second\n");
}

#[test]
fn heading_clears_list_state() {
    let mut renderer = StreamingMarkdownRenderer::new(0, true, true);

    strip_ansi(&renderer.render_line("- a\n"));
    strip_ansi(&renderer.render_line("  - b\n"));
    strip_ansi(&renderer.render_line("## Heading\n"));
    let after = strip_ansi(&renderer.render_line("- c\n"));

    // After heading, list state resets so depth goes back to 0
    assert_eq!(after, "  • c\n");
}

#[test]
fn ignores_presentation_only_html_wrapper_lines() {
    let mut renderer = StreamingMarkdownRenderer::new(0, true, true);

    assert_eq!(renderer.render_line("<div align=\"center\">\n"), "");
    assert_eq!(renderer.render_line("</div>\n"), "");

    let heading = strip_ansi(&renderer.render_line("# Title\n"));
    assert_eq!(heading, "Title\n━━━━━\n");
}

// ===========================================================================
// Batch E: trailing whitespace (and a trailing hard-break backslash) on a
// rendered text line must be stripped from the visible output. mdstream emits
// each source line as its own terminal line, so a hard break is implicit; the
// only defect is leaking the trailing spaces / backslash.
// ===========================================================================

#[test]
fn trailing_spaces_stripped_from_paragraph() {
    let mut r = StreamingMarkdownRenderer::new(0, true, true);
    let out = strip_ansi(&r.render_line("line one  \n"));
    assert_eq!(out, "line one\n", "trailing spaces leaked: {out:?}");
}

#[test]
fn trailing_hard_break_backslash_stripped() {
    let mut r = StreamingMarkdownRenderer::new(0, true, true);
    let out = strip_ansi(&r.render_line("line one\\\n"));
    assert_eq!(out, "line one\n", "trailing backslash leaked: {out:?}");
}

#[test]
fn trailing_whitespace_kept_inside_code_block() {
    // Inside fenced code, trailing whitespace is significant and must NOT be
    // stripped (it is content).
    let mut r = StreamingMarkdownRenderer::new(0, false, true);
    let _ = r.render_line("```\n");
    let body = strip_ansi(&r.render_line("code   \n"));
    assert!(body.contains("code   "), "code trailing space wrongly stripped: {body:?}");
}

// ===========================================================================
// Batch K: ATX heading correctness (CommonMark 4.2).
//   - a trailing '#' sequence is only a closer when preceded by a space;
//   - 1-3 leading spaces before the opening '#' are allowed;
//   - heading content gets inline formatting and the rule width counts
//     DISPLAY cells of the *visible* text (not markup chars).
// ===========================================================================

#[test]
fn atx_trailing_hashes_with_space_are_stripped() {
    let mut r = StreamingMarkdownRenderer::new(0, true, true);
    let out = strip_ansi(&r.render_line("## foo ##\n"));
    assert!(out.starts_with("foo\n"), "trailing ## not stripped: {out:?}");
    assert!(!out.contains('#'), "hash leaked: {out:?}");
}

#[test]
fn atx_trailing_hash_without_space_is_literal() {
    // ex75: '# foo#' -> the '#' is part of the text, content not lost.
    let mut r = StreamingMarkdownRenderer::new(0, true, true);
    let out = strip_ansi(&r.render_line("# foo#\n"));
    assert!(out.contains("foo#"), "trailing # wrongly stripped (content loss): {out:?}");
}

#[test]
fn atx_leading_spaces_recognized() {
    let mut r = StreamingMarkdownRenderer::new(0, true, true);
    let out = strip_ansi(&r.render_line("   ### foo\n"));
    assert!(out.starts_with("foo\n"), "indented heading not recognized: {out:?}");
    assert!(!out.contains('#'), "marker leaked: {out:?}");
}

#[test]
fn atx_four_leading_spaces_is_not_heading() {
    // 4 spaces => not a heading (would be indented code in real CM, but at
    // minimum must NOT render as a heading).
    let mut r = StreamingMarkdownRenderer::new(0, true, true);
    let out = strip_ansi(&r.render_line("    # foo\n"));
    assert!(out.contains("# foo"), "4-space line wrongly treated as heading: {out:?}");
}

#[test]
fn atx_heading_formats_inline_markup() {
    let mut r = StreamingMarkdownRenderer::new(0, true, true);
    let raw = r.render_line("# Title with `code`\n");
    let plain = strip_ansi(&raw);
    assert!(plain.starts_with("Title with code"), "code span not formatted in heading: {plain:?}");
    assert!(!plain.contains('`'), "backticks leaked in heading: {plain:?}");
}

#[test]
fn atx_h1_rule_width_matches_visible_text() {
    // '# Title **bold**' -> visible "Title bold" = 10 cells; rule should be 10.
    let mut r = StreamingMarkdownRenderer::new(0, true, true);
    let plain = strip_ansi(&r.render_line("# Title **bold**\n"));
    let lines: Vec<&str> = plain.split('\n').filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 2, "expected title + rule: {plain:?}");
    use unicode_width::UnicodeWidthStr;
    assert_eq!(lines[0].width(), lines[1].width(),
        "rule width != title width: {:?} vs {:?}", lines[0], lines[1]);
}

// ===========================================================================
// Batch J: thematic-break and empty-code-fence rule width should track the
// live terminal width when known (capped), and fall back to 40 when not.
// ===========================================================================

#[test]
fn thematic_break_scales_to_terminal_width() {
    use unicode_width::UnicodeWidthStr;
    let mut r = StreamingMarkdownRenderer::new(0, true, true);
    r.set_term_width_override_for_tests(20);
    let out = strip_ansi(&r.render_line("---\n"));
    let rule = out.trim_end_matches('\n');
    assert_eq!(rule.width(), 20, "HR should be 20 cells at width 20: {rule:?}");
}

#[test]
fn thematic_break_wide_terminal() {
    use unicode_width::UnicodeWidthStr;
    let mut r = StreamingMarkdownRenderer::new(0, true, true);
    r.set_term_width_override_for_tests(100);
    let out = strip_ansi(&r.render_line("***\n"));
    let rule = out.trim_end_matches('\n');
    assert_eq!(rule.width(), 100, "HR should fill width 100: {rule:?}");
}

#[test]
fn thematic_break_falls_back_to_40_when_width_unknown() {
    // No override; in the test process stdout is not a TTY and COLUMNS is
    // typically unset -> detect returns None -> fallback 40.
    // (Guard: only assert when COLUMNS is actually unset.)
    if std::env::var("COLUMNS").is_ok() {
        return;
    }
    let mut r = StreamingMarkdownRenderer::new(0, true, true);
    let out = strip_ansi(&r.render_line("---\n"));
    assert_eq!(out, "────────────────────────────────────────\n");
}
