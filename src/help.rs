use crate::renderer::StreamingMarkdownRenderer;
use crate::theme::{
    CodeTheme, DEFAULT_CODE_THEME, DEFAULT_INLINE_CODE_COLOR, DEFAULT_SHOW_CODE_BACKGROUND,
    PaletteColor,
};
use anyhow::Result;
use std::ffi::OsStr;
use std::io::Write;

pub const APP_NAME: &str = env!("CARGO_PKG_NAME");
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const APP_DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");
pub const LICENSE_NAME: &str = "MIT";
pub const COPYRIGHT_YEAR: &str = "2026";
pub const CREATOR_NAME: &str = "Gaston Morixe";
pub const CREATOR_EMAIL: &str = "gaston@gastonmorixe.com";
pub const REPOSITORY_URL: &str = env!("CARGO_PKG_REPOSITORY");

pub fn wants_help_flag_from<I, S>(args: I) -> bool
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut args = args.into_iter();
    let _ = args.next();

    args.any(|arg| {
        let value = arg.as_ref().to_string_lossy();
        value == "--help"
            || value == "-h"
            || (value.starts_with('-')
                && !value.starts_with("--")
                && value[1..].chars().all(|ch| ch.is_ascii_alphabetic())
                && value[1..].contains('h'))
    })
}

pub fn write_rendered_help<W: Write>(out: &mut W) -> Result<()> {
    out.write_all(render_help_text().as_bytes())?;
    out.flush()?;
    Ok(())
}

pub fn tty_banner() -> String {
    render_markdown(&tty_banner_markdown())
}

fn render_help_text() -> String {
    render_markdown(&help_markdown())
}

fn help_markdown() -> String {
    format!(
        "\
# {APP_NAME}

> {APP_DESCRIPTION}
> Version `{APP_VERSION}`

## Usage

```bash
{APP_NAME} < input.md
curl -sL https://raw.githubusercontent.com/gastonmorixe/mdstream/main/README.md | {APP_NAME}
cat README.md | {APP_NAME} --theme catppuccin-mocha
```

## Flags

- `-h`, `--help`: Show this help screen.
- `-V`, `--version`: Show the current version.
- `--padding <SPACES>`: Left padding in spaces. Default: `0`.
- `--no-lineno`: Disable fenced code line numbers.
- `--no-list-guides`: Disable vertical indent guides for nested lists.
- `--theme <THEME>`: Fenced code theme. Default: `{default_theme}`.
- `--inline-code-color <COLOR>`: Inline code accent from the heading palette. Default: `{default_inline_code_color}`.
- `--code-background`: Enable themed fenced code block backgrounds.
- `--no-code-background`: Disable themed fenced code block backgrounds.

## Environment

- `MDSTREAM_PADDING`: Left padding in spaces.
- `MDSTREAM_NO_LINENO`: Disable fenced code line numbers.
- `MDSTREAM_NO_LIST_GUIDES`: Disable vertical indent guides for nested lists.
- `MDSTREAM_THEME`: Fenced code theme.
- `MDSTREAM_INLINE_CODE_COLOR`: Inline code accent color.
- `MDSTREAM_CODE_BACKGROUND`: Enable themed fenced code block backgrounds.
- `MDSTREAM_NO_CODE_BACKGROUND`: Disable themed fenced code block backgrounds.

## Theme values

`{theme_values}`

## Inline palette values

`{palette_values}`

## Project

- License: `{LICENSE_NAME}`
- Creator: {CREATOR_NAME} `<{CREATOR_EMAIL}>`
- Copyright: `{COPYRIGHT_YEAR} {CREATOR_NAME}`
- Repository: `{REPOSITORY_URL}`
",
        default_theme = DEFAULT_CODE_THEME.cli_name(),
        default_inline_code_color = DEFAULT_INLINE_CODE_COLOR.cli_name(),
        theme_values = CodeTheme::all_names().join("`, `"),
        palette_values = PaletteColor::all_names().join("`, `"),
    )
}

fn tty_banner_markdown() -> String {
    format!(
        "\
# {APP_NAME}

{APP_DESCRIPTION}

## Usage

`{APP_NAME} < input.md`
`curl -sL https://raw.githubusercontent.com/gastonmorixe/mdstream/main/README.md | {APP_NAME}`

Run `{APP_NAME} --help` for flags, themes, palette values, examples, and project details.
"
    )
}

fn render_markdown(markdown: &str) -> String {
    let mut renderer = StreamingMarkdownRenderer::with_code_theme_and_inline_code_color(
        0,
        false,
        false,
        DEFAULT_CODE_THEME,
        DEFAULT_SHOW_CODE_BACKGROUND,
        DEFAULT_INLINE_CODE_COLOR,
    );
    let mut output = String::new();

    for line in markdown.lines() {
        output.push_str(&renderer.render_line(&format!("{line}\n")));
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_flag_detection_matches_short_and_long_forms() {
        assert!(wants_help_flag_from(["mdstream", "--help"]));
        assert!(wants_help_flag_from(["mdstream", "-h"]));
        assert!(wants_help_flag_from(["mdstream", "-vh"]));
        assert!(!wants_help_flag_from(["mdstream", "--theme", "mdstream"]));
    }

    #[test]
    fn help_markdown_contains_required_project_metadata() {
        let help = help_markdown();

        assert!(help.contains("License: `MIT`"));
        assert!(help.contains(CREATOR_NAME));
        assert!(help.contains(CREATOR_EMAIL));
        assert!(help.contains(REPOSITORY_URL));
        assert!(help.contains(COPYRIGHT_YEAR));
    }
}
