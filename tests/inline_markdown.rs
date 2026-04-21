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
fn inline_code_uses_foreground_only_style() {
    let mut renderer = StreamingMarkdownRenderer::new(0, true, true);

    let rendered = renderer.render_line("`code`\n");

    assert!(!rendered.contains("\x1b[48;"));
    assert!(rendered.contains("\x1b[38;2;180;140;255m"));
    assert_eq!(strip_ansi(&rendered), "code\n");
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
fn nested_stash_does_not_leak_placeholder_markers() {
    // `apply_escapes` stashes backslash escapes first, then `code_span_re`
    // stashes the wrapping code span whose value contains the inner escape
    // placeholder keys. `restore_placeholders` used to iterate forward, so by
    // the time it expanded the outer code-span placeholder, it had already
    // moved past the inner escape placeholders and never re-visited them.
    // The result: literal `\u{0}MDSTREAM0\u{0}` markers in the output, which
    // most terminals display as the visible ASCII `MDSTREAM0`.
    let mut renderer = StreamingMarkdownRenderer::new(0, true, true);

    let rendered = renderer.render_line("`\\| h \\|`\n");

    assert!(
        !rendered.contains('\u{0}'),
        "NUL-bracketed placeholder key leaked into output: {rendered:?}"
    );
    assert!(
        !rendered.contains("MDSTREAM"),
        "literal MDSTREAM marker leaked into output: {rendered:?}"
    );
    assert_eq!(strip_ansi(&rendered), "| h |\n");
}

#[test]
fn nested_stash_survives_link_with_bold_code_escape() {
    // Stress four levels of stashing: a link whose label contains bold which
    // contains a code span which contains an escaped pipe.
    let mut renderer = StreamingMarkdownRenderer::new(0, true, true);

    let rendered = renderer.render_line("[**`\\|`**](https://example.com)\n");

    assert!(
        !rendered.contains('\u{0}'),
        "placeholder key leaked: {rendered:?}"
    );
    assert!(
        !rendered.contains("MDSTREAM"),
        "literal MDSTREAM marker leaked: {rendered:?}"
    );
    // The link label should still contain the resolved pipe character.
    let plain = strip_ansi(&rendered);
    assert!(
        plain.contains('|'),
        "pipe lost during nested stash resolution: {plain:?}"
    );
    assert!(plain.contains("https://example.com"), "URL lost: {plain:?}");
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

#[test]
fn decodes_basic_html_entities() {
    let mut renderer = StreamingMarkdownRenderer::new(0, true, true);

    let rendered = strip_ansi(
        &renderer
            .render_line("**A**&nbsp;&amp;&nbsp;&lt;tag&gt;&nbsp;&quot;x&quot;&nbsp;&#39;y&#39;\n"),
    );

    assert_eq!(rendered, "A & <tag> \"x\" 'y'\n");
}
