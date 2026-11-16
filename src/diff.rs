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
    highlight_unified_diff_with_budget(
        language,
        code,
        style,
        colors,
        code_theme,
        show_background,
        CONTEXT_LINE_BUDGET,
    )
}

/// [`highlight_unified_diff`] with an explicit context-line budget. A budget
/// of 0 disables context continuity entirely, which tests use as the
/// pre-continuity reference rendering.
fn highlight_unified_diff_with_budget(
    language: &str,
    code: &str,
    style: DiffStyle,
    colors: &DiffColors,
    code_theme: CodeTheme,
    show_background: bool,
    context_line_budget: usize,
) -> Result<String> {
    // Fail closed on already-ANSI input: re-encoding escape sequences would
    // corrupt the payload, and the caller cannot round-trip them.
    if code.contains('\x1b') {
        bail!("input contains ANSI escape sequences; refusing to re-encode");
    }

    let mut from = LineHighlighter::new(language, code_theme, show_background);
    let mut to = LineHighlighter::new(language, code_theme, show_background);
    // Context lines are rendered verbatim, but they are still source that the
    // old and new file both contain. They are lexed (output discarded) so
    // lexical state carries across them: a `/*` opened on a context line must
    // keep the next `+` line inside the comment, and a `<?php` on a context
    // line must put the PHP grammar into PHP mode.
    //
    // Cost control. Context usually dominates a patch, so this pass must stay
    // close to the old cost:
    // - A context line is fed to a stream only when that stream still emits a
    //   changed line later in the same file. Trailing context and the context
    //   of an add-only or delete-only file never reach a lexer.
    // - While both streams hold identical state (the common case: no changed
    //   line since the last sync, or changes that ended in the same state),
    //   the line is lexed once and the state is copied.
    //
    // Structure comes from `line_roles`, a hunk-aware pass. Preamble and file
    // headers never reach a lexer, so a quote in a file name cannot poison
    // the first hunk. Each file starts fresh state.
    let lines: Vec<&str> = code.split_inclusive('\n').collect();
    let roles = line_roles(&lines);
    let kinds: Vec<LineKind> = lines
        .iter()
        .map(|line| classify(split_ending(line).0).0)
        .collect();
    let (from_emits_later, to_emits_later) = streams_emit_later(&roles, &kinds);
    // Hard bound on the extra work, decided UP FRONT and per file, all or
    // nothing. Stopping part-way is wrong, not just less informed: a `/*` fed
    // before the cut whose `*/` falls after it would leave every later changed
    // line styled as comment. A file is therefore either fed in full or not at
    // all, and an unfed file renders as it did before context continuity
    // existed. Files are admitted greedily in patch order, so one huge file
    // does not cost the small files around it their continuity. Each file
    // starts fresh state, so the files are independent.
    let fed: Vec<bool> = (0..lines.len())
        .map(|index| {
            let content = split_ending(lines[index]).0;
            roles[index] == Role::Body
                && (content.is_empty() || content.starts_with(' '))
                && (from_emits_later[index] || to_emits_later[index])
        })
        .collect();
    let file_fed = admit_files(&roles, &fed, context_line_budget);

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

    // Index into `file_fed`: 0 is anything before the first file header.
    let mut file = 0;
    for (index, line) in lines.iter().enumerate() {
        let (content, ending) = split_ending(line);

        if roles[index] == Role::FileStart {
            file += 1;
        }
        match roles[index] {
            // A new file in a multi-file patch starts fresh lexical state, so
            // one file's unterminated comment cannot bleed into the next file.
            Role::FileStart => {
                from = LineHighlighter::new(language, code_theme, show_background);
                to = LineHighlighter::new(language, code_theme, show_background);
            }
            // Some tools strip the single space from blank context lines, so
            // an empty line inside a hunk is blank context.
            _ if fed[index] && file_fed[file] => {
                let body = content.strip_prefix(' ').unwrap_or(content);
                match (from_emits_later[index], to_emits_later[index]) {
                    (true, true) if from.same_state(&to) => {
                        to.advance(body)?;
                        from.copy_state_from(&to);
                    }
                    (true, true) => {
                        from.advance(body)?;
                        to.advance(body)?;
                    }
                    (true, false) => from.advance(body)?,
                    (false, true) => to.advance(body)?,
                    // `fed` requires a live stream. Fail safe on the request
                    // path: a panic would kill the highlight server.
                    (false, false) => debug_assert!(false, "`fed` requires a live stream"),
                }
            }
            _ => {}
        }

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

/// Maximum context lines fed to the lexers per request. A fed line costs one
/// lex while both streams share state (the common case) and two otherwise, at
/// a measured ~0.15 ms per lex for the heaviest bundled grammars
/// (JavaScript/TypeScript). The bound is therefore ~150 ms typical and ~300 ms
/// worst case over the pre-continuity cost. Real edit hunks carry a few dozen
/// context lines, so this only binds on very large, dense patches.
const CONTEXT_LINE_BUDGET: usize = 1_000;

/// Decides, per file section, whether its context is fed: greedily in patch
/// order, a file is admitted when ALL of its fed lines fit the remaining
/// budget. Index 0 is the section before the first `FileStart`.
fn admit_files(roles: &[Role], fed: &[bool], budget: usize) -> Vec<bool> {
    let mut counts = vec![0usize];
    for (role, fed) in roles.iter().zip(fed) {
        if *role == Role::FileStart {
            counts.push(0);
        }
        if *fed {
            *counts.last_mut().unwrap() += 1;
        }
    }
    let mut remaining = budget;
    counts
        .into_iter()
        .map(|count| {
            let admit = count <= remaining;
            if admit {
                remaining -= count;
            }
            admit
        })
        .collect()
}

/// Structural role of one patch line, for lexer feeding only. Rendering still
/// uses [`classify`], so output colors of headers and markers are unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    /// Opens a new file section: lexical state resets here.
    FileStart,
    /// Preamble, `+++` header, or a hunk header. Never lexed.
    Structure,
    /// A line inside a hunk.
    Body,
}

/// Assigns a [`Role`] to every line with a hunk-aware pass.
///
/// When a hunk header carries line counts (`@@ -a,b +c,d @@`), the hunk ends
/// exactly when both counts are consumed. A deleted `-- x` line (`--- x` in
/// the patch) or an added `++ y` line (`+++ y`) inside a hunk is then body,
/// never a file header, so it cannot reset state mid-file.
///
/// A bare `@@` with no counts (hand-written patches) leaves the hunk
/// open-ended. There, a `---` / `+++` / `@@` triple is taken as a new file.
/// A false reset only drops back to fresh state, which is the pre-continuity
/// behavior. A missed reset would carry one file's comment into the next.
fn line_roles(lines: &[&str]) -> Vec<Role> {
    let content = |i: usize| lines.get(i).map(|line| split_ending(line).0);
    let header_triple = |i: usize| {
        content(i).is_some_and(|l| l.starts_with("--- "))
            && content(i + 1).is_some_and(|l| l.starts_with("+++ "))
            && content(i + 2).is_some_and(|l| l.starts_with("@@"))
    };

    // `None`: outside a hunk. `Some(None)`: open-ended hunk. `Some(Some(..))`:
    // remaining (old, new) line counts.
    let mut hunk: Option<Option<(u64, u64)>> = None;
    let mut roles = Vec::with_capacity(lines.len());
    for index in 0..lines.len() {
        let line = content(index).unwrap_or("");
        if let Some(Some((0, 0))) = hunk {
            hunk = None;
        }
        let role = if line.starts_with("diff ") {
            // Body lines start with ` `, `+`, `-` or `\`, so a `diff ` line is
            // always preamble.
            hunk = None;
            Role::FileStart
        } else if line.starts_with("@@") && !matches!(hunk, Some(Some(_))) {
            hunk = Some(hunk_counts(line));
            Role::Structure
        } else if hunk.is_none() {
            if line.starts_with("--- ") {
                Role::FileStart
            } else {
                Role::Structure
            }
        } else if hunk == Some(None) && header_triple(index) {
            hunk = None;
            Role::FileStart
        } else {
            if let Some(Some((old, new))) = hunk.as_mut() {
                match line.as_bytes().first() {
                    Some(b'-') => *old = old.saturating_sub(1),
                    Some(b'+') => *new = new.saturating_sub(1),
                    Some(b'\\') => {}
                    _ => {
                        *old = old.saturating_sub(1);
                        *new = new.saturating_sub(1);
                    }
                }
            }
            Role::Body
        };
        roles.push(role);
    }
    roles
}

/// Parses `@@ -a[,b] +c[,d] @@` into the (old, new) line counts. An omitted
/// count means 1. Returns `None` when the header has no parseable ranges.
fn hunk_counts(header: &str) -> Option<(u64, u64)> {
    let mut parts = header.strip_prefix("@@")?.split_whitespace();
    let count = |range: &str| -> Option<u64> {
        match range.split_once(',') {
            Some((start, len)) => {
                start.parse::<u64>().ok()?;
                len.parse().ok()
            }
            None => range.parse::<u64>().ok().map(|_| 1),
        }
    };
    let old = count(parts.next()?.strip_prefix('-')?)?;
    let new = count(parts.next()?.strip_prefix('+')?)?;
    Some((old, new))
}

/// For each line, whether the from (deleted) and to (added) streams still
/// emit a changed line later in the same file. Context after a stream's last
/// emitted line can never affect output, so it is not lexed for that stream.
fn streams_emit_later(roles: &[Role], kinds: &[LineKind]) -> (Vec<bool>, Vec<bool>) {
    let mut from = vec![false; roles.len()];
    let mut to = vec![false; roles.len()];
    let (mut from_later, mut to_later) = (false, false);
    for index in (0..roles.len()).rev() {
        from[index] = from_later;
        to[index] = to_later;
        if roles[index] == Role::FileStart {
            from_later = false;
            to_later = false;
        }
        match kinds[index] {
            LineKind::Deleted => from_later = true,
            LineKind::Added => to_later = true,
            LineKind::Context => {}
        }
    }
    (from, to)
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

    /// Distinct truecolor foregrounds on the first output line that starts
    /// with the colored `+` marker.
    fn added_row_colors(ansi: &str) -> std::collections::BTreeSet<String> {
        let fg = regex::Regex::new(r"\x1b\[38;2;[0-9;]+m").unwrap();
        let row = ansi
            .split('\n')
            .find(|row| strip_ansi(row).starts_with('+') && !row.starts_with('+'))
            .expect("an added row");
        fg.find_iter(row).map(|m| m.as_str().to_owned()).collect()
    }

    #[test]
    fn context_lines_carry_lexical_state_into_changed_lines() {
        // Regression: context lines were rendered verbatim and never lexed, so
        // a block comment opened on a context line did not reach the `+` line
        // inside it, which was then colored as live code.
        let code = "--- a/x.ts\n+++ b/x.ts\n@@ -1,2 +1,3 @@\n /* opened in context\n+const notCode = 1 */\n }\n";
        let ansi = highlight_unified_diff(
            "typescript",
            code,
            DiffStyle::MarkerFg,
            &default_colors(),
            CodeTheme::Mdstream,
            false,
        )
        .unwrap();
        assert_eq!(strip_ansi(&ansi), code);
        let keyword_blue = "\x1b[38;2;123;167;255m";
        let number_amber = "\x1b[38;2;255;183;94m";
        let colors = added_row_colors(&ansi);
        assert!(
            !colors.contains(keyword_blue) && !colors.contains(number_amber),
            "added line inside a context-opened comment must not get code colors: {:?}",
            ansi
        );
    }

    #[test]
    fn php_open_tag_on_context_line_enters_php_mode() {
        let code =
            "--- a/x.php\n+++ b/x.php\n@@ -1,2 +1,3 @@\n <?php\n+$x = \"s\" . 1;\n echo $x;\n";
        let ansi = highlight_unified_diff(
            "php",
            code,
            DiffStyle::MarkerFg,
            &default_colors(),
            CodeTheme::Mdstream,
            false,
        )
        .unwrap();
        assert_eq!(strip_ansi(&ansi), code);
        // Marker + base only was the flat bug (2). PHP tokens add string,
        // punctuation and number colors.
        assert!(
            added_row_colors(&ansi).len() >= 4,
            "added PHP line must be token-colored: {:?}",
            ansi
        );
    }

    #[test]
    fn file_headers_and_preamble_never_reach_the_lexer() {
        // A quote in a file name must not open a string that swallows the
        // first hunk. Compare against the same hunk with neutral headers.
        let hunk = "@@ -1 +1 @@\n-let a = 1;\n+let b = 2;\n";
        let quoted =
            format!("diff --git a/it's.rs b/it's.rs\n--- \"a/it's.rs\n+++ \"b/it's.rs\n{hunk}");
        let neutral = format!("--- a/x.rs\n+++ b/x.rs\n{hunk}");
        let render = |code: &str| {
            highlight_unified_diff(
                "rust",
                code,
                DiffStyle::MarkerFg,
                &default_colors(),
                CodeTheme::Mdstream,
                false,
            )
            .unwrap()
        };
        let tail = |ansi: String| {
            ansi.split('\n')
                .skip_while(|row| !row.starts_with("@@"))
                .collect::<Vec<_>>()
                .join("\n")
        };
        assert_eq!(tail(render(&quoted)), tail(render(&neutral)));
    }

    #[test]
    fn each_file_in_a_multi_file_patch_starts_fresh() {
        // An unterminated comment in the first file's context must not turn
        // the second file's added line into a comment.
        let code = "--- a/a.ts\n+++ b/a.ts\n@@ -1 +1,2 @@\n /* never closed\n+x\n\
                    --- a/b.ts\n+++ b/b.ts\n@@ -1 +1 @@\n-old\n+const live = 1;\n";
        let ansi = highlight_unified_diff(
            "typescript",
            code,
            DiffStyle::MarkerFg,
            &default_colors(),
            CodeTheme::Mdstream,
            false,
        )
        .unwrap();
        assert_eq!(strip_ansi(&ansi), code);
        let last_added = ansi
            .split('\n')
            .find(|row| strip_ansi(row).starts_with("+const"))
            .unwrap();
        assert!(
            last_added.contains("\x1b[38;2;123;167;255m"),
            "second file's added line must be live code: {:?}",
            last_added
        );
    }

    #[test]
    fn body_lines_that_mimic_file_headers_do_not_reset_state() {
        // A deleted `-- old` is `--- old` in the patch and an added `++ new`
        // is `+++ new`. Followed by the next `@@`, they look like a file
        // header triple. Hunk counts prove they are body, so the comment
        // opened in hunk 1's context must still hold in hunk 2.
        let code = "--- a/x.ts\n+++ b/x.ts\n@@ -1,2 +1,2 @@\n /* opened in context\n--- old\n+++ new\n\
                    @@ -9,1 +9,1 @@\n-x\n+const shouldBeComment = 1;\n";
        let ansi = highlight_unified_diff(
            "typescript",
            code,
            DiffStyle::MarkerFg,
            &default_colors(),
            CodeTheme::Mdstream,
            false,
        )
        .unwrap();
        assert_eq!(strip_ansi(&ansi), code);
        let row = ansi
            .split('\n')
            .find(|row| strip_ansi(row).starts_with("+const"))
            .unwrap();
        assert!(
            !row.contains("\x1b[38;2;123;167;255m"),
            "line inside the still-open comment must not be live code: {:?}",
            row
        );
    }

    /// One TS file section: a context `/*` on the first line, `pad` context
    /// lines, a context `*/`, then a change. Correct rendering: live code.
    fn comment_spanning_file(name: &str, pad: usize) -> String {
        let mut body = vec![" /* opens here".to_owned()];
        body.extend((0..pad).map(|i| format!(" const pad{i} = {i};")));
        body.extend([" */".to_owned(), "-const a = 1;".to_owned()]);
        body.push("+const b = 2;".to_owned());
        let count = body.len() - 1;
        format!(
            "--- a/{name}\n+++ b/{name}\n@@ -1,{count} +1,{count} @@\n{}\n",
            body.join("\n")
        )
    }

    fn render_ts(code: &str, budget: usize) -> String {
        highlight_unified_diff_with_budget(
            "typescript",
            code,
            DiffStyle::MarkerFg,
            &default_colors(),
            CodeTheme::Mdstream,
            false,
            budget,
        )
        .unwrap()
    }

    /// Whether each `+const b` row is live code (keyword blue), in order.
    fn added_rows_are_code(ansi: &str) -> Vec<bool> {
        ansi.split('\n')
            .filter(|row| strip_ansi(row).starts_with("+const b"))
            .map(|row| row.contains("\x1b[38;2;123;167;255m"))
            .collect()
    }

    #[test]
    fn over_budget_file_renders_exactly_as_without_continuity() {
        // Regression: the budget used to stop feeding part-way. The `/*` was
        // fed and its `*/` was not, so the changed line rendered as comment,
        // worse than never feeding context at all. Over budget must now be
        // byte-identical to the no-continuity rendering, and correct here.
        let code = comment_spanning_file("x.ts", CONTEXT_LINE_BUDGET + 400);
        let ansi = render_ts(&code, CONTEXT_LINE_BUDGET);
        assert_eq!(strip_ansi(&ansi), code);
        assert_eq!(ansi, render_ts(&code, 0));
        assert_eq!(added_rows_are_code(&ansi), vec![true]);
    }

    #[test]
    fn budget_boundary_is_all_or_nothing() {
        // The fixture's fed context is the `/*` line, `pad` filler lines and
        // the `*/` line: pad + 2 lines, all fed (both streams emit later).
        // Fed in full or not at all, the comment is closed or never seen, so
        // both render the `+` row as code with identical bytes. A feed that
        // stops anywhere before `*/` leaves the comment open and the row
        // comment-colored. So at the exact boundary, on both sides of it, the
        // output must equal the reference.
        let pad = 500;
        let fed = pad + 2;
        let code = comment_spanning_file("x.ts", pad);
        let reference = render_ts(&code, 0);
        assert_eq!(render_ts(&code, usize::MAX), reference);
        assert_eq!(render_ts(&code, fed), reference, "exactly fits");
        assert_eq!(render_ts(&code, fed - 1), reference, "one short");
        assert_eq!(added_rows_are_code(&reference), vec![true]);
        // Premise: the partial-feed shape (closer never lexed) is visible.
        let cut = code.replacen(" */\n", " const cut = 0;\n", 1);
        assert_eq!(
            added_rows_are_code(&render_ts(&cut, usize::MAX)),
            vec![false]
        );
    }

    #[test]
    fn within_budget_file_is_fed_in_full() {
        let code = comment_spanning_file("x.ts", CONTEXT_LINE_BUDGET - 10);
        assert_eq!(
            added_rows_are_code(&render_ts(&code, CONTEXT_LINE_BUDGET)),
            vec![true]
        );
        // And the feed is live: an unclosed comment does reach the `+` line.
        let open = "--- a/x.ts\n+++ b/x.ts\n@@ -1,2 +1,2 @@\n /* never closed\n-a\n+const b = 2;\n";
        assert_eq!(
            added_rows_are_code(&render_ts(open, CONTEXT_LINE_BUDGET)),
            vec![false]
        );
        assert_eq!(added_rows_are_code(&render_ts(open, 0)), vec![true]);
    }

    #[test]
    fn budget_is_admitted_per_file() {
        // A huge first file is skipped whole; a small file after it still gets
        // continuity, because files are independent sections.
        let small_open =
            "--- a/s.ts\n+++ b/s.ts\n@@ -1,2 +1,2 @@\n /* never closed\n-a\n+const b = 2;\n";
        let code = format!(
            "{}{small_open}",
            comment_spanning_file("big.ts", CONTEXT_LINE_BUDGET + 400)
        );
        let ansi = render_ts(&code, CONTEXT_LINE_BUDGET);
        assert_eq!(strip_ansi(&ansi), code);
        // big.ts: unfed, so `*/`-closed comment is irrelevant and the row is
        // code. s.ts: fed, so the unclosed comment holds and the row is not.
        assert_eq!(added_rows_are_code(&ansi), vec![true, false]);
    }

    #[test]
    fn admit_files_is_all_or_nothing_in_patch_order() {
        use Role::*;
        // Section 0 (pre-header): 1 fed line. File 1: 3. File 2: 2.
        let roles = [Body, FileStart, Body, Body, Body, FileStart, Body, Body];
        let fed = [true, false, true, true, true, false, true, true];
        assert_eq!(admit_files(&roles, &fed, 6), vec![true, true, true]);
        assert_eq!(admit_files(&roles, &fed, 4), vec![true, true, false]);
        assert_eq!(admit_files(&roles, &fed, 3), vec![true, false, true]);
        assert_eq!(admit_files(&roles, &fed, 0), vec![false, false, false]);
    }

    #[test]
    fn line_roles_follow_hunk_counts() {
        let code = "diff --git a/x b/x\nindex 1..2\n--- a/x\n+++ b/x\n@@ -1,2 +1 @@\n--- body\n ctx\n@@ -9 +8,2 @@\n+++ body\n+y\n z\n";
        let lines: Vec<&str> = code.split_inclusive('\n').collect();
        use Role::*;
        assert_eq!(
            line_roles(&lines),
            vec![
                // `---` after `diff --git` resets again: harmless, no lexing yet.
                FileStart, Structure, FileStart, Structure, Structure, Body, Body, Structure, Body,
                Body, Body
            ]
        );
        assert_eq!(hunk_counts("@@ -1,2 +3 @@ fn x()"), Some((2, 1)));
        assert_eq!(hunk_counts("@@"), None);
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
