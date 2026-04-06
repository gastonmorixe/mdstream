mod support;

use mdstream::renderer::StreamingMarkdownRenderer;
use support::strip_ansi;

#[test]
fn renders_inline_markdown_and_rules() {
    let mut renderer = StreamingMarkdownRenderer::new(0, true, true);

    let styled = strip_ansi(&renderer.render_line(
        "***both*** **bold** *italic* ~~gone~~ `code` <https://a.test> https://b.test\n",
    ));
    let rule = strip_ansi(&renderer.render_line("---\n"));

    assert_eq!(
        styled,
        "both bold italic gone code https://a.test https://b.test\n"
    );
    assert_eq!(rule, "────────────────────────────────────────\n");
}

#[test]
fn bare_url_skips_when_preceded_by_word_or_slash() {
    // Python `_BARE_URL_RE` at mdstream.py:149 carries `(?<![\w/])` to avoid
    // linkifying URLs that abut a word character or `/` on the left. The Rust
    // `regex` crate has no lookbehind, so the equivalent guard lives in
    // `format_inline` as a manual pre-match byte check.
    let mut renderer = StreamingMarkdownRenderer::new(0, true, true);

    let glued = renderer.render_line("xhttps://example.com\n");
    // Stripped output should be byte-identical to the input (no styling applied).
    assert_eq!(strip_ansi(&glued), "xhttps://example.com\n");
    // And no ANSI escape sequences should have been injected at all.
    assert!(
        !glued.contains('\x1b'),
        "expected no ANSI escapes, got: {glued:?}"
    );

    let path_glued = renderer.render_line("/foo/https://example.com\n");
    assert_eq!(strip_ansi(&path_glued), "/foo/https://example.com\n");
    assert!(
        !path_glued.contains('\x1b'),
        "expected no ANSI escapes, got: {path_glued:?}"
    );
}

#[test]
fn bare_url_still_formats_after_whitespace_or_at_start() {
    let mut renderer = StreamingMarkdownRenderer::new(0, true, true);

    // Leading-space case: should still get ANSI styling.
    let leading_space = renderer.render_line("see https://example.com now\n");
    assert!(
        leading_space.contains('\x1b'),
        "expected ANSI styling around the URL, got: {leading_space:?}"
    );
    assert_eq!(strip_ansi(&leading_space), "see https://example.com now\n");

    // Start-of-line case: should still get ANSI styling.
    let start_of_line = renderer.render_line("https://example.com\n");
    assert!(
        start_of_line.contains('\x1b'),
        "expected ANSI styling around the URL, got: {start_of_line:?}"
    );
    assert_eq!(strip_ansi(&start_of_line), "https://example.com\n");
}

#[test]
fn renders_links_images_and_escapes() {
    let mut renderer = StreamingMarkdownRenderer::new(0, true, true);

    let image = strip_ansi(&renderer.render_line("![Alt](https://example.com/image.png)\n"));
    let link = strip_ansi(&renderer.render_line("[OpenAI](https://openai.com)\n"));
    let escaped = strip_ansi(&renderer.render_line("\\*not italic\\* and https://example.com\n"));

    assert_eq!(image, "Image: Alt (https://example.com/image.png)\n");
    assert_eq!(link, "OpenAI (https://openai.com)\n");
    assert_eq!(escaped, "*not italic* and https://example.com\n");
}
