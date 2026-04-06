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
fn candidate_clears_if_not_promoted() {
    let mut renderer = StreamingMarkdownRenderer::new(0, true, true);
    let first = renderer.render_line("| maybe | table |\n");
    let second = renderer.render_line("not a separator\n");

    assert_eq!(strip_ansi(&first), "| maybe | table |\n");
    assert_eq!(strip_ansi(&second), "not a separator\n");
}
