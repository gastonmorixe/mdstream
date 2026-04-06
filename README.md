<div align="center">

# 🌊 mdstream

### Beautiful, byte-by-byte Markdown for your terminal.

*Pipe in a stream. Watch it bloom in full color, in real time.*

**⚡ Fast** &nbsp;·&nbsp; **🎨 24-bit color** &nbsp;·&nbsp; **🪄 Live re-render** &nbsp;·&nbsp; **📡 Real-time** &nbsp;·&nbsp; **🦀 Pure Rust**

[![Release](https://img.shields.io/github/v/tag/gastonmorixe/mdstream?label=release&color=blue&logo=rust&logoColor=white)](https://github.com/gastonmorixe/mdstream/releases)
[![Edition](https://img.shields.io/badge/edition-2024-orange?logo=rust&logoColor=white)](https://doc.rust-lang.org/edition-guide/rust-2024/index.html)
[![MSRV](https://img.shields.io/badge/MSRV-1.94%2B-blueviolet?logo=rust&logoColor=white)](https://blog.rust-lang.org/)
[![Stars](https://img.shields.io/github/stars/gastonmorixe/mdstream?style=flat&logo=github&logoColor=white)](https://github.com/gastonmorixe/mdstream/stargazers)

</div>

mdstream renders Markdown **the moment it arrives**. Pipe a partial document in and watch each line resolve into styled output as soon as its newline lands. Tables *repaint in place* when new rows show up. Code blocks pick up syntax highlighting. It's built to pair with **LLM CLIs** and anything else that emits text **a token at a time**.

## Why mdstream

Most terminal Markdown renderers **wait for stdin to close** before drawing anything. That's fine for files. It's painful for `llm "explain this"`, where output trickles in *word by word* and you want to read along.

mdstream takes the **opposite approach**. Raw partial lines stream straight to the terminal so you see characters appear *instantly*. The moment a newline arrives, the line is **erased and re-rendered** with full styling. Block-level state (tables, code fences, lists) persists across the stream so everything stays coherent. The result feels like a *chat UI*, but it's a plain terminal pipeline.

## Features

- 🌊 **Hybrid streaming**: raw character echo while bytes arrive, full re-render the moment a newline lands. EOF flushes any unterminated tail.
- 🎨 **24-bit true color**: heading levels, list bullets, code highlighting, link styles, and table separators all carry SGR colors.
- 📰 **Six heading levels**: distinct color for each. H1 underlined with `━`, H2 with a dimmed `─`, H3-H6 bold-colored only.
- 📋 **Lists**: depth-rotating bullets (`•◦▪‣`), vertical indent guides, hierarchical ordered numbering (write `1.2.3. Section` literally and it renders), task checkboxes (`☑` / `☐`), and continuation-line alignment.
- 📊 **Pipe tables**: promotion on the separator row, in-place repaint as new rows arrive, three alignment modes (left, right, center), and Unicode-correct column widths.
- 💻 **Fenced code blocks**: `syntect`-backed highlighting with selectable built-in themes, themed code backgrounds on by default, optional line numbers, and language-specific label colors for ~30 common languages including Rust, Python, JS/TS, Go, Ruby, Java, Swift, Bash, Zig, Elixir, Haskell, and friends.
- ✨ **Inline formatting**: bold, italic, bold+italic, strikethrough, code spans, links, images, autolinks, bare URLs (with adjacency guard), and backslash escapes for the full Markdown punctuation set.
- 💬 **Blockquotes**: depth tracking, tab- and unicode-whitespace-tolerant parsing, and dimmed `│` bars.
- 🌐 **True Unicode display width**: via `unicode-width`, so CJK, fullwidth digits, and emoji all align correctly inside tables and wrap math.
- 🛡️ **Broken-pipe safe**: piping into `head` or quitting `less` exits cleanly with code 0.
- 📦 **Single static binary**: pure Rust, no C dependencies (`syntect` runs in `default-fancy` mode).

## Install

```bash
git clone https://github.com/gastonmorixe/mdstream
cd mdstream
cargo install --path .
```

The `mdstream` binary lands in `~/.cargo/bin`. Make sure that directory is on your `$PATH`.

## Quick examples

```bash
# Pair with an LLM CLI
llm "explain quicksort" | mdstream

# Render a file
mdstream < README.md

# Stream a remote document
curl -sL https://raw.githubusercontent.com/owner/repo/main/README.md | mdstream

# Page a long document (note the -R for ANSI escapes)
mdstream < long.md | less -R

# Indent for centered terminals
MDSTREAM_PADDING=4 mdstream < README.md

# Hide line numbers in code blocks
mdstream --no-lineno < script.md

# Switch code highlighting theme
mdstream --theme solarized-dark < README.md

# Keep syntax colors but disable code-block backgrounds
mdstream --no-code-background < README.md

# Fake a slow stream and watch lines bloom in real time
{ for s in '# Hello' '' '- one' '- two' '- three'; do echo "$s"; sleep 0.5; done; } | mdstream
```

## What gets rendered

| Element | Markdown | Rendered as |
|---|---|---|
| Heading H1 | `# Title` | bold red, underlined with `━` |
| Heading H2 | `## Title` | bold orange, underlined with dim `─` |
| Heading H3-H6 | `### Title` | bold colored text (green, blue, purple, magenta) |
| Horizontal rule | `---` | dim 40-char `─` |
| Unordered list | `- item` | depth-rotating bullet (`•◦▪‣`) with vertical guides |
| Ordered list | `1. item` | bold marker; literal `1.2.3.` is also accepted |
| Task list | `- [x] done` | green `☑` / dim `☐` |
| Blockquote | `> quoted` | dim `│` prefix, depth-aware |
| Code span | `` `code` `` | cyan on a dark grey background |
| Bold | `**text**` | bold |
| Italic | `*text*` | italic |
| Bold + italic | `***text***` | bold and italic |
| Strikethrough | `~~text~~` | strikethrough |
| Link | `[label](url)` | underlined blue label, dim url in parens |
| Image | `![alt](url)` | `Image:` tag, magenta label, dim url |
| Bare URL | `https://...` | underlined blue (skipped if glued to a word) |
| Pipe table | `\| h \| ... \|` | bold centered header, `━`/`─` separators, `│` columns |
| Code fence | ```` ```rust ```` | colored language label, line numbers, syntect theme colors and backgrounds |

## Configuration

mdstream takes flags or environment variables. **Flags win** when both are set.

| Flag | Environment variable | Default | Description |
|---|---|---|---|
| `--padding N` | `MDSTREAM_PADDING` | `0` | Left padding in spaces |
| `--no-lineno` | `MDSTREAM_NO_LINENO` | off | Hide line numbers in fenced code blocks |
| `--no-list-guides` | `MDSTREAM_NO_LIST_GUIDES` | off | Hide vertical guides on nested lists |
| `--theme THEME` | `MDSTREAM_THEME` | `base16-ocean-dark` | Syntect theme for fenced code blocks |
| `--no-code-background` | `MDSTREAM_NO_CODE_BACKGROUND` | off | Disable themed backgrounds in fenced code blocks |

Run `mdstream --help` for the full surface.

Built-in themes:
`inspired-github`, `solarized-dark`, `solarized-light`, `base16-eighties-dark`, `base16-mocha-dark`, `base16-ocean-dark`, `base16-ocean-light`.

If you run `mdstream` without piping anything in, it prints a usage banner to stderr and exits **non-zero**, so shell scripts can detect the misuse.

## How it works

mdstream is a **line-oriented state machine**, not a full Markdown parser. It reads stdin in **4 KB chunks**, splits on newlines, and routes each completed line through one of a handful of render paths based on what it looks like: heading, list item, table row, code line, blockquote, paragraph. Partial lines (the bytes after the last newline) print as-is, then get *erased and re-rendered* the moment their newline arrives.

That structure keeps the renderer **fast and small**. There's no AST. No parser stack. Each block type carries just enough state to do its job: lists track nesting depth, tables buffer rows so they can be repainted in place, code fences own an active `syntect` highlighter for the current language. **UTF-8 chunk boundaries** are buffered until the bytes are complete, so multi-byte characters split across chunks render correctly. Wrapped partial lines erase via cursor-up + clear-to-end-of-screen.

All **18 regex patterns** compile *lazily, once*, via `OnceLock`. The `syntect` syntax and theme sets are loaded the same way, on first use. *Cold-start cost stays in the binary, not in the user's reading latency.*

Code highlighting goes through `syntect` with the bundled built-in themes and the `default-fancy` feature set, so the binary stays **pure Rust** with **no C dependencies**. `base16-ocean-dark` is the default, and fenced code now renders the theme background unless you opt out with `--no-code-background`.

If you need a structurally correct **full-document** Markdown parser, look at [`comrak`](https://github.com/kivikakk/comrak) or [`pulldown-cmark`](https://github.com/raphlinus/pulldown-cmark). If you want fast, terminal-shaped, **streaming** rendering, mdstream is built for that one job.

## Build from source

mdstream targets **Rust 1.94 or later** (edition 2024).

```bash
cargo build --release
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

The test suite is **27 checks** across seven integration files plus a lib unit module: streaming chunk handling, every block-level renderer, inline formatting, the table promotion and repaint state machine, fenced code blocks, the CLI surface, the TTY-detection branch, and an *end-to-end snapshot* of a mixed-content document.

## Contributing

Bug reports and pull requests are welcome at [github.com/gastonmorixe/mdstream](https://github.com/gastonmorixe/mdstream). Before opening a PR, please run `cargo fmt`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test`.

## License

[MIT](LICENSE) © [Gaston Morixe](https://gastonmorixe.com)
