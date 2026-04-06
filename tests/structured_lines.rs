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
        "  ☑ done\n"
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
