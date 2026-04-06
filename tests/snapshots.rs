mod support;

use insta::assert_snapshot;
use mdstream::renderer::StreamingMarkdownRenderer;

#[test]
fn mixed_markdown_document_snapshot() {
    let mut renderer = StreamingMarkdownRenderer::new(0, true, true);
    renderer.set_term_width_override_for_tests(120);

    let mut rendered = String::new();
    for line in [
        "# Title\n",
        "\n",
        "- [x] done\n",
        "> tip\n",
        "\n",
        "| Lang | Score |\n",
        "| :--- | ---: |\n",
        "| Rust | 10 |\n",
        "\n",
        "```python\n",
        "print('hello')\n",
        "```\n",
    ] {
        rendered.push_str(&renderer.render_line(line));
    }

    assert_snapshot!("mixed_markdown_document", rendered);
}
