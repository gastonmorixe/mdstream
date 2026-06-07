mod support;

use mdstream::renderer::StreamingMarkdownRenderer;
use mdstream::theme::CodeTheme;
use support::strip_ansi;

#[test]
fn renders_code_fences_with_line_numbers() {
    let mut renderer =
        StreamingMarkdownRenderer::with_code_theme(0, true, true, CodeTheme::Base16OceanDark, true);

    let start = strip_ansi(&renderer.render_line("```python\n"));
    let first = strip_ansi(&renderer.render_line("print('hello')\n"));
    let second = strip_ansi(&renderer.render_line("x = 1\n"));
    let end = strip_ansi(&renderer.render_line("```\n"));

    assert!(start.contains("python"));
    assert!(first.contains("  1  print('hello')"));
    assert!(second.contains("  2  x = 1"));
    assert_eq!(end, "────────────────────────────────────────\n");
}

#[test]
fn can_disable_line_numbers() {
    let mut renderer = StreamingMarkdownRenderer::with_code_theme(
        0,
        false,
        true,
        CodeTheme::Base16OceanDark,
        true,
    );
    renderer.render_line("```python\n");
    let line = strip_ansi(&renderer.render_line("print('hello')\n"));

    assert_eq!(line, "  print('hello')\n");
}

#[test]
fn code_fences_render_theme_backgrounds_by_default() {
    let mut renderer =
        StreamingMarkdownRenderer::with_code_theme(0, true, true, CodeTheme::Base16OceanDark, true);
    renderer.render_line("```python\n");
    let line = renderer.render_line("print('hello')\n");

    assert!(line.contains("\x1b[48;2;43;48;59m"));
}

#[test]
fn code_fences_can_disable_theme_backgrounds() {
    let mut renderer = StreamingMarkdownRenderer::with_code_theme(
        0,
        true,
        true,
        CodeTheme::Base16OceanDark,
        false,
    );
    renderer.render_line("```python\n");
    let line = renderer.render_line("print('hello')\n");

    assert!(!line.contains("\x1b[48;2;"));
    assert!(line.contains("\x1b[38;2;"));
}

#[test]
fn code_fences_use_selected_theme() {
    let mut renderer =
        StreamingMarkdownRenderer::with_code_theme(0, true, true, CodeTheme::InspiredGitHub, true);
    renderer.render_line("```python\n");
    let line = renderer.render_line("print('hello')\n");

    assert!(line.contains("\x1b[48;2;255;255;255m"));
}

#[test]
fn default_renderer_uses_mdstream_theme_without_backgrounds() {
    let mut renderer = StreamingMarkdownRenderer::new(0, true, true);
    renderer.render_line("```python\n");
    let line = renderer.render_line("if value == 1:\n");

    assert!(!line.contains("\x1b[48;2;"));
    assert!(line.contains("\x1b[38;2;255;97;172m"));
}

#[test]
fn code_fences_load_catppuccin_mocha() {
    let mut renderer =
        StreamingMarkdownRenderer::with_code_theme(0, true, true, CodeTheme::CatppuccinMocha, true);
    renderer.render_line("```python\n");
    let line = renderer.render_line("if value == 1:\n");

    assert!(line.contains("\x1b[48;2;30;30;46m"));
    assert!(line.contains("\x1b[38;2;203;166;247m"));
}

#[test]
fn code_fences_load_sublime_snazzy() {
    let mut renderer =
        StreamingMarkdownRenderer::with_code_theme(0, true, true, CodeTheme::SublimeSnazzy, true);
    renderer.render_line("```python\n");
    let line = renderer.render_line("if value == 1:\n");

    assert!(line.contains("\x1b[48;2;40;42;54m"));
    assert!(line.contains("\x1b[38;2;255;92;87m"));
}

#[test]
fn code_fences_load_dracula() {
    let mut renderer =
        StreamingMarkdownRenderer::with_code_theme(0, true, true, CodeTheme::Dracula, true);
    renderer.render_line("```python\n");
    let line = renderer.render_line("if value == 1:\n");

    assert!(line.contains("\x1b[48;2;40;42;54m"));
    assert!(line.contains("\x1b[38;2;255;121;198m"));
}

#[test]
fn typescript_fences_fall_back_to_javascript_highlighting() {
    let mut renderer = StreamingMarkdownRenderer::new(0, true, true);
    renderer.render_line("```typescript\n");
    let line = renderer.render_line("function greet(name: string): string {\n");

    assert!(line.contains("\x1b[38;2;123;167;255mfunction"));
    assert!(line.contains("\x1b[38;2;96;214;255mgreet"));
}

// ===========================================================================
// Batch A: CommonMark fence-closer rules. A closing fence must use the SAME
// fence character as the opener, be at least as long, be indented <=3 spaces,
// and carry no trailing non-whitespace text. Opener may not have a backtick
// in its info string (for ``` fences).
// ===========================================================================

#[test]
fn four_backtick_fence_keeps_inner_triple_backticks() {
    // K1: a 4-backtick fence is NOT closed by an inner ``` line.
    let mut r = StreamingMarkdownRenderer::new(0, true, true);
    let mut out = String::new();
    for l in ["````markdown\n", "```rust\n", "fn x(){}\n", "```\n", "````\n", "after\n"] {
        out.push_str(&strip_ansi(&r.render_line(l)));
    }
    assert!(out.contains("```rust"), "inner ```rust must be literal content: {out:?}");
    assert!(out.contains("fn x(){}"), "code body missing: {out:?}");
    // 'after' is outside the code block
    assert!(out.contains("after"), "trailing text missing: {out:?}");
}

#[test]
fn closing_fence_with_trailing_text_is_not_a_closer() {
    // K2: '``` aaa' carries trailing text, so it cannot close; it's content.
    let mut r = StreamingMarkdownRenderer::new(0, true, true);
    let mut out = String::new();
    for l in ["```\n", "``` aaa\n", "```\n", "after\n"] {
        out.push_str(&strip_ansi(&r.render_line(l)));
    }
    assert!(out.contains("aaa"), "content 'aaa' lost: {out:?}");
    assert!(out.contains("after"), "trailing text missing: {out:?}");
}

#[test]
fn backtick_fence_not_closed_by_tilde_line() {
    // A ~~~ line inside a ``` fence is literal content.
    let mut r = StreamingMarkdownRenderer::new(0, true, true);
    let mut out = String::new();
    for l in ["```\n", "aaa\n", "~~~\n", "```\n", "after\n"] {
        out.push_str(&strip_ansi(&r.render_line(l)));
    }
    assert!(out.contains("aaa"), "content missing: {out:?}");
    assert!(out.contains("~~~"), "tilde line must be literal content: {out:?}");
    assert!(out.contains("after"), "trailing text missing: {out:?}");
}

#[test]
fn tilde_fence_not_closed_by_backtick_line() {
    let mut r = StreamingMarkdownRenderer::new(0, true, true);
    let mut out = String::new();
    for l in ["~~~\n", "code\n", "```\n", "more\n", "~~~\n", "after\n"] {
        out.push_str(&strip_ansi(&r.render_line(l)));
    }
    assert!(out.contains("```"), "backtick line must be literal inside ~~~ fence: {out:?}");
    assert!(out.contains("code") && out.contains("more"), "content missing: {out:?}");
    assert!(out.contains("after"), "trailing text missing: {out:?}");
}

#[test]
fn shorter_closing_fence_does_not_close_longer_opener() {
    // Opener ````, a ``` line is too short to close.
    let mut r = StreamingMarkdownRenderer::new(0, true, true);
    let mut out = String::new();
    for l in ["````\n", "aaa\n", "```\n", "bbb\n", "````\n", "after\n"] {
        out.push_str(&strip_ansi(&r.render_line(l)));
    }
    assert!(out.contains("aaa") && out.contains("bbb"), "content missing: {out:?}");
    assert!(out.contains("after"), "trailing text missing: {out:?}");
}
