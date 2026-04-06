mod support;

use mdstream::renderer::StreamingMarkdownRenderer;
use support::{FakeTerminal, run_reader};

#[test]
fn writes_chunks_and_flushes_final_partial() {
    let mut renderer = StreamingMarkdownRenderer::new(0, true, true);
    let mut out = FakeTerminal::default();

    renderer.write_chunk("Hello", &mut out).unwrap();
    assert_eq!(renderer.partial, "Hello");
    assert_eq!(out.rendered(), "Hello");

    renderer.write_chunk(" world\nNext", &mut out).unwrap();
    assert_eq!(renderer.partial, "Next");
    assert_eq!(out.rendered(), "Hello world\nNext");

    renderer.finish(&mut out).unwrap();
    assert!(renderer.partial.is_empty());
    assert_eq!(out.rendered(), "Hello world\nNext");

    renderer.finish(&mut out).unwrap();
    assert_eq!(out.rendered(), "Hello world\nNext");
}

#[test]
fn handles_chunked_utf8() {
    let mut renderer = StreamingMarkdownRenderer::new(0, true, true);
    let rendered = run_reader(&mut renderer, b"caf\xc3\xa9\nna\xc3\xafve".to_vec());

    assert_eq!(rendered, "\ncafé\nnaïve");
}

#[test]
fn handles_multiple_complete_lines_in_one_chunk() {
    let mut renderer = StreamingMarkdownRenderer::new(0, true, true);
    let mut out = FakeTerminal::default();

    renderer.write_chunk("a\nb\nc\n", &mut out).unwrap();
    assert!(renderer.partial.is_empty());
    assert_eq!(out.rendered(), "a\nb\nc");
}
