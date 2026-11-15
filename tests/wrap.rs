//! `--wrap` integration tests (research/2026-11-15-opt-in-word-wrap/SPEC.md, T8).

use mdstream::cli::WrapMode;
use mdstream::renderer::StreamingMarkdownRenderer;
use std::io::Write;
use std::process::{Command, Stdio};
use unicode_width::UnicodeWidthStr;

const LONG: &str = "The quick brown fox jumps over the lazy dog and then keeps running across the wide open field forever and ever.";

fn strip(text: &str) -> String {
    let re = regex::Regex::new(r"\x1b\[[0-9;?]*[ -/]*[@-~]").unwrap();
    re.replace_all(text, "").into_owned()
}

fn width(row: &str) -> usize {
    UnicodeWidthStr::width(strip(row).as_str())
}

fn renderer(
    padding: usize,
    mode: Option<WrapMode>,
    term: usize,
    offset: i32,
) -> StreamingMarkdownRenderer {
    let mut r = StreamingMarkdownRenderer::new(padding, true, true);
    r.set_term_width_override_for_tests(term);
    r.set_wrap(mode);
    r.set_wrap_width_offset(offset);
    r
}

/// Render `input` line by line and return the raw (styled) output.
fn render(r: &mut StreamingMarkdownRenderer, input: &str) -> String {
    let mut out = String::new();
    for line in input.split_inclusive('\n') {
        out.push_str(&r.render_line(line));
    }
    out
}

fn rows(out: &str) -> Vec<String> {
    out.lines().map(str::to_owned).collect()
}

fn assert_max_width(out: &str, max: usize) {
    for row in rows(out) {
        assert!(
            width(&row) <= max,
            "row wider than {max}: {:?}",
            strip(&row)
        );
    }
}

/// Run the binary with `args` and `env`, `COLUMNS` removed unless set in `env`.
fn run_bin(args: &[&str], env: &[(&str, &str)], input: &str) -> String {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_mdstream"));
    cmd.args(args)
        .env_remove("COLUMNS")
        .env_remove("MDSTREAM_WRAP")
        .env_remove("MDSTREAM_WRAP_WIDTH_OFFSET")
        .env_remove("MDSTREAM_PADDING")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (k, v) in env {
        cmd.env(k, v);
    }
    let mut child = cmd.spawn().unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

const MIXED: &str = "Para one is a long line of text that goes on and on and on past every width.\n\n- item with a long body that goes on and on and on past every width we test\n  - nested item with a long body that goes on and on and on past every width\n1. ordered item with a long body that goes on and on and on past every width\n- [ ] task item with a long body that goes on and on and on past every width\n> quote with a long body that goes on and on and on past every width we test\n\n```\ncode line that goes on and on and on past every width we test here ok\n```\n# Heading that goes on and on and on past every width we test here ok\n\n| a | b |\n|---|---|\n| cell that goes on and on | other cell that goes on and on and on |\n\n---\n";

// T8-AC1
#[test]
fn off_is_byte_identical() {
    let mut base = StreamingMarkdownRenderer::new(0, true, true);
    base.set_term_width_override_for_tests(40);
    let mut off = renderer(0, None, 40, 0);
    assert_eq!(render(&mut base, MIXED), render(&mut off, MIXED));
}

// T8-AC2
#[test]
fn auto_wraps_at_terminal_width() {
    let mut r = renderer(0, Some(WrapMode::Auto), 40, 0);
    let out = render(&mut r, &format!("{LONG}\n"));
    assert!(rows(&out).len() > 2, "{out:?}");
    assert_max_width(&out, 40);
    // Breaks fall at spaces: joined rows give back the source text.
    let joined: Vec<String> = rows(&out).iter().map(|r| strip(r)).collect();
    assert_eq!(joined.join(" "), LONG);
}

// T8-AC3
#[test]
fn fixed_overrides_wide_terminal() {
    let mut r = renderer(0, Some(WrapMode::Fixed(60)), 200, 0);
    let out = render(&mut r, &format!("{LONG}\n"));
    assert!(rows(&out).len() > 1);
    assert_max_width(&out, 60);
}

// T8-AC4
#[test]
fn fixed_overrides_narrow_terminal() {
    let mut r = renderer(0, Some(WrapMode::Fixed(60)), 30, 0);
    let out = render(&mut r, &format!("{LONG}\n"));
    assert_max_width(&out, 60);
    assert!(rows(&out).iter().any(|row| width(row) > 30), "{out:?}");
}

// T8-AC5
#[test]
fn fixed_wraps_in_a_pipe_without_columns() {
    let out = run_bin(&["--wrap", "60"], &[], &format!("{LONG}\n"));
    let body: Vec<&str> = out.lines().filter(|l| !l.is_empty()).collect();
    assert!(body.len() > 1, "{out:?}");
    for row in body {
        assert!(width(row) <= 60, "{row:?}");
    }
}

// T8-AC6
#[test]
fn auto_does_not_wrap_in_a_pipe_without_columns() {
    let out = run_bin(&["--wrap"], &[], &format!("{LONG}\n"));
    assert!(out.lines().any(|l| strip(l) == LONG), "{out:?}");
}

// Env var forms (T2-AC5 / T2-AC6) through the binary.
#[test]
fn env_var_sets_mode_and_flag_wins() {
    let out = run_bin(&[], &[("MDSTREAM_WRAP", "50")], &format!("{LONG}\n"));
    assert!(
        out.lines()
            .filter(|l| !l.is_empty())
            .all(|l| width(l) <= 50),
        "{out:?}"
    );
    // Command-line `--wrap` (auto) beats the env var; no width in a pipe → no wrap.
    let out = run_bin(
        &["--wrap"],
        &[("MDSTREAM_WRAP", "50")],
        &format!("{LONG}\n"),
    );
    assert!(out.lines().any(|l| strip(l) == LONG), "{out:?}");
}

// T2-AC7
#[test]
fn too_small_fixed_width_exits_2() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdstream"))
        .args(["--wrap", "5"])
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("10 to 1000"));
}

// T8-AC7
#[test]
fn auto_applies_offset() {
    let mut r = renderer(0, Some(WrapMode::Auto), 120, -20);
    let out = render(&mut r, &format!("{LONG} {LONG}\n"));
    assert_max_width(&out, 100);
    assert!(rows(&out).iter().any(|row| width(row) > 90), "{out:?}");
}

// T8-AC8
#[test]
fn fixed_ignores_offset() {
    let mut r = renderer(0, Some(WrapMode::Fixed(60)), 200, -20);
    let out = render(&mut r, &format!("{LONG}\n"));
    assert_max_width(&out, 60);
    assert!(rows(&out).iter().any(|row| width(row) > 40), "{out:?}");
}

// T8-AC9
#[test]
fn no_trailing_spaces() {
    let mut r = renderer(0, Some(WrapMode::Auto), 40, 0);
    // Text blocks only: table rows pad cells with spaces by design.
    let text_only = MIXED.split("```").next().unwrap();
    let out = render(&mut r, text_only);
    for row in rows(&out) {
        assert!(!strip(&row).ends_with(' '), "{row:?}");
    }
}

// T8-AC10
#[test]
fn long_word_breaks_at_budget() {
    let word = "x".repeat(70);
    let mut r = renderer(0, Some(WrapMode::Fixed(30)), 200, 0);
    let out = render(&mut r, &format!("see {word} end\n"));
    assert_max_width(&out, 30);
    assert_eq!(
        strip(&out).replace(['\n', ' '], ""),
        format!("see{word}end")
    );
}

// T8-AC11
#[test]
fn bold_carries_across_breaks() {
    let mut r = renderer(0, Some(WrapMode::Fixed(30)), 200, 0);
    let out = render(&mut r, &format!("**{LONG}**\n"));
    let rs = rows(&out);
    assert!(rs.len() > 2);
    for row in &rs {
        assert!(row.contains("\x1b[1m"), "row lost bold: {row:?}");
    }
    for row in &rs[..rs.len() - 1] {
        assert!(
            row.ends_with("\x1b[0m"),
            "wrapped row has no reset: {row:?}"
        );
    }
}

fn continuation_column(out: &str, marker: &str) -> (usize, Vec<usize>) {
    let rs: Vec<String> = rows(out).iter().map(|r| strip(r)).collect();
    let first = &rs[0];
    let byte = first.find(marker).expect("marker") + marker.len();
    let text_col = UnicodeWidthStr::width(&first[..byte]) + 1;
    let cont = rs[1..]
        .iter()
        .map(|r| {
            // Column of the first text character, after spaces and guides.
            let text = r.trim_start_matches([' ', '│']);
            UnicodeWidthStr::width(r.as_str()) - UnicodeWidthStr::width(text)
        })
        .collect();
    (text_col, cont)
}

// T8-AC12
#[test]
fn list_items_hang_under_their_text() {
    for (src, marker) in [
        (format!("- {LONG}\n"), "•"),
        (format!("12. {LONG}\n"), "12."),
        (format!("- [ ] {LONG}\n"), "☐"),
    ] {
        let mut r = renderer(0, Some(WrapMode::Fixed(40)), 200, 0);
        let out = render(&mut r, &src);
        assert_max_width(&out, 40);
        let (col, cont) = continuation_column(&out, marker);
        assert!(!cont.is_empty(), "{out:?}");
        assert!(
            cont.iter().all(|c| *c == col),
            "{marker}: col {col}, got {cont:?}"
        );
    }
}

// T8-AC13
#[test]
fn nested_lists_keep_guides_and_indent() {
    let src = format!("- {LONG}\n  - {LONG}\n    - {LONG}\n");
    let mut r = renderer(0, Some(WrapMode::Fixed(50)), 200, 0);
    let out = render(&mut r, &src);
    assert_max_width(&out, 50);
    let plain: Vec<String> = rows(&out).iter().map(|r| strip(r)).collect();
    let level3_start = plain
        .iter()
        .position(|r| r.contains('▪'))
        .expect("level 3 row");
    // Every row after the level-3 marker shows both guides.
    for row in &plain[level3_start + 1..] {
        assert_eq!(row.matches('│').count(), 2, "{row:?}");
    }
    // Rows of level 2 (between the level-2 marker and level 3) show one guide.
    let level2_start = plain.iter().position(|r| r.contains('│')).unwrap();
    for row in &plain[level2_start..level3_start] {
        assert_eq!(row.matches('│').count(), 1, "{row:?}");
    }
}

// T8-AC14
#[test]
fn blockquote_repeats_bar() {
    let mut r = renderer(0, Some(WrapMode::Fixed(40)), 200, 0);
    let out = render(&mut r, &format!("> {LONG}\n"));
    assert!(rows(&out).len() > 2);
    for row in rows(&out) {
        assert!(strip(&row).starts_with("  │ "), "{row:?}");
    }
    assert_max_width(&out, 40);
}

// T8-AC15
#[test]
fn blockquote_list_keeps_bar_and_indent() {
    let mut r = renderer(0, Some(WrapMode::Fixed(40)), 200, 0);
    let out = render(&mut r, &format!("> - {LONG}\n"));
    let (col, cont) = continuation_column(&out, "•");
    for row in rows(&out) {
        assert!(strip(&row).starts_with("  │ "), "{row:?}");
    }
    assert!(cont.iter().all(|c| *c == col), "col {col}, got {cont:?}");
    assert_max_width(&out, 40);
}

// T8-AC16
#[test]
fn cjk_fits_width() {
    let cjk = "これは日本語の文章です。スペースがありません。".repeat(4);
    let mut r = renderer(0, Some(WrapMode::Fixed(30)), 200, 0);
    let out = render(&mut r, &format!("{cjk} 中文 {cjk}\n"));
    assert!(rows(&out).len() > 2);
    assert_max_width(&out, 30);
}

// T8-AC17
#[test]
fn vs16_emoji_is_not_split() {
    let src = format!("{} ⚠️ Degraded ⚠️ Paused ⚠️ Done\n", "word ".repeat(12));
    for w in 10..40 {
        let mut r = renderer(0, Some(WrapMode::Fixed(w.max(10))), 200, 0);
        let out = render(&mut r, &src);
        assert_max_width(&out, w.max(10));
        for row in rows(&out) {
            let plain = strip(&row);
            assert!(
                !plain.starts_with('\u{FE0F}'),
                "split VS16 at {w}: {plain:?}"
            );
        }
        assert_eq!(strip(&out).matches("⚠️").count(), 3);
    }
}

// T8-AC18
#[test]
fn padding_prefixes_every_row() {
    let mut r = renderer(4, Some(WrapMode::Fixed(40)), 200, 0);
    let out = render(&mut r, &format!("{LONG}\n"));
    assert!(rows(&out).len() > 2);
    for row in rows(&out) {
        let plain = strip(&row);
        assert!(
            plain.starts_with("    ") && !plain.starts_with("     "),
            "{plain:?}"
        );
    }
    assert_max_width(&out, 40);
}

// T8-AC19
#[test]
fn tiny_budget_does_not_wrap() {
    let src = format!("- a\n  - b\n    - c\n      - d\n        - {LONG}\n");
    let mut r = renderer(0, Some(WrapMode::Fixed(20)), 200, 0);
    let out = render(&mut r, &src);
    assert!(
        rows(&out).iter().any(|row| strip(row).ends_with(LONG)),
        "{out:?}"
    );
}

// T6-AC2: code blocks, tables, headings and rules do not change.
#[test]
fn non_text_blocks_unchanged() {
    let src = "```\ncode line that goes on and on and on past every width we test here ok\n```\n# Heading that goes on and on and on past every width we test here ok\n\n| a | b |\n|---|---|\n| cell that goes on and on | other cell that goes on and on and on |\n\n---\n";
    let mut off = renderer(0, None, 40, 0);
    let mut on = renderer(0, Some(WrapMode::Auto), 40, 0);
    assert_eq!(render(&mut off, src), render(&mut on, src));
}

// T6-AC3: the live partial row keeps its tail-window behavior.
#[test]
fn live_partial_unchanged() {
    let chunks = [
        "The quick brown fox ",
        "jumps over the lazy dog and ",
        "keeps running across the field",
    ];
    let drive = |mode: Option<WrapMode>| {
        let mut r = renderer(0, mode, 40, 0);
        let mut out = Vec::new();
        for c in chunks {
            r.write_chunk(c, &mut out).unwrap();
        }
        out
    };
    assert_eq!(drive(None), drive(Some(WrapMode::Auto)));
}
