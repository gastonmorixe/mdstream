use clap::Parser;

use crate::theme::{CodeTheme, DEFAULT_CODE_THEME, DEFAULT_INLINE_CODE_COLOR, PaletteColor};

#[derive(Debug, Parser)]
#[command(
    name = "mdstream",
    version,
    about = "Streaming Markdown renderer for terminals"
)]
pub struct Cli {
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
}
