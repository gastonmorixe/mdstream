use std::sync::OnceLock;

use syntect::easy::HighlightLines;
use syntect::highlighting::{Theme, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::as_24_bit_terminal_escaped;

use crate::theme::{CodeTheme, load_theme_set};

const RESET: &str = "\x1b[0m";

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
        if code.is_empty() {
            return Ok(String::new());
        }

        let assets = syntect_assets();
        let mut highlighter = make_highlighter(language, self.code_theme);
        let mut output = String::with_capacity(code.len());

        for line in code.split_inclusive('\n') {
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
            output.push_str(escaped.trim_end_matches('\n'));
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
}
