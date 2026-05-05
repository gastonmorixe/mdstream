use crate::theme::{
    CodeTheme, DEFAULT_CODE_THEME, DEFAULT_INLINE_CODE_COLOR, DEFAULT_SHOW_CODE_BACKGROUND,
    PaletteColor, load_theme_set,
};
use anyhow::Result;
use crossterm::terminal;
use regex::Regex;
use std::io::{Read, Write};
use std::sync::OnceLock;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Theme, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::as_24_bit_terminal_escaped;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const ITALIC: &str = "\x1b[3m";
const UNDERLINE: &str = "\x1b[4m";
const STRIKETHROUGH: &str = "\x1b[9m";
const BRIGHT_BLUE: &str = "\x1b[94m";
const BRIGHT_GREEN: &str = "\x1b[92m";
const BRIGHT_MAGENTA: &str = "\x1b[95m";

const LIST_BULLETS: &[&str] = &["•", "◦", "▪", "‣"];

const LANG_COLOR_DEFAULT: &str = DIM;

const LANG_COLORS: &[(&str, &str)] = &[
    ("rust", "\x1b[38;2;255;140;60m"),
    ("c", "\x1b[38;2;100;160;255m"),
    ("cpp", "\x1b[38;2;100;160;255m"),
    ("python", "\x1b[38;2;80;180;255m"),
    ("py", "\x1b[38;2;80;180;255m"),
    ("javascript", "\x1b[38;2;255;220;60m"),
    ("js", "\x1b[38;2;255;220;60m"),
    ("typescript", "\x1b[38;2;50;150;255m"),
    ("ts", "\x1b[38;2;50;150;255m"),
    ("go", "\x1b[38;2;0;173;216m"),
    ("swift", "\x1b[38;2;255;100;50m"),
    ("html", "\x1b[38;2;255;100;50m"),
    ("css", "\x1b[38;2;50;130;255m"),
    ("bash", "\x1b[38;2;190;150;80m"),
    ("sh", "\x1b[38;2;190;150;80m"),
    ("zsh", "\x1b[38;2;100;200;100m"),
    ("ruby", "\x1b[38;2;220;60;60m"),
    ("java", "\x1b[38;2;240;130;50m"),
    ("kotlin", "\x1b[38;2;180;100;255m"),
    ("sql", "\x1b[38;2;200;200;100m"),
    ("shell", "\x1b[38;2;190;150;80m"),
    ("fish", "\x1b[38;2;190;150;80m"),
    ("json", "\x1b[38;2;180;180;180m"),
    ("yaml", "\x1b[38;2;180;180;180m"),
    ("toml", "\x1b[38;2;180;180;180m"),
    ("markdown", "\x1b[38;2;180;180;180m"),
    ("md", "\x1b[38;2;180;180;180m"),
    ("lua", "\x1b[38;2;50;50;200m"),
    ("php", "\x1b[38;2;120;120;200m"),
    ("r", "\x1b[38;2;40;100;200m"),
    ("elixir", "\x1b[38;2;120;80;160m"),
    ("haskell", "\x1b[38;2;120;100;160m"),
    ("zig", "\x1b[38;2;255;180;50m"),
    ("nim", "\x1b[38;2;255;220;80m"),
    ("dart", "\x1b[38;2;0;180;220m"),
    ("scala", "\x1b[38;2;200;50;50m"),
];

fn heading_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^(#{1,6})\s+(.*)$").unwrap())
}

fn fence_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^(\s*)(```+|~~~+)(.*)$").unwrap())
}

fn task_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^(\s*)([-*+])\s+\[([ xX])\]\s+(.*)$").unwrap())
}

fn list_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^(\s*)([-*+])\s+(.*)$").unwrap())
}

fn ordered_re() -> &'static Regex {
    // Accepts both single-integer markers (`1.`) and explicit hierarchical
    // markers typed in source (`1.2.3.`), matching Python mdstream.py:153.
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^(\s*)((?:\d+\.)*\d+)\.?\s+(.*)$").unwrap())
}

fn rule_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^(\s*[-*_]\s*){3,}$").unwrap())
}

fn bold_italic_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\*\*\*(.+?)\*\*\*").unwrap())
}

fn bold_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\*\*(.+?)\*\*").unwrap())
}

fn italic_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\*([^*\n]+)\*").unwrap())
}

fn strike_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"~~(.+?)~~").unwrap())
}

fn code_span_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"`([^`]+)`").unwrap())
}

fn image_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"!\[([^\]]*)\]\(([^)]+)\)").unwrap())
}

fn link_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\[([^\]]+)\]\(([^)]+)\)").unwrap())
}

fn autolink_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"<(https?://[^>\s]+)>").unwrap())
}

fn bare_url_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"https?://[^\s<>()]+").unwrap())
}

fn ansi_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\x1b\[[0-9;?]*[ -/]*[@-~]").unwrap())
}

fn table_candidate_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^\s*\|?.+\|.+\|?\s*$").unwrap())
}

fn table_separator_cell_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^:?-{3,}:?$").unwrap())
}

struct SyntectAssets {
    syntax_set: SyntaxSet,
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

fn syntect_assets() -> &'static SyntectAssets {
    static ASSETS: OnceLock<SyntectAssets> = OnceLock::new();
    ASSETS.get_or_init(SyntectAssets::load)
}

fn lang_color(lang: &str) -> &'static str {
    LANG_COLORS
        .iter()
        .find_map(|(candidate, color)| (*candidate == lang).then_some(*color))
        .unwrap_or(LANG_COLOR_DEFAULT)
}

fn normalize_syntax_token(lang: &str) -> &str {
    if matches!(
        lang.trim().to_ascii_lowercase().as_str(),
        "typescript" | "ts" | "mts" | "cts" | "tsx"
    ) {
        "javascript"
    } else {
        lang
    }
}

fn split_blockquote(line: &str) -> (usize, &str) {
    // Strip any leading whitespace (tabs, spaces, NBSP, ...) before the first
    // `>` to match Python `_split_blockquote` (mdstream.py:236, `\s*` prefix).
    let mut rest = line.trim_start_matches(char::is_whitespace);
    let mut depth = 0;

    while let Some(after) = rest.strip_prefix('>') {
        depth += 1;
        rest = after.strip_prefix(' ').unwrap_or(after);
    }

    if depth == 0 { (0, line) } else { (depth, rest) }
}

fn ignored_html_wrapper_tag(line: &str) -> bool {
    let stripped = line.trim();
    if !(stripped.starts_with('<') && stripped.ends_with('>')) {
        return false;
    }
    if stripped.contains("://") {
        return false;
    }

    let inner = stripped
        .trim_start_matches('<')
        .trim_end_matches('>')
        .trim_start_matches('/')
        .trim_end_matches('/')
        .trim();
    let tag = inner
        .chars()
        .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '-')
        .collect::<String>()
        .to_ascii_lowercase();

    matches!(tag.as_str(), "div" | "span" | "p" | "center")
}

fn split_table_row(line: &str) -> Option<Vec<String>> {
    let mut stripped = line.trim();
    if !stripped.contains('|') {
        return None;
    }
    if let Some(rest) = stripped.strip_prefix('|') {
        stripped = rest;
    }
    if let Some(rest) = stripped.strip_suffix('|') {
        stripped = rest;
    }
    let mut cells = Vec::new();
    let mut current = String::new();
    let mut chars = stripped.chars().peekable();
    let mut code_span_ticks: Option<usize> = None;

    while let Some(ch) = chars.next() {
        if ch == '\\' {
            current.push(ch);
            if let Some(next) = chars.next() {
                current.push(next);
            }
            continue;
        }

        if ch == '`' {
            let mut tick_count = 1usize;
            while matches!(chars.peek(), Some('`')) {
                chars.next();
                tick_count += 1;
            }
            current.push_str(&"`".repeat(tick_count));
            match code_span_ticks {
                Some(active) if active == tick_count => code_span_ticks = None,
                None => code_span_ticks = Some(tick_count),
                _ => {}
            }
            continue;
        }

        if ch == '|' && code_span_ticks.is_none() {
            cells.push(current.trim().to_owned());
            current.clear();
            continue;
        }

        current.push(ch);
    }
    cells.push(current.trim().to_owned());
    if cells.len() < 2 { None } else { Some(cells) }
}

fn parse_table_separator(line: &str) -> Option<Vec<Alignment>> {
    let cells = split_table_row(line)?;
    let mut alignments = Vec::with_capacity(cells.len());
    for cell in cells {
        if !table_separator_cell_re().is_match(&cell) {
            return None;
        }
        let alignment = if cell.starts_with(':') && cell.ends_with(':') {
            Alignment::Center
        } else if cell.ends_with(':') {
            Alignment::Right
        } else {
            Alignment::Left
        };
        alignments.push(alignment);
    }
    Some(alignments)
}

fn looks_like_table_row(line: &str) -> bool {
    let stripped = line.trim();
    !stripped.is_empty()
        && table_candidate_re().is_match(stripped)
        && split_table_row(stripped).is_some()
}

fn block_prefix(base_pad: &str, quote_depth: usize) -> String {
    if quote_depth == 0 {
        return base_pad.to_owned();
    }

    let mut prefix = String::from(base_pad);
    prefix.push_str("  ");
    for _ in 0..quote_depth {
        prefix.push_str(DIM);
        prefix.push('│');
        prefix.push_str(RESET);
        prefix.push(' ');
    }
    prefix
}

fn visible_width(text: &str) -> usize {
    let plain = ansi_re().replace_all(text, "");
    UnicodeWidthStr::width(plain.as_ref())
}

fn indent_columns(indent: &str) -> usize {
    let mut total = 0;
    for ch in indent.chars() {
        if ch == '\t' {
            total = ((total / 4) + 1) * 4;
        } else {
            total += 1;
        }
    }
    total
}

fn format_ordered_marker(path: &[String]) -> (String, usize) {
    if path.len() == 1 {
        let marker = format!("{}.", path[0]);
        let styled = format!("{BOLD}{}{RESET}.", path[0]);
        return (styled, marker.len());
    }

    let faded = path[..path.len() - 1].join(".");
    let bright = format!(".{}", path[path.len() - 1]);
    let styled = format!("{DIM}{faded}{RESET}{BOLD}{bright}{RESET}");
    (styled, faded.len() + bright.len())
}

fn align_cell(text: &str, width: usize, alignment: Alignment) -> String {
    let pad = width.saturating_sub(visible_width(text));
    match alignment {
        Alignment::Left => format!("{text}{}", " ".repeat(pad)),
        Alignment::Right => format!("{}{text}", " ".repeat(pad)),
        Alignment::Center => {
            let left = pad / 2;
            let right = pad - left;
            format!("{}{}{}", " ".repeat(left), text, " ".repeat(right))
        }
    }
}

fn stash_placeholder(value: String, placeholders: &mut Vec<String>) -> String {
    let key = format!("\u{0}MDSTREAM{}\u{0}", placeholders.len());
    placeholders.push(value);
    key
}

fn restore_placeholders(mut text: String, placeholders: &[String]) -> String {
    // Iterate highest-index first. Outer stashes (code span, link, image,
    // ...) always carry larger indices than the inner stashes they enclose,
    // because `stash_placeholder` uses `placeholders.len()` at the moment of
    // stashing. Reversing the loop guarantees we expand the outer
    // placeholder first, which injects inner keys back into `text`, and the
    // subsequent inner substitutions then resolve them in the same pass. A
    // forward loop would expand the outer placeholder only after the loop
    // had moved past the inner indices, leaving literal `\u{0}MDSTREAMn\u{0}`
    // markers in the output (visible as `MDSTREAMn` on terminals that drop
    // NUL bytes).
    for (index, value) in placeholders.iter().enumerate().rev() {
        let key = format!("\u{0}MDSTREAM{}\u{0}", index);
        text = text.replace(&key, value);
    }
    text
}

fn render_link(label: &str, url: &str) -> String {
    format!("{UNDERLINE}{BRIGHT_BLUE}{label}{RESET}{DIM} ({url}){RESET}")
}

fn render_image(alt: &str, url: &str) -> String {
    let label = if alt.is_empty() { url } else { alt };
    format!("{DIM}Image:{RESET} {BRIGHT_MAGENTA}{label}{RESET}{DIM} ({url}){RESET}")
}

fn decode_basic_html_entities(text: &str) -> String {
    text.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

fn apply_escapes(text: &str, placeholders: &mut Vec<String>) -> String {
    let mut out = String::new();
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\'
            && let Some(next) = chars.peek().copied()
            && "\\`*_{}[]()#+-.!>~|".contains(next)
        {
            chars.next();
            out.push_str(&stash_placeholder(next.to_string(), placeholders));
            continue;
        }
        out.push(ch);
    }
    out
}

fn format_inline(text: &str, inline_code_color: PaletteColor) -> String {
    let mut placeholders = Vec::<String>::new();
    let decoded = decode_basic_html_entities(text);
    let mut current = apply_escapes(&decoded, &mut placeholders);

    current = code_span_re()
        .replace_all(&current, |captures: &regex::Captures| {
            stash_placeholder(
                format!(
                    "{BOLD}{}{}{RESET}",
                    inline_code_color.ansi_escape(),
                    captures.get(1).map(|m| m.as_str()).unwrap_or_default()
                ),
                &mut placeholders,
            )
        })
        .into_owned();
    current = image_re()
        .replace_all(&current, |captures: &regex::Captures| {
            stash_placeholder(
                render_image(
                    captures.get(1).map(|m| m.as_str()).unwrap_or_default(),
                    captures.get(2).map(|m| m.as_str()).unwrap_or_default(),
                ),
                &mut placeholders,
            )
        })
        .into_owned();
    current = link_re()
        .replace_all(&current, |captures: &regex::Captures| {
            stash_placeholder(
                render_link(
                    captures.get(1).map(|m| m.as_str()).unwrap_or_default(),
                    captures.get(2).map(|m| m.as_str()).unwrap_or_default(),
                ),
                &mut placeholders,
            )
        })
        .into_owned();
    current = autolink_re()
        .replace_all(&current, |captures: &regex::Captures| {
            stash_placeholder(
                format!(
                    "{UNDERLINE}{BRIGHT_BLUE}{}{RESET}",
                    captures.get(1).map(|m| m.as_str()).unwrap_or_default()
                ),
                &mut placeholders,
            )
        })
        .into_owned();
    // Manual walk so we can mirror Python's `(?<![\w/])` lookbehind on
    // `_BARE_URL_RE` (mdstream.py:149). The `regex` crate cannot express
    // lookbehind, so we inspect the preceding ASCII byte and skip the match
    // when it would glue the URL onto a word character or path segment.
    current = {
        let mut result = String::new();
        let mut last = 0;
        for mat in bare_url_re().find_iter(&current) {
            let start = mat.start();
            if start > 0 {
                let prev = current.as_bytes()[start - 1];
                if prev == b'/' || prev == b'_' || prev.is_ascii_alphanumeric() {
                    continue;
                }
            }
            result.push_str(&current[last..start]);
            result.push_str(&stash_placeholder(
                format!("{UNDERLINE}{BRIGHT_BLUE}{}{RESET}", mat.as_str()),
                &mut placeholders,
            ));
            last = mat.end();
        }
        result.push_str(&current[last..]);
        result
    };
    current = bold_italic_re()
        .replace_all(&current, format!("{BOLD}{ITALIC}$1{RESET}"))
        .into_owned();
    current = bold_re()
        .replace_all(&current, format!("{BOLD}$1{RESET}"))
        .into_owned();
    current = italic_re()
        .replace_all(&current, format!("{ITALIC}$1{RESET}"))
        .into_owned();
    current = strike_re()
        .replace_all(&current, format!("{STRIKETHROUGH}$1{RESET}"))
        .into_owned();

    restore_placeholders(current, &placeholders)
}

pub struct StreamingMarkdownRenderer {
    pub partial: String,
    pad: String,
    previous_was_blank: bool,
    in_code_block: bool,
    code_lang: String,
    code_line_num: usize,
    show_lineno: bool,
    code_theme: CodeTheme,
    inline_code_color: PaletteColor,
    show_code_background: bool,
    code_highlighter: Option<HighlightLines<'static>>,
    pending_table_header: Option<String>,
    table_lines: Vec<String>,
    table_alignments: Vec<Alignment>,
    list_indent_stack: Vec<usize>,
    list_level_meta: Vec<ListLevelMeta>,
    active_list_context: Option<ListContext>,
    show_list_guides: bool,
    term_width_override: Option<usize>,
    /// Number of fully-wrapped rows the current partial occupies ABOVE the
    /// cursor's current row. Total visual rows for the partial =
    /// `partial_rows + 1`. Reset to 0 whenever `partial` is cleared, when a
    /// `render_line` flush returns the cursor to col 0 of a fresh row, or
    /// after `erase_partial` rewinds to the start of the partial.
    partial_rows: usize,
    /// Visual column the terminal cursor currently sits at within the
    /// bottom row of the partial. Tracked using the terminal width that
    /// was live at the moment each character of `partial` (and its
    /// preceding `pad`) was emitted, so the count is robust against
    /// mid-stream resizes — `term_width()` may report a different value
    /// at erase time than it did at emit time, but we already accounted
    /// for the wraps that actually happened on the user's terminal.
    partial_col: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Alignment {
    Left,
    Center,
    Right,
}

#[derive(Clone, Debug)]
enum ListKind {
    Unordered,
    Ordered,
    Task,
}

#[derive(Clone, Debug)]
struct ListLevelMeta {
    kind: ListKind,
    path: Vec<String>,
}

#[derive(Clone, Debug)]
struct ListContext {
    source_indent_width: usize,
    continuation_prefix: String,
}

impl StreamingMarkdownRenderer {
    pub fn new(padding: usize, show_lineno: bool, show_list_guides: bool) -> Self {
        Self::with_code_theme(
            padding,
            show_lineno,
            show_list_guides,
            DEFAULT_CODE_THEME,
            DEFAULT_SHOW_CODE_BACKGROUND,
        )
    }

    pub fn with_code_theme(
        padding: usize,
        show_lineno: bool,
        show_list_guides: bool,
        code_theme: CodeTheme,
        show_code_background: bool,
    ) -> Self {
        Self::with_code_theme_and_inline_code_color(
            padding,
            show_lineno,
            show_list_guides,
            code_theme,
            show_code_background,
            DEFAULT_INLINE_CODE_COLOR,
        )
    }

    pub fn with_code_theme_and_inline_code_color(
        padding: usize,
        show_lineno: bool,
        show_list_guides: bool,
        code_theme: CodeTheme,
        show_code_background: bool,
        inline_code_color: PaletteColor,
    ) -> Self {
        Self {
            partial: String::new(),
            pad: " ".repeat(padding),
            previous_was_blank: true,
            in_code_block: false,
            code_lang: String::new(),
            code_line_num: 0,
            show_lineno,
            code_theme,
            inline_code_color,
            show_code_background,
            code_highlighter: None,
            pending_table_header: None,
            table_lines: Vec::new(),
            table_alignments: Vec::new(),
            list_indent_stack: Vec::new(),
            list_level_meta: Vec::new(),
            active_list_context: None,
            show_list_guides,
            term_width_override: None,
            partial_rows: 0,
            partial_col: 0,
        }
    }

    pub fn from_env() -> Self {
        let padding = std::env::var("MDSTREAM_PADDING")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(0);
        let show_lineno = std::env::var("MDSTREAM_NO_LINENO").is_err();
        let show_list_guides = std::env::var("MDSTREAM_NO_LIST_GUIDES").is_err();
        let code_theme = std::env::var("MDSTREAM_THEME")
            .ok()
            .and_then(|value| CodeTheme::parse(&value).ok())
            .unwrap_or(DEFAULT_CODE_THEME);
        let inline_code_color = std::env::var("MDSTREAM_INLINE_CODE_COLOR")
            .ok()
            .and_then(|value| PaletteColor::parse(&value).ok())
            .unwrap_or(DEFAULT_INLINE_CODE_COLOR);
        let show_code_background = std::env::var("MDSTREAM_CODE_BACKGROUND")
            .ok()
            .map(|_| true)
            .unwrap_or_else(|| {
                std::env::var("MDSTREAM_NO_CODE_BACKGROUND")
                    .ok()
                    .map(|_| false)
                    .unwrap_or(DEFAULT_SHOW_CODE_BACKGROUND)
            });
        Self::with_code_theme_and_inline_code_color(
            padding,
            show_lineno,
            show_list_guides,
            code_theme,
            show_code_background,
            inline_code_color,
        )
    }

    pub fn render_line(&mut self, line: &str) -> String {
        let stripped = line.trim_end_matches('\n');
        let ignored_html = !self.in_code_block && ignored_html_wrapper_tag(stripped);
        let rendered = if ignored_html {
            self.clear_list_state();
            self.flush_buffered_table()
        } else if self.in_code_block {
            self.render_code_line(stripped)
        } else {
            self.render_noncode_line(stripped)
        };
        self.previous_was_blank = stripped.is_empty() || ignored_html;
        rendered
    }

    pub fn write_chunk<W: Write>(&mut self, chunk: &str, out: &mut W) -> Result<()> {
        let parts: Vec<&str> = chunk.split('\n').collect();
        if parts.len() == 1 {
            if self.partial.is_empty() {
                write!(out, "{}", self.pad)?;
                self.advance_partial_position(&self.pad.clone());
            }
            self.partial.push_str(parts[0]);
            write!(out, "{}", parts[0])?;
            self.advance_partial_position(parts[0]);
            return Ok(());
        }

        let first_complete = format!("{}{}", self.partial, parts[0]);
        self.erase_partial(out)?;
        self.partial.clear();
        self.partial_rows = 0;
        self.partial_col = 0;
        write!(out, "{}", self.render_line(&(first_complete + "\n")))?;

        for part in &parts[1..parts.len() - 1] {
            write!(out, "{}", self.render_line(&((*part).to_owned() + "\n")))?;
        }

        self.partial = parts[parts.len() - 1].to_owned();
        if !self.partial.is_empty() {
            write!(out, "{}", self.pad)?;
            self.advance_partial_position(&self.pad.clone());
            let new_partial = self.partial.clone();
            write!(out, "{}", new_partial)?;
            self.advance_partial_position(&new_partial);
        }
        Ok(())
    }

    pub fn finish<W: Write>(&mut self, out: &mut W) -> Result<()> {
        if !self.partial.is_empty() {
            self.erase_partial(out)?;
            let partial = std::mem::take(&mut self.partial);
            self.partial_rows = 0;
            self.partial_col = 0;
            write!(out, "{}", self.render_line(&(partial + "\n")))?;
        }
        let buffered = self.flush_buffered_table();
        if !buffered.is_empty() {
            write!(out, "{buffered}")?;
        }
        Ok(())
    }

    pub fn run_reader<R: Read, W: Write>(&mut self, input: &mut R, out: &mut W) -> Result<()> {
        let mut pending = Vec::<u8>::new();
        let mut chunk = [0_u8; 4096];

        writeln!(out)?;
        out.flush()?;

        loop {
            let read = input.read(&mut chunk)?;
            if read == 0 {
                break;
            }

            pending.extend_from_slice(&chunk[..read]);

            loop {
                match std::str::from_utf8(&pending) {
                    Ok(text) => {
                        self.write_chunk(text, out)?;
                        pending.clear();
                        break;
                    }
                    Err(error) if error.valid_up_to() > 0 => {
                        let valid = error.valid_up_to();
                        let text = std::str::from_utf8(&pending[..valid])?;
                        self.write_chunk(text, out)?;
                        pending.drain(..valid);
                        if error.error_len().is_none() {
                            break;
                        }
                    }
                    Err(error) if error.error_len().is_none() => break,
                    Err(error) => return Err(error.into()),
                }
            }

            out.flush()?;
        }

        self.finish(out)?;
        out.flush()?;
        Ok(())
    }

    fn clear_list_state(&mut self) {
        self.list_indent_stack.clear();
        self.list_level_meta.clear();
        self.active_list_context = None;
    }

    fn list_depth(&mut self, indent: &str) -> (usize, usize) {
        let width = indent_columns(indent);
        if self.list_indent_stack.is_empty() {
            self.list_indent_stack.push(width);
            return (0, width);
        }

        while self.list_indent_stack.len() > 1 && width < *self.list_indent_stack.last().unwrap() {
            self.list_indent_stack.pop();
        }

        if width < self.list_indent_stack[0] {
            self.list_indent_stack = vec![width];
            return (0, width);
        }

        if width > *self.list_indent_stack.last().unwrap() {
            self.list_indent_stack.push(width);
        }

        (self.list_indent_stack.len().saturating_sub(1).max(0), width)
    }

    fn list_level_prefix(&self, depth: usize) -> String {
        if depth == 0 || !self.show_list_guides {
            return "  ".repeat(depth + 1);
        }
        let guides: String = (0..depth).map(|_| format!("{DIM}│{RESET} ")).collect();
        format!("  {guides}")
    }

    fn set_list_context(
        &mut self,
        rendered_prefix: &str,
        source_indent_width: usize,
        marker_width: usize,
    ) {
        self.active_list_context = Some(ListContext {
            source_indent_width,
            continuation_prefix: format!("{}{}", rendered_prefix, " ".repeat(marker_width + 1)),
        });
    }

    fn remember_list_level(&mut self, depth: usize, kind: ListKind, path: Vec<String>) {
        self.list_level_meta.truncate(depth);
        self.list_level_meta.push(ListLevelMeta { kind, path });
    }

    fn ordered_path(&self, depth: usize, marker: &str) -> Vec<String> {
        let parts: Vec<String> = marker
            .split('.')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_owned())
            .collect();
        if parts.len() > 1 {
            return parts;
        }

        if depth > 0
            && depth <= self.list_level_meta.len()
            && matches!(self.list_level_meta[depth - 1].kind, ListKind::Ordered)
        {
            let mut parent_path = self.list_level_meta[depth - 1].path.clone();
            parent_path.push(parts[0].clone());
            return parent_path;
        }

        parts
    }

    fn render_list_item(
        &mut self,
        prefix: &str,
        indent: &str,
        marker: &str,
        body: &str,
        kind: ListKind,
    ) -> String {
        let (depth, source_indent_width) = self.list_depth(indent);
        let rendered_prefix = format!("{prefix}{}", self.list_level_prefix(depth));

        let (marker_text, marker_width) = match &kind {
            ListKind::Unordered => {
                let bullet = LIST_BULLETS[depth % LIST_BULLETS.len()];
                self.remember_list_level(depth, ListKind::Unordered, Vec::new());
                (bullet.to_owned(), bullet.width())
            }
            ListKind::Ordered => {
                let path = self.ordered_path(depth, marker);
                let (text, width) = format_ordered_marker(&path);
                self.remember_list_level(depth, ListKind::Ordered, path);
                (text, width)
            }
            ListKind::Task => {
                self.remember_list_level(depth, ListKind::Task, Vec::new());
                (marker.to_owned(), visible_width(marker))
            }
        };

        self.set_list_context(&rendered_prefix, source_indent_width, marker_width);
        let rendered_body = format_inline(body, self.inline_code_color);
        format!("{rendered_prefix}{marker_text} {rendered_body}\n")
    }

    fn render_list_continuation(&self, prefix: &str, text: &str) -> Option<String> {
        let ctx = self.active_list_context.as_ref()?;
        if text.trim().is_empty() {
            return None;
        }

        let trimmed = text.trim_start();
        if trimmed.len() == text.len() {
            return None;
        }
        let leading: &str = &text[..text.len() - trimmed.len()];
        let indent_width = indent_columns(leading);
        if indent_width <= ctx.source_indent_width {
            return None;
        }

        let _ = prefix; // continuation_prefix already includes the full prefix
        Some(format!(
            "{}{}\n",
            ctx.continuation_prefix,
            format_inline(trimmed, self.inline_code_color)
        ))
    }

    fn render_noncode_line(&mut self, stripped: &str) -> String {
        if !self.table_lines.is_empty() {
            return self.render_after_active_table(stripped);
        }

        if self.pending_table_header.is_some() {
            return self.render_after_table_candidate(stripped);
        }

        self.render_noncode_line_without_table(stripped)
    }

    fn render_noncode_line_without_table(&mut self, stripped: &str) -> String {
        if let Some(captures) = fence_re().captures(stripped) {
            self.clear_list_state();
            return self.render_code_fence(captures.get(3).map(|m| m.as_str()).unwrap_or_default());
        }

        if looks_like_table_row(stripped) {
            self.clear_list_state();
            self.pending_table_header = Some(stripped.to_owned());
            return String::new();
        }

        self.render_structured_line(stripped)
    }

    fn render_after_table_candidate(&mut self, stripped: &str) -> String {
        let alignments = parse_table_separator(stripped);
        let header_cells = self
            .pending_table_header
            .as_deref()
            .and_then(split_table_row);

        if let (Some(alignments), Some(header_cells)) = (alignments, header_cells)
            && alignments.len() == header_cells.len()
        {
            self.table_lines = vec![
                self.pending_table_header.take().unwrap(),
                stripped.to_owned(),
            ];
            self.table_alignments = alignments;
            return String::new();
        }

        let pending = self.flush_buffered_table();
        let current = self.render_noncode_line(stripped);
        format!("{pending}{current}")
    }

    fn render_after_active_table(&mut self, stripped: &str) -> String {
        let row_cells = split_table_row(stripped);
        let header_cells = self
            .table_lines
            .first()
            .and_then(|line| split_table_row(line));

        if let (Some(row_cells), Some(header_cells)) = (row_cells, header_cells)
            && row_cells.len() == header_cells.len()
            && parse_table_separator(stripped).is_none()
        {
            self.table_lines.push(stripped.to_owned());
            return String::new();
        }

        let table = self.flush_buffered_table();
        let current = self.render_noncode_line(stripped);
        format!("{table}{current}")
    }

    fn render_code_fence(&mut self, rest: &str) -> String {
        if !self.in_code_block {
            self.code_lang = rest
                .split_whitespace()
                .next()
                .unwrap_or_default()
                .to_owned();
            self.in_code_block = true;
            self.code_line_num = 0;
            self.code_highlighter = Some(Self::make_code_highlighter(
                &self.code_lang,
                self.code_theme,
            ));

            if self.code_lang.is_empty() {
                return format!("{}{}{}{}\n", self.pad, DIM, "─".repeat(40), RESET);
            }

            let color = lang_color(&self.code_lang.to_lowercase());
            let label = format!("{RESET} {color}{BOLD}{}{RESET} ", self.code_lang);
            let tail = 38usize
                .saturating_sub(self.code_lang.chars().count() + 2)
                .max(1);
            return format!(
                "{}{}──{}{}{}{}\n",
                self.pad,
                DIM,
                label,
                DIM,
                "─".repeat(tail),
                RESET
            );
        }

        self.in_code_block = false;
        self.code_lang.clear();
        self.code_line_num = 0;
        self.code_highlighter = None;
        format!("{}{}{}{}\n", self.pad, DIM, "─".repeat(40), RESET)
    }

    fn make_code_highlighter(lang: &str, code_theme: CodeTheme) -> HighlightLines<'static> {
        let assets = syntect_assets();
        let syntax = assets
            .syntax_set
            .find_syntax_by_token(normalize_syntax_token(lang))
            .unwrap_or_else(|| assets.syntax_set.find_syntax_plain_text());
        HighlightLines::new(syntax, assets.theme(code_theme))
    }

    fn render_code_line(&mut self, stripped: &str) -> String {
        if let Some(captures) = fence_re().captures(stripped) {
            return self.render_code_fence(captures.get(3).map(|m| m.as_str()).unwrap_or_default());
        }

        self.code_line_num += 1;
        let line = stripped.to_owned() + "\n";
        let highlighted = self
            .code_highlighter
            .as_mut()
            .expect("code highlighter should exist inside fenced blocks")
            .highlight_line(&line, &syntect_assets().syntax_set)
            .expect("syntect highlighting should not fail");

        let escaped = as_24_bit_terminal_escaped(&highlighted, self.show_code_background);
        if self.show_lineno {
            return format!(
                "{}{}{:>3}  {}{}{}\n",
                self.pad,
                DIM,
                self.code_line_num,
                RESET,
                escaped.trim_end_matches('\n'),
                RESET
            );
        }

        format!(
            "{}  {}{}\n",
            self.pad,
            escaped.trim_end_matches('\n'),
            RESET
        )
    }

    fn render_structured_line(&mut self, stripped: &str) -> String {
        let (quote_depth, inner) = split_blockquote(stripped);
        let prefix = block_prefix(&self.pad, quote_depth);

        if quote_depth > 0 && inner.is_empty() {
            return format!("{}\n", prefix.trim_end());
        }

        if quote_depth > 0 {
            return self.render_content_line(inner, &prefix, false);
        }

        self.render_content_line(stripped, &prefix, true)
    }

    fn render_content_line(&mut self, text: &str, prefix: &str, allow_headings: bool) -> String {
        if allow_headings && let Some(captures) = heading_re().captures(text) {
            self.clear_list_state();
            let level = captures.get(1).map(|m| m.as_str().len()).unwrap_or(1);
            let value = captures
                .get(2)
                .map(|m| m.as_str())
                .unwrap_or_default()
                .trim_end_matches('#')
                .trim_end();
            let color = PaletteColor::for_heading_level(level).ansi_escape();
            let leading_gap = if self.previous_was_blank { "" } else { "\n" };
            let heading = format!("{leading_gap}{}{BOLD}{color}{value}{RESET}\n", self.pad);
            return match level {
                1 => format!(
                    "{heading}{}{color}{}{RESET}\n",
                    self.pad,
                    "━".repeat(value.chars().count())
                ),
                2 => format!(
                    "{heading}{}{DIM}{color}{}{RESET}\n",
                    self.pad,
                    "─".repeat(value.chars().count())
                ),
                _ => heading,
            };
        }

        if rule_re().is_match(text) {
            self.clear_list_state();
            return format!("{prefix}{DIM}{}{RESET}\n", "─".repeat(40));
        }

        if let Some(captures) = task_re().captures(text) {
            let indent = captures.get(1).map(|m| m.as_str()).unwrap_or_default();
            let checked = captures
                .get(3)
                .map(|m| m.as_str())
                .unwrap_or_default()
                .eq_ignore_ascii_case("x");
            let body = captures.get(4).map(|m| m.as_str()).unwrap_or_default();
            let checkbox = if checked {
                format!("{BRIGHT_GREEN}{BOLD}✔{RESET}")
            } else {
                format!("{DIM}☐{RESET}")
            };
            return self.render_list_item(prefix, indent, &checkbox, body, ListKind::Task);
        }

        if let Some(captures) = list_re().captures(text) {
            let indent = captures.get(1).map(|m| m.as_str()).unwrap_or_default();
            let body = captures.get(3).map(|m| m.as_str()).unwrap_or_default();
            return self.render_list_item(prefix, indent, "•", body, ListKind::Unordered);
        }

        if let Some(captures) = ordered_re().captures(text) {
            let indent = captures.get(1).map(|m| m.as_str()).unwrap_or_default();
            let number = captures.get(2).map(|m| m.as_str()).unwrap_or_default();
            let body = captures.get(3).map(|m| m.as_str()).unwrap_or_default();
            return self.render_list_item(prefix, indent, number, body, ListKind::Ordered);
        }

        if let Some(cont) = self.render_list_continuation(prefix, text) {
            return cont;
        }

        if text.is_empty() {
            self.clear_list_state();
            return "\n".to_owned();
        }

        self.clear_list_state();
        format!("{prefix}{}\n", format_inline(text, self.inline_code_color))
    }

    fn flush_buffered_table(&mut self) -> String {
        if !self.table_lines.is_empty() {
            let rendered = self.render_table();
            self.table_lines.clear();
            self.table_alignments.clear();
            return rendered;
        }

        if let Some(header) = self.pending_table_header.take() {
            return self.render_structured_line(&header);
        }

        String::new()
    }

    fn render_table(&self) -> String {
        let header = split_table_row(self.table_lines.first().expect("table header should exist"))
            .expect("table header should parse");
        let body_rows: Vec<Vec<String>> = self
            .table_lines
            .iter()
            .skip(2)
            .filter_map(|line| split_table_row(line))
            .collect();
        let mut widths = vec![0usize; header.len()];

        let styled_header: Vec<String> = header
            .iter()
            .map(|cell| {
                format!(
                    "{BOLD}{}{RESET}",
                    format_inline(cell, self.inline_code_color)
                )
            })
            .collect();
        let styled_rows: Vec<Vec<String>> = body_rows
            .iter()
            .map(|row| {
                row.iter()
                    .map(|cell| format_inline(cell, self.inline_code_color))
                    .collect()
            })
            .collect();

        for (idx, cell) in styled_header.iter().enumerate() {
            widths[idx] = widths[idx].max(visible_width(cell));
        }
        for row in &styled_rows {
            for (idx, cell) in row.iter().enumerate() {
                widths[idx] = widths[idx].max(visible_width(cell));
            }
        }

        let mut lines = Vec::new();
        let header_alignments = vec![Alignment::Center; styled_header.len()];
        lines.push(self.render_table_row(&styled_header, &header_alignments, &widths));
        lines.push(self.render_table_separator('━', '┿', false, &widths));
        for (idx, row) in styled_rows.iter().enumerate() {
            lines.push(self.render_table_row(row, &self.table_alignments, &widths));
            if idx < styled_rows.len() - 1 {
                lines.push(self.render_table_separator('─', '┼', false, &widths));
            }
        }
        lines.join("")
    }

    fn render_table_separator(
        &self,
        ch: char,
        joiner: char,
        dim: bool,
        widths: &[usize],
    ) -> String {
        let style = if dim { DIM } else { "" };
        let parts: Vec<String> = widths
            .iter()
            .map(|width| ch.to_string().repeat(width + 2))
            .collect();
        format!(
            "{}{}{}{}\n",
            self.pad,
            style,
            parts.join(&joiner.to_string()),
            RESET
        )
    }

    fn render_table_row(
        &self,
        cells: &[String],
        alignments: &[Alignment],
        widths: &[usize],
    ) -> String {
        let padded: Vec<String> = cells
            .iter()
            .enumerate()
            .map(|(idx, cell)| format!(" {} ", align_cell(cell, widths[idx], alignments[idx])))
            .collect();
        format!("{}{}\n", self.pad, padded.join("│"))
    }

    fn erase_partial<W: Write>(&self, out: &mut W) -> Result<()> {
        if self.partial.is_empty() {
            return Ok(());
        }

        // Use the row count we've been tracking incrementally as bytes were
        // emitted — this is robust against terminal resize because each
        // wrap was decided using the width that was live at the moment
        // each character actually hit the terminal. Querying width here
        // (the old approach) would mis-compute on any mid-stream resize
        // and on hosts where `/dev/tty` reports a width different from the
        // one in effect when the bytes were originally written.
        if self.partial_rows == 0 {
            write!(out, "\r\x1b[K")?;
        } else {
            write!(out, "\x1b[{}A\r\x1b[J", self.partial_rows)?;
        }
        Ok(())
    }

    /// Advance `partial_col` / `partial_rows` to reflect having written
    /// `text` to the terminal at the current `term_width()`. Combining
    /// marks (zero-width) are skipped. A character that doesn't fit in
    /// the remaining columns wraps to the next row before being placed —
    /// matching xterm-style auto-wrap with the cursor advancing past the
    /// glyph's cells. This is called immediately after each `write!` of
    /// raw partial bytes so the snapshot stays in lock-step with what
    /// the terminal actually drew.
    fn advance_partial_position(&mut self, text: &str) {
        let width = self.term_width();
        if width == 0 {
            return;
        }
        for c in text.chars() {
            let w = UnicodeWidthChar::width(c).unwrap_or(0);
            if w == 0 {
                continue;
            }
            if self.partial_col + w > width {
                self.partial_rows += 1;
                self.partial_col = 0;
            }
            self.partial_col += w;
        }
    }

    fn term_width(&self) -> usize {
        if let Some(width) = self.term_width_override {
            return width.max(1);
        }
        // Honor an explicit COLUMNS env var (set by hosts that pipe stdio
        // to a child and want the child to use the host's notion of the
        // terminal width). This takes precedence over `/dev/tty` because
        // the host's measurement reflects the real surface where output
        // will be displayed; the child's `/dev/tty` query may diverge in
        // edge cases (multiplexers, pipes through wrappers).
        if let Some(width) = std::env::var("COLUMNS")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|n| *n > 0)
        {
            return width;
        }
        terminal::size()
            .map(|(columns, _)| usize::from(columns.max(1)))
            .unwrap_or(80)
    }

    #[doc(hidden)]
    pub fn set_term_width_override_for_tests(&mut self, width: usize) {
        self.term_width_override = Some(width);
    }

    #[doc(hidden)]
    pub fn partial_rows_for_tests(&self) -> usize {
        self.partial_rows
    }

    #[doc(hidden)]
    pub fn partial_col_for_tests(&self) -> usize {
        self.partial_col
    }
}

#[cfg(test)]
mod visible_width_tests {
    use super::*;

    #[test]
    fn ascii_string_width_matches_char_count() {
        assert_eq!(visible_width("plain ascii"), 11);
        assert_eq!(visible_width(""), 0);
        assert_eq!(visible_width("a"), 1);
    }

    #[test]
    fn cjk_string_width_is_double_char_count() {
        // Each CJK character has East Asian Width = Wide = 2 cells.
        assert_eq!(visible_width("日本語"), 6);
    }

    #[test]
    fn single_flag_is_two_cells() {
        // Regional indicator pair (U+1F1FA + U+1F1F8 = 🇺🇸) forms a flag
        // emoji. Standards-compliant terminals render it as a single 2-cell
        // glyph; fonts without flag ligatures typically show two narrow
        // tofu boxes that still land at ~2 cells total. Counting 2 matches
        // both rendering modes better than the unicode-width default of 4.
        assert_eq!(visible_width("🇺🇸"), 2);
        assert_eq!(visible_width("🇨🇳"), 2);
        assert_eq!(visible_width("🇯🇵"), 2);
    }

    #[test]
    fn flag_plus_label_width_accounts_for_pair_as_two_cells() {
        assert_eq!(visible_width("🇺🇸 United States"), 16);
        assert_eq!(visible_width("🇨🇳 China"), 8);
        assert_eq!(visible_width("🇬🇧 United Kingdom"), 17);
    }

    #[test]
    fn lone_regional_indicator_is_single_cell() {
        // An unpaired regional indicator is not a flag. Terminals with
        // Nerd Font render it as a single tofu box; we match that.
        assert_eq!(visible_width("\u{1F1FA}"), 1);
    }

    #[test]
    fn ansi_escapes_are_stripped_before_counting() {
        assert_eq!(visible_width("\x1b[1mbold\x1b[0m"), 4);
        assert_eq!(visible_width("\x1b[38;2;255;0;0m🇺🇸\x1b[0m hi"), 5);
    }
}

#[cfg(test)]
mod erase_partial_tests {
    //! Regression coverage for the partial-redraw race that surfaced as a
    //! visible "duplicate paragraph" in scrollback (one wrap row of raw
    //! markdown left behind above the rendered version).
    //!
    //! The pre-fix `erase_partial` queried `terminal::size()` at flush
    //! time and divided the partial's display length by that value. When
    //! the terminal width seen at flush time was wider than the width
    //! that was live when the partial bytes were originally emitted (a
    //! resize, or `/dev/tty` returning a different value than the host's
    //! `process.stdout.columns`), the row count under-estimated and the
    //! cursor-up sequence didn't reach the start of the wrapped region —
    //! `\r\x1b[K` then only cleared the bottom row.
    //!
    //! The fix is to track the partial's row count incrementally as
    //! bytes are written, using the width that was live at each emit.

    use super::*;

    fn extract_erase_seq(out: &[u8]) -> Option<String> {
        // The interesting prefix of a flush is the cursor-up + clear.
        // Either `\r\x1b[K` (single-row partial) or `\x1b[<n>A\r\x1b[J`
        // (multi-row). Find whichever appears first in the stream.
        let s = String::from_utf8_lossy(out);
        // Search for either prefix.
        let candidates = [("\r\x1b[K", 4usize)];
        for (needle, len) in candidates {
            if let Some(idx) = s.find(needle) {
                return Some(s[idx..idx + len].to_string());
            }
        }
        // Multi-row form `\x1b[<n>A\r\x1b[J`.
        let mut i = 0;
        let bytes = s.as_bytes();
        while i + 2 < bytes.len() {
            if bytes[i] == 0x1b && bytes[i + 1] == b'[' {
                let mut j = i + 2;
                while j < bytes.len() && bytes[j].is_ascii_digit() {
                    j += 1;
                }
                if j < bytes.len() && bytes[j] == b'A' {
                    // Look for trailing `\r\x1b[J`.
                    let tail_start = j + 1;
                    let tail = "\r\x1b[J".as_bytes();
                    if bytes.len() >= tail_start + tail.len()
                        && &bytes[tail_start..tail_start + tail.len()] == tail
                    {
                        let end = tail_start + tail.len();
                        return Some(String::from_utf8_lossy(&bytes[i..end]).into_owned());
                    }
                }
            }
            i += 1;
        }
        None
    }

    #[test]
    fn partial_row_count_matches_width_when_paragraph_wraps() {
        // 135-cell wide pane, 170 char ASCII partial → wraps to 2 rows.
        let mut r = StreamingMarkdownRenderer::new(2, false, true);
        r.set_term_width_override_for_tests(135);

        let mut out = Vec::<u8>::new();
        // Stream the partial in two chunks (no \n yet).
        let part_a: String = "a".repeat(100);
        let part_b: String = "b".repeat(70);
        r.write_chunk(&part_a, &mut out).unwrap();
        r.write_chunk(&part_b, &mut out).unwrap();

        // pad (2) + 100 a + 70 b = 172 cells. At width 135: ceil(172/135) = 2 rows.
        assert_eq!(
            r.partial_rows_for_tests(),
            1,
            "should be 1 row above bottom"
        );
        assert!(r.partial_col_for_tests() < 135);
    }

    #[test]
    fn partial_redraw_steps_up_one_row_for_two_row_partial() {
        let mut r = StreamingMarkdownRenderer::new(2, false, true);
        r.set_term_width_override_for_tests(135);

        let mut out = Vec::<u8>::new();
        // Build a 170-char partial that wraps to 2 rows on a 135-col term.
        r.write_chunk(&"x".repeat(170), &mut out).unwrap();
        out.clear();

        // Now feed a `\n` — triggers erase_partial.
        r.write_chunk("\n", &mut out).unwrap();
        let seq = extract_erase_seq(&out).expect("erase sequence emitted");
        assert_eq!(seq, "\x1b[1A\r\x1b[J", "must step up to start of wrap");
    }

    #[test]
    fn resize_after_emit_does_not_break_redraw() {
        // Emit at width 135 (wraps to 2 rows), then "resize" wider before
        // the \n arrives. Pre-fix: erase_partial would query the current
        // (wider) width, see display_len < width, emit just `\r\x1b[K`,
        // and leave the top wrap row stranded. Post-fix: row count was
        // captured at emit time, so the up-step is preserved.
        let mut r = StreamingMarkdownRenderer::new(2, false, true);
        r.set_term_width_override_for_tests(135);

        let mut out = Vec::<u8>::new();
        r.write_chunk(&"x".repeat(170), &mut out).unwrap();

        // Simulate a resize to a much wider terminal.
        r.set_term_width_override_for_tests(300);
        out.clear();

        r.write_chunk("\n", &mut out).unwrap();
        let seq = extract_erase_seq(&out).expect("erase sequence emitted");
        assert_eq!(
            seq, "\x1b[1A\r\x1b[J",
            "row count must reflect width-at-emit, not width-at-flush"
        );
    }

    #[test]
    fn single_row_partial_uses_cr_clear() {
        let mut r = StreamingMarkdownRenderer::new(2, false, true);
        r.set_term_width_override_for_tests(135);

        let mut out = Vec::<u8>::new();
        r.write_chunk("hello world", &mut out).unwrap();
        out.clear();

        r.write_chunk("\n", &mut out).unwrap();
        let seq = extract_erase_seq(&out).expect("erase sequence emitted");
        assert_eq!(seq, "\r\x1b[K");
    }

    #[test]
    fn three_row_partial_steps_up_two() {
        let mut r = StreamingMarkdownRenderer::new(2, false, true);
        r.set_term_width_override_for_tests(50);

        let mut out = Vec::<u8>::new();
        // pad(2) + 130 chars = 132 cells. At width 50: ceil(132/50) = 3 rows.
        r.write_chunk(&"x".repeat(130), &mut out).unwrap();
        assert_eq!(r.partial_rows_for_tests(), 2);
        out.clear();

        r.write_chunk("\n", &mut out).unwrap();
        let seq = extract_erase_seq(&out).expect("erase sequence emitted");
        assert_eq!(seq, "\x1b[2A\r\x1b[J");
    }

    #[test]
    fn columns_env_var_is_honored_when_no_override() {
        // term_width_override takes precedence; clear it and verify
        // COLUMNS is consulted before falling back to /dev/tty / 80.
        let mut r = StreamingMarkdownRenderer::new(0, false, true);
        // SAFETY: tests in this module are not executed in parallel with
        // other tests touching the COLUMNS env var.
        // SAFETY: tests run single-threaded under `cargo test -- --test-threads=1`
        // for this module; in parallel runs, the env var is restored
        // before the test exits and other tests use the override path.
        unsafe { std::env::set_var("COLUMNS", "200") };

        let mut out = Vec::<u8>::new();
        // 150 chars at COLUMNS=200 fits in 1 row (no wrap).
        r.write_chunk(&"x".repeat(150), &mut out).unwrap();
        assert_eq!(r.partial_rows_for_tests(), 0);

        unsafe { std::env::remove_var("COLUMNS") };
    }

    #[test]
    fn paragraph_break_resets_partial_position() {
        let mut r = StreamingMarkdownRenderer::new(2, false, true);
        r.set_term_width_override_for_tests(50);

        let mut out = Vec::<u8>::new();
        r.write_chunk(&"x".repeat(130), &mut out).unwrap();
        assert_eq!(r.partial_rows_for_tests(), 2);

        // \n flushes the partial. New partial is empty → counters reset.
        r.write_chunk("\n", &mut out).unwrap();
        assert_eq!(r.partial_rows_for_tests(), 0);
        assert_eq!(r.partial_col_for_tests(), 0);
    }
}
