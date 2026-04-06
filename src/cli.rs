use clap::Parser;

#[derive(Debug, Parser)]
#[command(
    name = "mdstream",
    version,
    about = "Hybrid streaming Markdown renderer for terminal output"
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
}
