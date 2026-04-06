mod support;

use mdstream::renderer::StreamingMarkdownRenderer;
use support::strip_ansi;

#[test]
fn renders_code_fences_with_line_numbers() {
    let mut renderer = StreamingMarkdownRenderer::new(0, true, true);

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
    let mut renderer = StreamingMarkdownRenderer::new(0, false, true);
    renderer.render_line("```python\n");
    let line = strip_ansi(&renderer.render_line("print('hello')\n"));

    assert_eq!(line, "  print('hello')\n");
}
