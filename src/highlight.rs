use std::sync::OnceLock;

use syntect::easy::HighlightLines;
use syntect::highlighting::{Theme, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::as_24_bit_terminal_escaped;

use crate::theme::{CodeTheme, load_theme_set};

const RESET: &str = "\x1b[0m";

/// The ANSI escape emitted to set an 8-bit foreground color (e.g. `#ff5d7a`).
pub(crate) fn fg_escape(hex: &str) -> Option<String> {
    let hex = hex.strip_prefix('#').unwrap_or(hex);
    if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(format!("\x1b[38;2;{r};{g};{b}m"))
}

/// The ANSI escape emitted to set an 8-bit background color (e.g. `#ff5d7a`).
pub(crate) fn bg_escape(hex: &str) -> Option<String> {
    let hex = hex.strip_prefix('#').unwrap_or(hex);
    if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(format!("\x1b[48;2;{r};{g};{b}m"))
}

/// Re-asserts a per-line background after every `\x1b[0m` reset in `escaped`.
///
/// `as_24_bit_terminal_escaped` styles each span and appends a trailing
/// `\x1b[0m` which clears foreground AND background together. To paint a
/// per-line wash while keeping token foreground colors, the wash must be
/// re-asserted after every reset syntect emits. Returns the escaped text with
/// the wash re-asserted, or the input unchanged when `wash` is `None`.
pub(crate) fn reassert_wash(escaped: &str, wash: &Option<String>) -> String {
    let Some(wash) = wash else {
        return escaped.to_owned();
    };
    if !escaped.contains(RESET) {
        return format!("{wash}{escaped}");
    }
    let mut out = String::with_capacity(escaped.len() + wash.len() * 4);
    out.push_str(wash);
    let mut rest = escaped;
    while let Some(idx) = rest.find(RESET) {
        out.push_str(&rest[..idx + RESET.len()]);
        out.push_str(wash);
        rest = &rest[idx + RESET.len()..];
    }
    out.push_str(rest);
    out
}

pub(crate) struct SyntectAssets {
    pub(crate) syntax_set: SyntaxSet,
    theme_set: ThemeSet,
}

impl SyntectAssets {
    fn load() -> Self {
        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: load_theme_set(),
        }
    }

    fn theme(&self, code_theme: CodeTheme) -> &Theme {
        self.theme_set
            .themes
            .get(code_theme.theme_key())
            .or_else(|| self.theme_set.themes.values().next())
            .expect("syntect default themes should not be empty")
    }
}

pub(crate) fn syntect_assets() -> &'static SyntectAssets {
    static ASSETS: OnceLock<SyntectAssets> = OnceLock::new();
    ASSETS.get_or_init(SyntectAssets::load)
}

pub(crate) fn make_highlighter(language: &str, code_theme: CodeTheme) -> HighlightLines<'static> {
    let assets = syntect_assets();
    let syntax = assets
        .syntax_set
        .find_syntax_by_token(normalize_syntax_token(language))
        .unwrap_or_else(|| assets.syntax_set.find_syntax_plain_text());
    HighlightLines::new(syntax, assets.theme(code_theme))
}

/// A persistent per-stream highlighter for dual-stream diff composition.
///
/// Each stream (deleted/from, added/to) keeps its own lexical state so an open
/// comment on the deleted side cannot poison the added side. `highlight_line`
/// returns the escaped content for one logical line, without the trailing
/// newline (the caller appends framing and the line ending).
pub(crate) struct LineHighlighter {
    highlighter: HighlightLines<'static>,
    show_background: bool,
}

impl LineHighlighter {
    pub(crate) fn new(language: &str, code_theme: CodeTheme, show_background: bool) -> Self {
        Self {
            highlighter: make_highlighter(language, code_theme),
            show_background,
        }
    }

    pub(crate) fn highlight_line(&mut self, content: &str) -> Result<String, syntect::Error> {
        let assets = syntect_assets();
        let mut parse_line = String::with_capacity(content.len() + 1);
        parse_line.push_str(content);
        parse_line.push('\n');
        let regions = self
            .highlighter
            .highlight_line(&parse_line, &assets.syntax_set)?;
        let escaped = as_24_bit_terminal_escaped(&regions, self.show_background);
        Ok(escaped.trim_end_matches('\n').to_owned())
    }
}

fn normalize_syntax_token(language: &str) -> &str {
    if matches!(
        language.trim().to_ascii_lowercase().as_str(),
        "typescript" | "ts" | "mts" | "cts" | "tsx"
    ) {
        "javascript"
    } else {
        language
    }
}

/// Highlights raw source code without adding Markdown framing, line numbers,
/// indentation, or padding.
///
/// Syntect's syntax and theme assets are shared process-wide and loaded by
/// [`RawCodeHighlighter::new`]. Each call to [`Self::highlight`] starts with a
/// fresh lexical state, while state is retained between lines in that call.
pub struct RawCodeHighlighter {
    code_theme: CodeTheme,
    show_background: bool,
}

impl RawCodeHighlighter {
    /// Creates a raw-code highlighter and eagerly initializes shared syntect
    /// syntax and theme assets.
    pub fn new(code_theme: CodeTheme, show_background: bool) -> Self {
        let _ = syntect_assets();
        Self {
            code_theme,
            show_background,
        }
    }

    /// Highlights one complete source string.
    ///
    /// Unknown languages use syntect's plain-text syntax. Input line endings
    /// and the presence or absence of a final newline are preserved. Every
    /// non-empty logical output line ends with an ANSI reset before its line
    /// ending, so terminal styling cannot leak into later output.
    pub fn highlight(&self, language: &str, code: &str) -> Result<String, syntect::Error> {
        self.highlight_with_washes(language, code, None)
    }

    /// Like [`Self::highlight`], but paints an optional per-line background
    /// wash on each logical line.
    ///
    /// `washes` must have exactly one entry per logical line of `code` (split
    /// preserving `\r\n` and the trailing-newline shape): `None` for no wash on
    /// that line, `Some(hex)` for a `#rrggbb` background. The wash is re-asserted
    /// after every `\x1b[0m` reset within the line and cleared before the line
    /// ending, so it never leaks across lines. Pass `None` for `washes` to get
    /// byte-identical output to [`Self::highlight`].
    pub fn highlight_with_washes(
        &self,
        language: &str,
        code: &str,
        washes: Option<&[Option<String>]>,
    ) -> Result<String, syntect::Error> {
        if code.is_empty() {
            return Ok(String::new());
        }

        let assets = syntect_assets();
        let mut highlighter = make_highlighter(language, self.code_theme);
        let mut output = String::with_capacity(code.len());

        for (line_idx, line) in code.split_inclusive('\n').enumerate() {
            let (content, ending) = if let Some(content) = line.strip_suffix("\r\n") {
                (content, "\r\n")
            } else if let Some(content) = line.strip_suffix('\n') {
                (content, "\n")
            } else {
                (line, "")
            };

            let mut parse_line = String::with_capacity(content.len() + 1);
            parse_line.push_str(content);
            parse_line.push('\n');
            let regions = highlighter.highlight_line(&parse_line, &assets.syntax_set)?;
            let escaped = as_24_bit_terminal_escaped(&regions, self.show_background);
            let wash = washes.and_then(|w| w.get(line_idx)).and_then(|w| w.clone());
            let escaped = reassert_wash(escaped.trim_end_matches('\n'), &wash);
            output.push_str(&escaped);
            output.push_str(RESET);
            output.push_str(ending);
        }

        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strip_ansi(text: &str) -> String {
        let ansi = regex::Regex::new(r"\x1b\[[0-9;?]*[ -/]*[@-~]").unwrap();
        ansi.replace_all(text, "").into_owned()
    }

    #[test]
    fn preserves_multiline_lexical_state() {
        let highlighter = RawCodeHighlighter::new(CodeTheme::Mdstream, false);
        let code = "/* first\nstill comment */\nconst value = 1;";
        let ansi = highlighter.highlight("typescript", code).unwrap();

        assert_eq!(strip_ansi(&ansi), code);
        assert!(ansi.contains("\x1b[38;2;113;123;148m"));
        assert!(ansi.contains("\x1b[38;2;123;167;255m"));
    }

    #[test]
    fn preserves_trailing_newline_shape() {
        let highlighter = RawCodeHighlighter::new(CodeTheme::Mdstream, false);

        for code in ["const x = 1;", "const x = 1;\n", "a\n\n", "a\r\nb\r\n"] {
            let ansi = highlighter.highlight("typescript", code).unwrap();
            assert_eq!(strip_ansi(&ansi), code);
            assert_eq!(ansi.ends_with('\n'), code.ends_with('\n'));
        }
    }

    #[test]
    fn diff_lines_use_distinct_theme_colors() {
        // Regression: ` ```diff ` fences were lexed by syntect's Diff grammar
        // but rendered monochrome under the default mdstream theme, because the
        // theme mapped no diff scopes. Each line kind must get its own color.
        let highlighter = RawCodeHighlighter::new(CodeTheme::Mdstream, false);
        let code = "@@ -1,3 +1,3 @@\n-old line\n+new line\n context\n";
        let ansi = highlighter.highlight("diff", code).unwrap();

        let range_gray = "\x1b[38;2;113;123;148m"; // meta.diff.range -> comment
        let deleted_red = "\x1b[38;2;255;93;122m"; // markup.deleted.diff
        let inserted_green = "\x1b[38;2;120;227;140m"; // markup.inserted.diff -> string

        let lines: Vec<&str> = ansi.split('\n').collect();
        assert_eq!(lines.len(), 5);
        assert!(
            lines[0].contains(range_gray),
            "@@ hunk header not gray: {}",
            lines[0]
        );
        assert!(
            lines[1].contains(deleted_red),
            "deleted line not red: {}",
            lines[1]
        );
        assert!(
            lines[2].contains(inserted_green),
            "inserted line not green: {}",
            lines[2]
        );
        assert!(
            !lines[3].contains(deleted_red)
                && !lines[3].contains(inserted_green)
                && !lines[3].contains(range_gray),
            "context line must stay base-colored: {}",
            lines[3]
        );
    }

    #[test]
    fn highlight_with_washes_is_byte_identical_when_all_none() {
        let highlighter = RawCodeHighlighter::new(CodeTheme::Mdstream, false);
        for code in [
            "const x = 1;",
            "const x = 1;\n",
            "a\n\n",
            "a\r\nb\r\n",
            "/* c\nstill c */\nlet y: number = 2;",
        ] {
            let plain = highlighter.highlight("typescript", code).unwrap();
            let washes: Vec<Option<String>> = code.lines().map(|_| None).collect();
            let washed = highlighter
                .highlight_with_washes("typescript", code, Some(&washes))
                .unwrap();
            assert_eq!(washed, plain, "code: {:?}", code);
        }
    }

    #[test]
    fn wash_reasserts_after_each_reset_and_clears_at_line_end() {
        // Directly exercise the re-assert primitive with a hand-built string
        // containing an internal reset (the case that would otherwise clear
        // the wash mid-line).
        let wash = Some("\x1b[48;2;255;93;122m".to_string());
        let escaped = "\x1b[38;2;123;167;255mlet\x1b[0m rest";
        let reasserted = reassert_wash(escaped, &wash);
        // Wash leads, is re-asserted after the internal reset.
        assert!(reasserted.starts_with("\x1b[48;2;255;93;122m"));
        assert!(reasserted.contains("\x1b[0m\x1b[48;2;255;93;122m rest"));
        assert_eq!(reasserted.matches("\x1b[48;2;255;93;122m").count(), 2);

        // Through the public API: a washed line leads with wash and ends
        // reset (the caller's trailing RESET clears it, so it never leaks
        // past the newline); an unwashed sibling line is wash-free.
        let highlighter = RawCodeHighlighter::new(CodeTheme::Mdstream, false);
        let code = "let s: string = \"hi\"; // trailing\nnext";
        let wash = "\x1b[48;2;255;93;122m".to_string();
        let washes = vec![Some(wash.clone()), None];
        let ansi = highlighter
            .highlight_with_washes("typescript", code, Some(&washes))
            .unwrap();
        let lines: Vec<&str> = ansi.split('\n').collect();
        assert!(
            lines[0].starts_with(&wash),
            "line must open with wash: {:?}",
            lines[0]
        );
        assert!(
            lines[0].ends_with("\x1b[0m"),
            "line must end reset: {:?}",
            lines[0]
        );
        assert!(
            !lines[1].contains(&wash),
            "second line must be wash-free: {:?}",
            lines[1]
        );
    }

    #[test]
    fn wash_preserves_trailing_newline_shape() {
        let highlighter = RawCodeHighlighter::new(CodeTheme::Mdstream, false);
        let wash = "\x1b[48;2;0;255;0m".to_string();
        for code in ["a\n", "a", "a\r\nb\r\n", "a\n\n"] {
            let count = if code.ends_with('\n') {
                code.lines().count()
            } else {
                1
            };
            let washes: Vec<Option<String>> = (0..count).map(|_| Some(wash.clone())).collect();
            let ansi = highlighter
                .highlight_with_washes("plaintext", code, Some(&washes))
                .unwrap();
            assert_eq!(
                ansi.ends_with('\n'),
                code.ends_with('\n'),
                "code: {:?}",
                code
            );
            assert!(
                !ansi.ends_with(&wash),
                "wash must not leak past final newline: {:?}",
                ansi
            );
        }
    }
}
