use clap::Parser;

use crate::theme::{CodeTheme, DEFAULT_CODE_THEME, DEFAULT_INLINE_CODE_COLOR, PaletteColor};

#[derive(Debug, Parser)]
#[command(
    name = "mdstream",
    version,
    about = "Streaming Markdown renderer for terminals"
)]
pub struct Cli {
    #[arg(long, hide = true, default_value_t = false)]
    pub highlight_server: bool,

    #[arg(
        long,
        env = "MDSTREAM_PADDING",
        default_value_t = 0,
        value_name = "SPACES",
        help_heading = "Display",
        help = "Left padding in spaces."
    )]
    pub padding: usize,

    #[arg(
        long,
        env = "MDSTREAM_NO_LINENO",
        default_value_t = false,
        help_heading = "Display",
        help = "Disable fenced-code line numbers."
    )]
    pub no_lineno: bool,

    #[arg(
        long,
        env = "MDSTREAM_NO_LIST_GUIDES",
        default_value_t = false,
        help_heading = "Display",
        help = "Disable vertical indent guides for nested lists."
    )]
    pub no_list_guides: bool,

    #[arg(
        long,
        env = "MDSTREAM_THEME",
        default_value_t = DEFAULT_CODE_THEME,
        help_heading = "Code highlighting",
        help = "Code theme for fenced code blocks."
    )]
    pub theme: CodeTheme,

    #[arg(
        long,
        env = "MDSTREAM_INLINE_CODE_COLOR",
        default_value_t = DEFAULT_INLINE_CODE_COLOR,
        value_name = "COLOR",
        help_heading = "Code highlighting",
        help = "Inline code accent color from the h1-h6 palette."
    )]
    pub inline_code_color: PaletteColor,

    #[arg(
        long,
        env = "MDSTREAM_CODE_BACKGROUND",
        default_value_t = false,
        conflicts_with = "no_code_background",
        help_heading = "Code highlighting",
        help = "Enable themed backgrounds in fenced code blocks."
    )]
    pub code_background: bool,

    #[arg(
        long,
        env = "MDSTREAM_NO_CODE_BACKGROUND",
        default_value_t = false,
        conflicts_with = "code_background",
        help_heading = "Code highlighting",
        help = "Disable themed backgrounds in fenced code blocks."
    )]
    pub no_code_background: bool,

    #[arg(
        long,
        env = "MDSTREAM_TABLE_FIT",
        default_value_t = false,
        help_heading = "Tables",
        help = "Cap tables at the live terminal width; soft-wrap cells only when the natural table would overflow (auto-disables when not a TTY)."
    )]
    pub table_fit: bool,

    #[arg(
        long,
        env = "MDSTREAM_TABLE_WIDTH_OFFSET",
        default_value_t = 0,
        allow_negative_numbers = true,
        value_name = "CELLS",
        help_heading = "Tables",
        help = "Cells to add (+) or subtract (-) from the table-fit cap width; e.g. -10 leaves a 10-cell right gutter."
    )]
    pub table_width_offset: i32,

    #[arg(
        long,
        env = "MDSTREAM_WRAP",
        num_args = 0..=1,
        default_missing_value = "auto",
        value_name = "auto|COLS",
        help_heading = "Text",
        help = "Wrap long text lines. No value uses the terminal width. COLS (10-1000) sets a fixed width."
    )]
    pub wrap: Option<WrapMode>,

    #[arg(
        long,
        env = "MDSTREAM_WRAP_WIDTH_OFFSET",
        default_value_t = 0,
        allow_negative_numbers = true,
        value_name = "CELLS",
        help_heading = "Text",
        help = "Cells to add (+) or remove (-) from the terminal width when --wrap has no value."
    )]
    pub wrap_width_offset: i32,
}

/// Smallest fixed wrap width that `--wrap COLS` accepts.
pub const WRAP_MIN_COLS: usize = 10;
/// Largest fixed wrap width that `--wrap COLS` accepts.
pub const WRAP_MAX_COLS: usize = 1000;

/// How `--wrap` gets its width.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WrapMode {
    /// Use the live terminal width plus `--wrap-width-offset`.
    Auto,
    /// Use this width. It replaces the terminal width.
    Fixed(usize),
}

impl std::str::FromStr for WrapMode {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let value = value.trim();
        if value.eq_ignore_ascii_case("auto") {
            return Ok(WrapMode::Auto);
        }
        let invalid = || {
            format!(
                "invalid wrap width '{value}': use 'auto' or an integer from {WRAP_MIN_COLS} to {WRAP_MAX_COLS}"
            )
        };
        let cols = value.parse::<usize>().map_err(|_| invalid())?;
        if (WRAP_MIN_COLS..=WRAP_MAX_COLS).contains(&cols) {
            Ok(WrapMode::Fixed(cols))
        } else {
            Err(invalid())
        }
    }
}

#[cfg(test)]
mod wrap_mode_tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn parses_auto_and_fixed() {
        assert_eq!("auto".parse::<WrapMode>(), Ok(WrapMode::Auto));
        assert_eq!("80".parse::<WrapMode>(), Ok(WrapMode::Fixed(80)));
        assert_eq!("10".parse::<WrapMode>(), Ok(WrapMode::Fixed(10)));
        assert_eq!("1000".parse::<WrapMode>(), Ok(WrapMode::Fixed(1000)));
    }

    #[test]
    fn rejects_out_of_range_and_garbage() {
        for bad in ["9", "1001", "0", "-5", "", "abc"] {
            let err = bad.parse::<WrapMode>().expect_err(bad);
            assert!(err.contains("auto"), "{err}");
            assert!(err.contains("10 to 1000"), "{err}");
        }
    }

    fn parse(args: &[&str]) -> Cli {
        let mut argv = vec!["mdstream"];
        argv.extend_from_slice(args);
        Cli::try_parse_from(argv).expect("args should parse")
    }

    #[test]
    fn flag_forms() {
        // Env vars may leak in from the developer shell; these cases only
        // assert command-line forms, which always win over env.
        assert_eq!(parse(&["--wrap"]).wrap, Some(WrapMode::Auto));
        assert_eq!(parse(&["--wrap", "auto"]).wrap, Some(WrapMode::Auto));
        assert_eq!(parse(&["--wrap", "80"]).wrap, Some(WrapMode::Fixed(80)));
        assert_eq!(parse(&["--wrap=80"]).wrap, Some(WrapMode::Fixed(80)));
        let cli = parse(&["--wrap", "--padding", "2"]);
        assert_eq!(cli.wrap, Some(WrapMode::Auto));
        assert_eq!(cli.padding, 2);
        assert_eq!(
            parse(&["--wrap-width-offset", "-10"]).wrap_width_offset,
            -10
        );
    }

    #[test]
    fn rejects_small_fixed_width() {
        let err = Cli::try_parse_from(["mdstream", "--wrap", "5"]).expect_err("5 is too small");
        assert_eq!(err.exit_code(), 2);
    }
}
