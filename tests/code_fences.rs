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
