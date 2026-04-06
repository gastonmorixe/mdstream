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
