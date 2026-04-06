use clap::Parser;

use crate::theme::{CodeTheme, DEFAULT_CODE_THEME};

#[derive(Debug, Parser)]
#[command(
    name = "mdstream",
    version,
    about = "Hybrid streaming Markdown renderer for terminal output",
    after_help = "License: MIT  ·  Copyright (c) 2026 Gaston Morixe <gaston@gastonmorixe.com>\nHomepage: https://github.com/gastonmorixe/mdstream"
)]
pub struct Cli {
    #[arg(
        long,
        env = "MDSTREAM_PADDING",
        default_value_t = 0,
        help = "Left padding in spaces"
    )]
    pub padding: usize,

    #[arg(
        long,
        env = "MDSTREAM_NO_LINENO",
        default_value_t = false,
        help = "Disable fenced-code line numbers"
    )]
    pub no_lineno: bool,

    #[arg(
        long,
        env = "MDSTREAM_NO_LIST_GUIDES",
        default_value_t = false,
        help = "Disable vertical indent guides for nested lists"
    )]
    pub no_list_guides: bool,

    #[arg(
        long,
        env = "MDSTREAM_THEME",
        default_value_t = DEFAULT_CODE_THEME,
        help = "Syntect theme for fenced code blocks"
    )]
    pub theme: CodeTheme,

    #[arg(
        long,
        env = "MDSTREAM_NO_CODE_BACKGROUND",
        default_value_t = false,
        help = "Disable themed backgrounds in fenced code blocks"
    )]
    pub no_code_background: bool,
}
