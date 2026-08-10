//! Unified-diff composition for mixed language+diff highlighting.
//!
//! Given a unified diff body (e.g. the contents of a ```rust fence whose lines
//! carry `-`/`+` markers), this module highlights the added and deleted sides
//! as two independent lexical streams while preserving the diff structure.
//! Deleted lines (`-`) run through a "from" stream, added lines (`+`) through a
//! "to" stream; each keeps its own syntect lexical state so an open comment on
//! one side cannot poison the other.
//!
//! Two output styles:
//! - `marker-fg` (default): the `-`/`+` marker is painted in the delete/insert
//!   color and the content gets normal language token colors.
//! - `bg-wash`: the marker keeps its color and the whole line gets a background
//!   wash, re-asserted after every `\x1b[0m` (the Phase 1 primitive).
//!
//! The payload is always byte-identical after ANSI stripping: no host frame is
//! synthesized, diff structural markers (`diff --git`, `---`, `+++`, `@@`,
//! `\ No newline`) are preserved verbatim, and line endings (`\n` vs `\r\n`)
//! plus trailing-newline shape are preserved.

use anyhow::{Result, bail};
use serde::Deserialize;

use crate::highlight::{LineHighlighter, bg_escape, fg_escape, reassert_wash};
use crate::theme::CodeTheme;

const RESET: &str = "\x1b[0m";

/// Default marker colors: inserted green, deleted red.
pub const DEFAULT_INSERTED: &str = "#78e38c";
pub const DEFAULT_DELETED: &str = "#ff5d7a";

/// How a diff line's marker and content are styled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffStyle {
    /// Colored `-`/`+` marker, language token colors on content.
    MarkerFg,
    /// Colored marker plus a whole-line background wash.
    BgWash,
}

impl DiffStyle {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "marker-fg" => Some(Self::MarkerFg),
            "bg-wash" => Some(Self::BgWash),
            _ => None,
        }
    }
}

/// Overridable marker colors.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct DiffColors {
    #[serde(default)]
    pub inserted: Option<String>,
    #[serde(default)]
    pub deleted: Option<String>,
}

impl DiffColors {
    pub fn resolved(&self) -> (String, String) {
        (
            self.inserted
                .clone()
                .unwrap_or_else(|| DEFAULT_INSERTED.to_owned()),
            self.deleted
                .clone()
                .unwrap_or_else(|| DEFAULT_DELETED.to_owned()),
        )
    }
}

/// One logical line of the diff body, classified.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LineKind {
    /// Added line (`+`): content after the marker.
    Added,
    /// Deleted line (`-`): content after the marker.
    Deleted,
    /// Context line (` `) or any non-add/del line: rendered as-is.
    Context,
}

/// Composes a unified diff body into a highlighted ANSI string.
///
/// `language` is the fence language whose tokens color the body (added and
/// deleted sides each get an independent stream). `code` is the diff body
/// including any `diff --git` / `---` / `+++` / `@@` structural lines; those
/// are preserved verbatim (uncolored or plain) while `-`/`+` body lines get
/// dual-stream highlighting. Returns the ANSI string with the exact line
/// endings and trailing-newline shape of `code`.
pub fn highlight_unified_diff(
    language: &str,
    code: &str,
    style: DiffStyle,
    colors: &DiffColors,
    code_theme: CodeTheme,
    show_background: bool,
) -> Result<String> {
    // Fail closed on already-ANSI input: re-encoding escape sequences would
    // corrupt the payload, and the caller cannot round-trip them.
    if code.contains('\x1b') {
        bail!("input contains ANSI escape sequences; refusing to re-encode");
    }

    let mut from = LineHighlighter::new(language, code_theme, show_background);
    let mut to = LineHighlighter::new(language, code_theme, show_background);

    let (inserted_hex, deleted_hex) = colors.resolved();
    let inserted_fg = fg_escape(&inserted_hex)
        .ok_or_else(|| anyhow::anyhow!("invalid inserted color: {}", inserted_hex))?;
    let deleted_fg = fg_escape(&deleted_hex)
        .ok_or_else(|| anyhow::anyhow!("invalid deleted color: {}", deleted_hex))?;
    // The background wash is derived: blend the semantic color ~22% toward
    // black so the full-payload wash is obvious but not loud, while the marker
    // keeps its bright color and syntax foreground stays readable. The agent
    // sends bright semantic hex; the server owns wash strength.
    let inserted_bg = bg_escape(&wash_tint(&inserted_hex, 0.22)).unwrap();
    let deleted_bg = bg_escape(&wash_tint(&deleted_hex, 0.22)).unwrap();

    let mut output = String::with_capacity(code.len() * 2);
    let mut pending_no_newline: Option<(String, String)> = None;

    for line in code.split_inclusive('\n') {
        let (content, ending) = split_ending(line);

        // Handle `\ No newline at end of file`: the previous body line should
        // not get the trailing newline appended. Render it now.
        if content.starts_with("\\ No newline at end of file") {
            if let Some((prev_content, _)) = pending_no_newline.take() {
                output.push_str(&prev_content);
                // The no-newline marker line itself carries the ending.
                output.push_str(ending);
            } else {
                output.push_str(content);
                output.push_str(ending);
            }
            continue;
        }

        // A normal body line: flush any pending no-newline previous line.
        if let Some((prev_content, _)) = pending_no_newline.take() {
            output.push_str(&prev_content);
        }

        let (kind, body) = classify(content);

        // Structural or context lines render verbatim (no marker color).
        let rendered = match kind {
            LineKind::Context => content.to_owned(),
            LineKind::Added | LineKind::Deleted => {
                let is_added = kind == LineKind::Added;
                let marker = if is_added { "+" } else { "-" };
                let marker_fg = if is_added { &inserted_fg } else { &deleted_fg };
                let stream = if is_added { &mut to } else { &mut from };
                let escaped = stream.highlight_line(body)?;
                match style {
                    DiffStyle::MarkerFg => {
                        // Trailing RESET: escaped ends on a fg escape; without
                        // it the color would leak past the newline.
                        format!("{marker_fg}{marker}{RESET}{escaped}{RESET}")
                    }
                    DiffStyle::BgWash => {
                        let wash = if is_added { &inserted_bg } else { &deleted_bg };
                        let escaped = reassert_wash(&escaped, &Some(wash.clone()));
                        // Marker painted on the wash, content on wash + token fg.
                        // Trailing RESET clears both so the wash never leaks
                        // past the newline into the next host row.
                        format!("{marker_fg}{marker}{RESET}{escaped}{RESET}")
                    }
                }
            }
        };

        if ending.is_empty() && !code.ends_with('\n') && kind != LineKind::Context {
            // Last line, no trailing newline: hold it in case a `\ No newline`
            // marker follows; otherwise emit with no ending.
            pending_no_newline = Some((rendered, String::new()));
        } else {
            output.push_str(&rendered);
            output.push_str(ending);
        }
    }

    if let Some((prev, _)) = pending_no_newline.take() {
        output.push_str(&prev);
    }

    Ok(output)
}

/// Blends a `#rrggbb` hex color toward black by `factor` (0..=1), producing a
/// soft wash background derived from the marker color.
fn wash_tint(hex: &str, factor: f64) -> String {
    let hex = hex.strip_prefix('#').unwrap_or(hex);
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0) as f64;
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0) as f64;
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0) as f64;
    let f = factor.clamp(0.0, 1.0);
    let tint = |c: f64| (c * f) as u8;
    format!("#{:02x}{:02x}{:02x}", tint(r), tint(g), tint(b))
}

/// Splits a split_inclusive line into (content, ending).
fn split_ending(line: &str) -> (&str, &str) {
    if let Some(content) = line.strip_suffix("\r\n") {
        (content, "\r\n")
    } else if let Some(content) = line.strip_suffix('\n') {
        (content, "\n")
    } else {
        (line, "")
    }
}

/// Classifies a body line and returns (kind, content-after-marker).
fn classify(line: &str) -> (LineKind, &str) {
    if let Some(rest) = line.strip_prefix('+') {
        // `+++ b/file` and `++` edge cases: a `+` line whose content starts
        // with another `+` is a structural file header, not an addition.
        if rest.starts_with('+') {
            (LineKind::Context, line)
        } else {
            (LineKind::Added, rest)
        }
    } else if let Some(rest) = line.strip_prefix('-') {
        // `--- a/file` and `--` edge cases: a `-` line whose content starts
        // with another `-` is a structural file header, not a deletion.
        if rest.starts_with('-') {
            (LineKind::Context, line)
        } else {
            (LineKind::Deleted, rest)
        }
    } else {
        (LineKind::Context, line)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strip_ansi(text: &str) -> String {
        let ansi = regex::Regex::new(r"\x1b\[[0-9;?]*[ -/]*[@-~]").unwrap();
        ansi.replace_all(text, "").into_owned()
    }

    fn default_colors() -> DiffColors {
        DiffColors::default()
    }

    #[test]
    fn marker_fg_preserves_payload_and_structure() {
        let code = "diff --git a/src/lib.rs b/src/lib.rs\n\
             --- a/src/lib.rs\n\
             +++ b/src/lib.rs\n\
             @@ -1,5 +1,6 @@\n\
              use std::io;\n\
             -fn old() {\n\
             +fn new() {\n\
             }\n";
        let ansi = highlight_unified_diff(
            "rust",
            code,
            DiffStyle::MarkerFg,
            &default_colors(),
            CodeTheme::Mdstream,
            false,
        )
        .unwrap();
        assert_eq!(strip_ansi(&ansi), code);
        assert!(
            ansi.contains("\x1b[38;2;255;93;122m-"),
            "deleted marker color"
        );
        assert!(
            ansi.contains("\x1b[38;2;120;227;140m+"),
            "inserted marker color"
        );
        // Added content gets token colors (fn keyword).
        assert!(ansi.contains("\x1b[38;2;"), "token colors present");
    }

    #[test]
    fn dual_stream_isolates_lexical_state() {
        let code = "--- a/x.rs\n+++ b/x.rs\n@@ -1,4 +1,4 @@\n\
            -/* open comment\n\
            -   still deleted */\n\
            +fn live() {\n\
            +    let x = 1;\n\
             }\n";
        let ansi = highlight_unified_diff(
            "rust",
            code,
            DiffStyle::MarkerFg,
            &default_colors(),
            CodeTheme::Mdstream,
            false,
        )
        .unwrap();
        assert_eq!(strip_ansi(&ansi), code);
        // Added side must get real token colors (function name), proving the
        // deleted-side comment did not poison the added stream.
        assert!(
            ansi.contains("\x1b[38;2;123;167;255m"),
            "added stream should have function-color tokens: {:?}",
            ansi
        );
    }

    #[test]
    fn bg_wash_uses_background_sgr() {
        let code = "--- a/x.rs\n+++ b/x.rs\n@@ -1 +1 @@\n-old\n+new\n";
        let ansi = highlight_unified_diff(
            "rust",
            code,
            DiffStyle::BgWash,
            &default_colors(),
            CodeTheme::Mdstream,
            false,
        )
        .unwrap();
        assert_eq!(strip_ansi(&ansi), code);
        assert!(
            ansi.contains("\x1b[48;2;"),
            "bg-wash must emit background SGR: {:?}",
            ansi
        );
    }

    #[test]
    fn color_overrides_apply() {
        let code = "--- a/x.rs\n+++ b/x.rs\n@@ -1 +1 @@\n-old\n+new\n";
        let colors = DiffColors {
            inserted: Some("#010203".to_owned()),
            deleted: Some("#040506".to_owned()),
        };
        let ansi = highlight_unified_diff(
            "rust",
            code,
            DiffStyle::MarkerFg,
            &colors,
            CodeTheme::Mdstream,
            false,
        )
        .unwrap();
        assert_eq!(strip_ansi(&ansi), code);
        assert!(ansi.contains("\x1b[38;2;4;5;6m-"), "deleted override");
        assert!(ansi.contains("\x1b[38;2;1;2;3m+"), "inserted override");
    }

    #[test]
    fn unknown_language_falls_back_to_plaintext() {
        let code = "--- a/x\n+++ b/x\n@@ -1 +1 @@\n-old\n+new\n";
        let ansi = highlight_unified_diff(
            "totally-unknown",
            code,
            DiffStyle::MarkerFg,
            &default_colors(),
            CodeTheme::Mdstream,
            false,
        )
        .unwrap();
        assert_eq!(strip_ansi(&ansi), code);
        // Still has marker colors even with no language tokens.
        assert!(ansi.contains("\x1b[38;2;"));
    }

    #[test]
    fn crlf_and_trailing_shape_preserved() {
        for code in [
            "--- a/x.rs\n+++ b/x.rs\n@@ -1 +1 @@\n-old\n+new\n",
            "--- a/x.rs\n+++ b/x.rs\n@@ -1 +1 @@\n-old\n+new", // no trailing \n
            "--- a/x.rs\r\n+++ b/x.rs\r\n@@ -1 +1 @@\r\n-old\r\n+new\r\n",
        ] {
            let ansi = highlight_unified_diff(
                "rust",
                code,
                DiffStyle::MarkerFg,
                &default_colors(),
                CodeTheme::Mdstream,
                false,
            )
            .unwrap();
            assert_eq!(strip_ansi(&ansi), code, "code: {:?}", code);
            assert_eq!(ansi.ends_with('\n'), code.ends_with('\n'));
        }
    }

    #[test]
    fn no_newline_sentinel_handled() {
        let code = "--- a/x\n+++ b/x\n@@ -1 +1 @@\n-a\n\\ No newline at end of file\n+b\n";
        let ansi = highlight_unified_diff(
            "rust",
            code,
            DiffStyle::MarkerFg,
            &default_colors(),
            CodeTheme::Mdstream,
            false,
        )
        .unwrap();
        assert_eq!(strip_ansi(&ansi), code);
    }

    #[test]
    fn ansi_input_fails_closed() {
        let code = "--- a/x\n+++ b/x\n@@ -1 +1 @@\n-\u{1b}[31mold\n+new\n";
        assert!(
            highlight_unified_diff(
                "rust",
                code,
                DiffStyle::MarkerFg,
                &default_colors(),
                CodeTheme::Mdstream,
                false,
            )
            .is_err()
        );
    }

    #[test]
    fn multi_hunk_preserved() {
        let code = "--- a/a.rs\n+++ b/a.rs\n@@ -1,2 +1,2 @@\n-old_one\n+new_one\n context\n@@ -10,2 +10,2 @@\n-old_two\n+new_two\n\n";
        let ansi = highlight_unified_diff(
            "rust",
            code,
            DiffStyle::MarkerFg,
            &default_colors(),
            CodeTheme::Mdstream,
            false,
        )
        .unwrap();
        assert_eq!(strip_ansi(&ansi), code);
        assert_eq!(strip_ansi(&ansi).matches("@@ ").count(), 2);
    }

    /// Every non-empty output line must end with an ANSI reset before its line
    /// ending (and at EOF), so no SGR leaks past a newline into the host's
    /// next row. Covers both styles and the no-trailing-newline case.
    #[test]
    fn every_line_ends_balanced_reset() {
        for style in [DiffStyle::MarkerFg, DiffStyle::BgWash] {
            for code in [
                "--- a/x.rs\n+++ b/x.rs\n@@ -1 +1 @@\n-old\n+new\n",
                "--- a/x.rs\n+++ b/x.rs\n@@ -1 +1 @@\n-old\n+new", // no trailing \n
            ] {
                let ansi = highlight_unified_diff(
                    "rust",
                    code,
                    style,
                    &default_colors(),
                    CodeTheme::Mdstream,
                    false,
                )
                .unwrap();
                let has_trailing = code.ends_with('\n');
                let lines: Vec<&str> = ansi.split('\n').collect();
                let body: Vec<&str> = if has_trailing {
                    &lines[..lines.len() - 1]
                } else {
                    &lines[..]
                }
                .to_vec();
                for (i, l) in body.iter().enumerate() {
                    // Plain (structural/context) lines carry no ANSI and need
                    // no reset. Styled lines must end reset before the newline.
                    if !l.is_empty() && l.contains("\x1b[") {
                        assert!(
                            l.ends_with("\x1b[0m"),
                            "style={style:?} line {i} must end reset: {:?}",
                            l
                        );
                    }
                }
                assert!(
                    ansi.ends_with('\n') || ansi.ends_with("\x1b[0m"),
                    "style={style:?} output must end reset at EOF: {:?}",
                    ansi
                );
            }
        }
    }
}
