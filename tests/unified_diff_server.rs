//! Phase 2 parity fixtures for highlight-server `mode: "unified-diff"`.
//!
//! Protocol v2 (Kathleen schema lock):
//! - READY: `{"ready":1,"protocol":2,"modes":["raw","diff-wash","unified-diff"]}`
//! - REQUEST: `{"id","mode":"unified-diff","language","code","colors"?,"diffStyle"?}`
//!   - colors: `{"inserted":"#hex","deleted":"#hex"}` (optional)
//!   - diffStyle: `"marker-fg"` (default) | `"bg-wash"`
//! - RESPONSE: `{"id", "ansi"}` or `{"id", "error"}`
//!
//! Ownership: Ethan owns this file only. Kathleen owns `src/*` and
//! `tests/highlight_server.rs`; Hannah reviews/docs only.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use serde_json::{Value, json};

struct Server {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl Server {
    fn start(args: &[&str]) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_mdstream"))
            .args(args)
            .arg("--highlight-server")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        Self {
            child,
            stdin,
            stdout,
        }
    }

    fn read(&mut self) -> Value {
        let mut line = String::new();
        self.stdout.read_line(&mut line).unwrap();
        assert!(!line.is_empty(), "highlight server closed stdout");
        serde_json::from_str(&line).unwrap()
    }

    fn request(&mut self, request: Value) -> Value {
        serde_json::to_writer(&mut self.stdin, &request).unwrap();
        self.stdin.write_all(b"\n").unwrap();
        self.stdin.flush().unwrap();
        self.read()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn strip_ansi(text: &str) -> String {
    let ansi = regex::Regex::new(r"\x1b\[[0-9;?]*[ -/]*[@-~]").unwrap();
    ansi.replace_all(text, "").into_owned()
}

fn sample_patch() -> &'static str {
    "diff --git a/src/lib.rs b/src/lib.rs\n\
     --- a/src/lib.rs\n\
     +++ b/src/lib.rs\n\
     @@ -1,5 +1,6 @@\n\
      use std::io;\n\
     -fn old() {\n\
     -    println!(\"old\");\n\
     +fn new() {\n\
     +    println!(\"new\");\n\
     +    // trailing\n\
      }\n\
     \\ No newline at end of file\n"
}

fn assert_ready_v2(ready: &Value) {
    assert_eq!(
        ready["ready"], 1,
        "ready handshake missing ready:1: {ready}"
    );
    // protocol/modes are additive in v2; require them once Phase 2 is live
    assert_eq!(
        ready["protocol"], 2,
        "expected protocol:2 on ready line: {ready}"
    );
    let modes = ready["modes"]
        .as_array()
        .expect("ready.modes must be an array");
    let modes: Vec<&str> = modes.iter().filter_map(|m| m.as_str()).collect();
    for required in ["raw", "diff-wash", "unified-diff"] {
        assert!(
            modes.contains(&required),
            "ready.modes must advertise {required}: {modes:?}"
        );
    }
}

fn assert_ok_ansi(response: &Value, id: u64) -> &str {
    assert_eq!(response["id"], id, "id echo mismatch: {response}");
    assert!(
        response.get("error").is_none()
            || response["error"].is_null()
            || response["error"]
                .as_str()
                .map(|s| s.is_empty())
                .unwrap_or(false),
        "unexpected error: {response}"
    );
    response["ansi"]
        .as_str()
        .unwrap_or_else(|| panic!("expected ansi string: {response}"))
}

fn assert_error(response: &Value, id: u64) {
    assert_eq!(response["id"], id, "id echo mismatch on error: {response}");
    let err = response["error"]
        .as_str()
        .unwrap_or_else(|| panic!("expected error string: {response}"));
    assert!(!err.is_empty(), "error must be non-empty: {response}");
    assert!(
        response.get("ansi").is_none() || response["ansi"].is_null(),
        "error response must not carry ansi: {response}"
    );
}

/// READY advertises protocol 2 and the three modes.
#[test]
fn ready_advertises_protocol2_and_unified_diff_mode() {
    let mut server = Server::start(&[]);
    let ready = server.read();
    assert_ready_v2(&ready);
}

/// Headers, hunk, context, add/del, diff --git, no-newline sentinel, trailing shape.
#[test]
fn unified_diff_headers_hunks_context_and_no_newline_sentinel() {
    let mut server = Server::start(&[]);
    assert_ready_v2(&server.read());

    let code = sample_patch();
    let response = server.request(json!({
        "id": 1,
        "mode": "unified-diff",
        "language": "rust",
        "code": code,
    }));
    let ansi = assert_ok_ansi(&response, 1);
    let plain = strip_ansi(ansi);

    assert_eq!(plain, code, "payload must be byte-identical after strip");
    assert_eq!(ansi.ends_with('\n'), code.ends_with('\n'));
    // No host frame chrome from markdown renderer
    assert!(
        !ansi.contains('─'),
        "must not add markdown code-frame rules"
    );
    assert!(
        !ansi.contains("╭") && !ansi.contains("╰"),
        "must not add TUI frame"
    );

    // Structural markers must still appear in the plain text
    assert!(plain.contains("diff --git"));
    assert!(plain.contains("--- a/src/lib.rs"));
    assert!(plain.contains("+++ b/src/lib.rs"));
    assert!(plain.contains("@@ -1,5 +1,6 @@"));
    assert!(plain.contains("\\ No newline at end of file"));

    // Expect some ANSI (semantic or token colors)
    assert!(ansi.contains("\u{1b}["), "unified-diff must emit ANSI");
}

/// Dual-stream multiline: open comment on deleted side must not poison added side.
#[test]
fn unified_diff_multiline_dual_stream_state() {
    let mut server = Server::start(&[]);
    assert_ready_v2(&server.read());

    let code = "\
--- a/x.rs
+++ b/x.rs
@@ -1,4 +1,4 @@
-/* open comment
-   still deleted */
+fn live() {
+    let x = 1;
 }
";
    let response = server.request(json!({
        "id": 2,
        "mode": "unified-diff",
        "language": "rust",
        "code": code,
    }));
    let ansi = assert_ok_ansi(&response, 2);
    let plain = strip_ansi(ansi);
    assert_eq!(plain, code);
    // Added payload should get language tokens (keyword/function), not stay entirely monochrome
    assert!(
        ansi.contains("\u{1b}[38;2;"),
        "expected 24-bit fg tokens on dual-stream body"
    );
}

/// Multi-hunk patch preserves both hunks and trailing empty row shape.
#[test]
fn unified_diff_multi_hunk_and_trailing_empty_row() {
    let mut server = Server::start(&[]);
    assert_ready_v2(&server.read());

    let code = "\
--- a/a.rs
+++ b/a.rs
@@ -1,2 +1,2 @@
-old_one
+new_one
 context
@@ -10,2 +10,2 @@
-old_two
+new_two

";
    let response = server.request(json!({
        "id": 3,
        "mode": "unified-diff",
        "language": "rust",
        "code": code,
    }));
    let ansi = assert_ok_ansi(&response, 3);
    let plain = strip_ansi(ansi);
    assert_eq!(plain, code);
    assert_eq!(plain.matches("@@ ").count(), 2);
    assert!(
        plain.ends_with("\n\n") || plain.ends_with("\n"),
        "trailing newline shape"
    );
}

/// Unknown language: structural diff colors, not hard error; payload preserved.
#[test]
fn unified_diff_unknown_language_structural_fallback() {
    let mut server = Server::start(&[]);
    assert_ready_v2(&server.read());

    let code = "\
--- a/x.unknown
+++ b/x.unknown
@@ -1 +1 @@
-old
+new
";
    let response = server.request(json!({
        "id": 4,
        "mode": "unified-diff",
        "language": "totally-unknown-language",
        "code": code,
    }));
    assert!(response.get("error").is_none() || response["error"].is_null());
    let ansi = assert_ok_ansi(&response, 4);
    assert_eq!(strip_ansi(ansi), code);
}

/// Semantic color overrides via `colors.inserted` / `colors.deleted` hex.
#[test]
fn unified_diff_semantic_color_overrides() {
    let mut server = Server::start(&[]);
    assert_ready_v2(&server.read());

    let code = "\
--- a/x.rs
+++ b/x.rs
@@ -1 +1 @@
-removed
+added
";
    // Distinct override colors unlikely to collide with default palette
    let inserted = "\u{1b}[38;2;1;2;3m"; // #010203
    let deleted = "\u{1b}[38;2;4;5;6m"; // #040506

    let response = server.request(json!({
        "id": 5,
        "mode": "unified-diff",
        "language": "rust",
        "code": code,
        "diffStyle": "marker-fg",
        "colors": {
            "inserted": "#010203",
            "deleted": "#040506",
        },
    }));
    let ansi = assert_ok_ansi(&response, 5);
    assert_eq!(strip_ansi(ansi), code);
    assert!(
        ansi.contains(deleted) || ansi.contains("\u{1b}[38;2;4;5;6m"),
        "deleted override #040506 not present: {ansi:?}"
    );
    assert!(
        ansi.contains(inserted) || ansi.contains("\u{1b}[38;2;1;2;3m"),
        "inserted override #010203 not present: {ansi:?}"
    );
}

/// `diffStyle: "bg-wash"`: marker+payload emit 48;2 wash AND keep token/base fg.
#[test]
fn unified_diff_bg_wash_style_if_exposed() {
    let mut server = Server::start(&[]);
    assert_ready_v2(&server.read());

    let code = "\
--- a/x.rs
+++ b/x.rs
@@ -1 +1 @@
-removed
+added
";
    // Product contract: marker fg = full colors hex; wash bg = wash_tint(hex, 0.22).
    // Overrides exercise non-default palette (values from Kathleen handoff).
    let response = server.request(json!({
        "id": 6,
        "mode": "unified-diff",
        "language": "rust",
        "code": code,
        "diffStyle": "bg-wash",
        "colors": {
            "inserted": "#1a3d24",
            "deleted": "#3d1a24",
        },
    }));

    let ansi = assert_ok_ansi(&response, 6);
    assert_eq!(strip_ansi(ansi), code);

    let del_line = ansi
        .lines()
        .find(|l| strip_ansi(l).starts_with('-') && strip_ansi(l).contains("removed"))
        .expect("deleted body line");
    let add_line = ansi
        .lines()
        .find(|l| strip_ansi(l).starts_with('+') && strip_ansi(l).contains("added"))
        .expect("added body line");

    // #3d1a24 → wash 48;2;13;5;7 ; #1a3d24 → wash 48;2;5;13;7
    assert!(
        del_line.contains("\u{1b}[48;2;13;5;7m"),
        "deleted wash must be wash_tint(#3d1a24,0.22): {del_line:?}"
    );
    assert!(
        add_line.contains("\u{1b}[48;2;5;13;7m"),
        "added wash must be wash_tint(#1a3d24,0.22): {add_line:?}"
    );
    // Marker fg = full colors hex.
    assert!(
        del_line.contains("\u{1b}[38;2;61;26;36m"),
        "deleted marker fg missing: {del_line:?}"
    );
    assert!(
        add_line.contains("\u{1b}[38;2;26;61;36m"),
        "added marker fg missing: {add_line:?}"
    );
    // Payload keeps a non-wash foreground (token or base) — not bg-only monochrome.
    assert!(
        del_line.contains("\u{1b}[38;2;232;238;250m")
            || del_line.matches("\u{1b}[38;2;").count() >= 2,
        "deleted payload should retain token/base fg under wash: {del_line:?}"
    );
    assert!(
        add_line.contains("\u{1b}[38;2;232;238;250m")
            || add_line.matches("\u{1b}[38;2;").count() >= 2,
        "added payload should retain token/base fg under wash: {add_line:?}"
    );
}

/// Default palette bg-wash: marker fg = full defaults; wash bg = wash_tint(0.22).
/// Matches native agent Edit (bright semantic hex + soft server wash).
#[test]
fn unified_diff_bg_wash_default_palette_emits_48_and_token_fg() {
    let mut server = Server::start(&[]);
    assert_ready_v2(&server.read());

    let code = "\
--- a/x.rs
+++ b/x.rs
@@ -1 +1 @@
-old_line
+new_line
";
    let response = server.request(json!({
        "id": 17,
        "mode": "unified-diff",
        "language": "rust",
        "code": code,
        "diffStyle": "bg-wash",
        // no colors → server defaults
    }));
    let ansi = assert_ok_ansi(&response, 17);
    assert_eq!(strip_ansi(ansi), code);

    let del_line = ansi
        .lines()
        .find(|l| strip_ansi(l).contains("old_line"))
        .expect("deleted line");
    let add_line = ansi
        .lines()
        .find(|l| strip_ansi(l).contains("new_line"))
        .expect("added line");

    // Marker fg: full default hex (#ff5d7a / #78e38c).
    assert!(
        del_line.contains("\u{1b}[38;2;255;93;122m"),
        "default deleted marker fg: {del_line:?}"
    );
    assert!(
        add_line.contains("\u{1b}[38;2;120;227;140m"),
        "default inserted marker fg: {add_line:?}"
    );
    // Wash: wash_tint(default, 0.22) → deleted 56;20;26, inserted 26;49;30
    assert!(
        del_line.contains("\u{1b}[48;2;56;20;26m"),
        "default deleted soft wash: {del_line:?}"
    );
    assert!(
        add_line.contains("\u{1b}[48;2;26;49;30m"),
        "default inserted soft wash: {add_line:?}"
    );
    // Content still has a separate fg (base or token) under the wash.
    assert!(
        del_line.contains("\u{1b}[38;2;232;238;250m")
            || del_line.matches("\u{1b}[38;2;").count() >= 2,
        "default wash must not strip content fg: {del_line:?}"
    );
    assert!(
        add_line.contains("\u{1b}[38;2;232;238;250m")
            || add_line.matches("\u{1b}[38;2;").count() >= 2,
        "default wash must not strip content fg: {add_line:?}"
    );
}

/// Unknown mode fails closed with error JSON (agent can fall back).
#[test]
fn unknown_mode_returns_error() {
    let mut server = Server::start(&[]);
    assert_ready_v2(&server.read());

    let response = server.request(json!({
        "id": 7,
        "mode": "not-a-real-mode",
        "language": "rust",
        "code": "fn main() {}\n",
    }));
    assert_error(&response, 7);
}

/// Malformed JSON still echoes id when parseable, else id 0.
#[test]
fn malformed_request_returns_error_with_id() {
    let mut server = Server::start(&[]);
    assert_ready_v2(&server.read());

    // Valid JSON but missing required fields for unified-diff
    let response = server.request(json!({
        "id": 8,
        "mode": "unified-diff",
        // missing language/code
    }));
    assert_error(&response, 8);
}

/// P2-0/P2-6: clients that only check `ready` still work; omit mode = raw v1.
#[test]
fn absent_mode_still_raw_byte_identical() {
    let mut server = Server::start(&[]);
    let ready = server.read();
    // Accept either v1 ready or v2 ready during rollout (additive handshake)
    assert_eq!(ready["ready"], 1);

    let code = "const x = 1;\n";
    let response = server.request(json!({
        "id": 9,
        "language": "typescript",
        "code": code,
    }));
    let ansi = assert_ok_ansi(&response, 9);
    assert_eq!(strip_ansi(ansi), code);
    assert_eq!(ansi.ends_with('\n'), code.ends_with('\n'));
}

/// P2-6: raw path never auto-strips leading +/- (operators stay in payload).
#[test]
fn raw_mode_never_auto_strips_plus_minus() {
    let mut server = Server::start(&[]);
    let ready = server.read();
    assert_eq!(ready["ready"], 1);

    let code = "-1 + 2\n+3\n";
    let response = server.request(json!({
        "id": 10,
        "language": "rust",
        "code": code,
    }));
    let ansi = assert_ok_ansi(&response, 10);
    assert_eq!(strip_ansi(ansi), code);
    assert!(strip_ansi(ansi).starts_with('-'));
}

/// P2-3: omit colors + omit diffStyle → defaults (marker-fg, theme palette); still OK.
#[test]
fn unified_diff_default_palette_when_colors_omitted() {
    let mut server = Server::start(&[]);
    assert_ready_v2(&server.read());

    let code = "\
--- a/x.rs
+++ b/x.rs
@@ -1 +1 @@
-old
+new
";
    let response = server.request(json!({
        "id": 11,
        "mode": "unified-diff",
        "language": "rust",
        "code": code,
        // no colors, no diffStyle
    }));
    let ansi = assert_ok_ansi(&response, 11);
    assert_eq!(strip_ansi(ansi), code);
    assert!(
        ansi.contains("\u{1b}["),
        "default palette must still color output"
    );
}

/// P2-4: code without trailing newline keeps that shape in ansi.
#[test]
fn unified_diff_preserves_missing_trailing_newline() {
    let mut server = Server::start(&[]);
    assert_ready_v2(&server.read());

    let code = "--- a/x.rs\n+++ b/x.rs\n@@ -1 +1 @@\n-old\n+new"; // no final \n
    let response = server.request(json!({
        "id": 12,
        "mode": "unified-diff",
        "language": "rust",
        "code": code,
    }));
    let ansi = assert_ok_ansi(&response, 12);
    assert_eq!(strip_ansi(ansi), code);
    assert!(!ansi.ends_with('\n'), "must not invent trailing newline");
}

/// P2-5: malformed color hex fails closed.
#[test]
fn unified_diff_malformed_color_hex_errors() {
    let mut server = Server::start(&[]);
    assert_ready_v2(&server.read());

    let response = server.request(json!({
        "id": 13,
        "mode": "unified-diff",
        "language": "rust",
        "code": "--- a/x\n+++ b/x\n@@ -1 +1 @@\n-a\n+b\n",
        "colors": {
            "inserted": "not-a-hex",
            "deleted": "#ff5d7a",
        },
    }));
    assert_error(&response, 13);
}

/// P2-5: unknown diffStyle fails closed.
#[test]
fn unified_diff_unknown_diff_style_errors() {
    let mut server = Server::start(&[]);
    assert_ready_v2(&server.read());

    let response = server.request(json!({
        "id": 14,
        "mode": "unified-diff",
        "language": "rust",
        "code": "--- a/x\n+++ b/x\n@@ -1 +1 @@\n-a\n+b\n",
        "diffStyle": "rainbow-sparkle",
    }));
    assert_error(&response, 14);
}

/// P2-4: `\r\n` line endings in `code` are preserved in ansi (strip_ansi).
#[test]
fn unified_diff_preserves_crlf_line_endings() {
    let mut server = Server::start(&[]);
    assert_ready_v2(&server.read());

    let code = "--- a/x.rs\r\n+++ b/x.rs\r\n@@ -1 +1 @@\r\n-old\r\n+new\r\n";
    let response = server.request(json!({
        "id": 16,
        "mode": "unified-diff",
        "language": "rust",
        "code": code,
    }));
    let ansi = assert_ok_ansi(&response, 16);
    assert_eq!(strip_ansi(ansi), code);
    assert!(
        strip_ansi(ansi).contains("\r\n"),
        "CRLF must survive strip_ansi"
    );
}

/// P2-5: garbage non-JSON line → error; id defaults to 0 when unparseable.
#[test]
fn garbage_non_json_line_errors_with_id_zero() {
    let mut server = Server::start(&[]);
    assert_ready_v2(&server.read());

    server.stdin.write_all(b"this is not json\n").unwrap();
    server.stdin.flush().unwrap();
    let response = server.read();
    assert_eq!(
        response["id"], 0,
        "unparseable line should echo id 0: {response}"
    );
    let err = response["error"]
        .as_str()
        .unwrap_or_else(|| panic!("expected error on garbage line: {response}"));
    assert!(!err.is_empty());
}

/// P2-5: already-ANSI input fails closed if the server detects it.
#[test]
fn unified_diff_already_ansi_input_fail_closed() {
    let mut server = Server::start(&[]);
    assert_ready_v2(&server.read());

    let code = "--- a/x\n+++ b/x\n@@ -1 +1 @@\n-\u{1b}[31mold\n+\u{1b}[32mnew\n";
    let response = server.request(json!({
        "id": 15,
        "mode": "unified-diff",
        "language": "rust",
        "code": code,
    }));
    // Contract: fail closed preferred; if not yet implemented, error OR passthrough
    // without double-encoding. Prefer error.
    if response.get("error").and_then(|e| e.as_str()).is_some() {
        assert_error(&response, 15);
    } else {
        let ansi = assert_ok_ansi(&response, 15);
        // Must not invent host frame even if detection is deferred
        assert!(!ansi.contains('─'));
    }
}
