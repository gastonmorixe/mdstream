# How mdstream's streaming renderer works

`mdstream` is not a full Markdown parser with an AST. It is a
stateful streaming line transformer that reads stdin in chunks,
keeps just enough state to recognize Markdown structures, and
writes ANSI formatted terminal output as soon as it safely can.

## Data flow

1. Entry point: [src/main.rs](../src/main.rs) parses CLI flags and
   hands control to `mdstream::run`.
2. Setup: [src/lib.rs](../src/lib.rs) builds a
   `StreamingMarkdownRenderer` with padding, line number, list
   guide, code theme, and inline code settings.
3. Streaming read loop: `run_reader` in
   [src/renderer.rs](../src/renderer.rs) reads stdin in 4096 byte
   chunks.
4. UTF-8 safety: the same loop keeps a `pending` byte buffer and
   only decodes valid UTF-8 slices, so split multibyte characters
   do not break rendering.
5. Immediate output: decoded text goes to `write_chunk`, which
   prints incomplete trailing text right away, then re-renders it
   once a newline arrives.

That last part is the core realtime trick. If a producer is still
typing a line, `mdstream` shows the raw partial line immediately.
When the newline finally arrives, it erases that provisional text
and replaces it with the formatted line via `erase_partial`.

## How formatting works

- Plain lines: `render_line` routes each complete line based on
  current state.
- Fenced code: `render_code_fence` and `render_code_line` toggle
  code mode, track line numbers, and feed each code line through
  `syntect`.
- Syntax highlight: `make_code_highlighter` picks a syntax and
  theme, normalizes common TypeScript fence labels onto the
  JavaScript syntax when `syntect` does not expose a native token,
  then `as_24_bit_terminal_escaped(...)` turns tokens into ANSI
  color sequences.
- Inline Markdown: `format_inline` handles code spans, links,
  images, URLs, bold, italic, and strike using regex plus
  placeholder stashing, so nested replacements do not stomp on
  each other.
- Headings, rules, lists, blockquotes: `render_content_line` is
  basically a prioritized matcher over one line at a time.
- Nested list behavior: the renderer keeps `list_indent_stack`,
  `list_level_meta`, and `active_list_context` so continuation
  lines and nested numbering stay aligned.
- Tables: unlike most other constructs, tables are buffered. A
  possible header row is parked in `pending_table_header`, then
  rows accumulate in `table_lines`, and the whole table is emitted
  only when the renderer knows the shape.

So the design is:

- Stream immediately when the current bytes are not enough to make
  a structural decision.
- Keep minimal state for constructs that span lines.
- Delay only the structures that require lookahead, mainly tables.
- Never build a full document tree.

## Why this fits realtime output

A normal Markdown renderer wants the whole document first. This
one does not. It keeps just enough mutable state to answer: "Can I
render this line now, or do I need one more line to know what it
is?"

Examples:

- Incomplete line: print it now, repaint later.
- Code fence: enter code mode and highlight line by line.
- Table candidate: hold one line back until the separator line confirms it.
- List continuation: reuse stored indentation context instead of
  reparsing the whole block.

Tests reflect that model too. The `FakeTerminal` in
[tests/support/mod.rs](../tests/support/mod.rs) simulates carriage
return and erase behavior, so tests verify terminal repaint
semantics, not just final strings.
