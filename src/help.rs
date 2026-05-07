use crate::cli::Cli;
use crate::renderer::StreamingMarkdownRenderer;
use crate::theme::{
    CodeTheme, DEFAULT_CODE_THEME, DEFAULT_INLINE_CODE_COLOR, DEFAULT_SHOW_CODE_BACKGROUND,
    PaletteColor,
};
use anyhow::Result;
use clap::{Arg, ArgAction, CommandFactory};
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

/// Heading order used to emit grouped Flag sections. Anything declared
/// in [`crate::cli::Cli`] without a `help_heading` (or with one not
/// listed here) lands in `Other` at the bottom — so a forgotten
/// grouping shows up obviously instead of vanishing.
const SECTION_ORDER: &[&str] = &["Display", "Code highlighting", "Tables"];

/// Pseudo-heading we synthesize for `--help` / `--version`. clap
/// auto-injects these and they don't carry a `help_heading`; we
/// surface them under "Common" so users find them first.
const COMMON_SECTION: &str = "Common";

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

/// Build the rendered `--help` markdown by introspecting the clap
/// parser. The clap derive in [`crate::cli`] is the single source of
/// truth for flag names, env vars, defaults, and help strings —
/// editing this file should never be required when adding a new flag,
/// only updating the derive (and adding a new entry to
/// [`SECTION_ORDER`] if you introduce a new heading).
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

{flags_section}
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
        flags_section = flags_markdown(),
        theme_values = CodeTheme::all_names().join("`, `"),
        palette_values = PaletteColor::all_names().join("`, `"),
    )
}

/// Render the `## Flags` body as grouped, single-line entries:
///
/// ```text
/// ### Display
///
/// - `--padding <SPACES>`: Left padding in spaces. Default: `0`. Env: `MDSTREAM_PADDING`.
/// ```
///
/// Each entry is built from `clap::Arg` introspection — the long flag
/// name, value name (if any), help text, default value (skipped for
/// boolean / count actions where "false" / "0" is just noise), and
/// the `env =` binding. Sections are ordered per [`SECTION_ORDER`];
/// `Common` (synthesized for `--help` / `--version`) leads.
fn flags_markdown() -> String {
    let cmd = Cli::command();

    // (heading, lines) preserving insertion order. Args declared
    // consecutively in cli.rs with the same `help_heading` end up in
    // the same bucket.
    let mut sections: Vec<(String, Vec<String>)> = vec![(
        COMMON_SECTION.to_string(),
        vec![
            "- `-h`, `--help`: Show this help screen.".to_string(),
            "- `-V`, `--version`: Show the current version.".to_string(),
        ],
    )];

    for arg in cmd.get_arguments() {
        let Some(long) = arg.get_long() else { continue };
        if matches!(long, "help" | "version") {
            // clap auto-injects these — handled in the static
            // `Common` section above.
            continue;
        }
        let line = render_flag_line(arg, long);
        let heading = arg.get_help_heading().unwrap_or("Other").to_string();
        push_to_section(&mut sections, heading, line);
    }

    // Sort sections by SECTION_ORDER (Common first, then explicit
    // order, then `Other` and anything unrecognized last in
    // first-seen order).
    sections.sort_by_key(|(h, _)| section_rank(h));

    let mut out = String::new();
    for (heading, lines) in &sections {
        if lines.is_empty() {
            continue;
        }
        out.push_str(&format!("### {heading}\n\n"));
        for line in lines {
            out.push_str(line);
            out.push('\n');
        }
        out.push('\n');
    }
    out
}

fn render_flag_line(arg: &Arg, long: &str) -> String {
    let value_part = arg
        .get_value_names()
        .and_then(|names| names.first().cloned())
        .filter(|_| takes_value(arg))
        .map(|name| format!(" <{name}>"))
        .unwrap_or_default();

    let help_text = arg.get_help().map(|h| h.to_string()).unwrap_or_default();

    let mut line = format!("- `--{long}{value_part}`: {help_text}");
    if !ends_with_terminal_punct(&line) {
        line.push('.');
    }

    if takes_value(arg)
        && let Some(default) = arg
            .get_default_values()
            .iter()
            .next()
            .map(|v| v.to_string_lossy().into_owned())
        && !default.is_empty()
    {
        line.push_str(&format!(" Default: `{default}`."));
    }

    if let Some(env) = arg.get_env() {
        line.push_str(&format!(" Env: `{}`.", env.to_string_lossy()));
    }

    line
}

/// True when the arg consumes a value on the command line (e.g.
/// `--theme <THEME>`), as opposed to a presence-only boolean flag.
/// Defaults are only meaningful for value-taking args — surfacing
/// `false` next to `--no-lineno` is just visual noise.
fn takes_value(arg: &Arg) -> bool {
    !matches!(
        arg.get_action(),
        ArgAction::SetTrue
            | ArgAction::SetFalse
            | ArgAction::Count
            | ArgAction::Help
            | ArgAction::HelpShort
            | ArgAction::HelpLong
            | ArgAction::Version
    )
}

fn ends_with_terminal_punct(s: &str) -> bool {
    matches!(s.chars().last(), Some('.') | Some('!') | Some('?'))
}

fn section_rank(heading: &str) -> usize {
    if heading == COMMON_SECTION {
        return 0;
    }
    SECTION_ORDER
        .iter()
        .position(|h| *h == heading)
        .map(|p| p + 1)
        .unwrap_or(usize::MAX)
}

fn push_to_section(sections: &mut Vec<(String, Vec<String>)>, heading: String, line: String) {
    if let Some(entry) = sections.iter_mut().find(|(h, _)| h == &heading) {
        entry.1.push(line);
    } else {
        sections.push((heading, vec![line]));
    }
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

    #[test]
    fn help_markdown_lists_every_clap_flag_and_env() {
        // Companion to the binary-spawning integration test in
        // tests/cli_integration.rs. Asserted at the unit level too so
        // failures point at this file directly.
        let help = help_markdown();
        let cmd = Cli::command();
        for arg in cmd.get_arguments() {
            if let Some(long) = arg.get_long() {
                if matches!(long, "help" | "version") {
                    continue;
                }
                let needle = format!("--{long}");
                assert!(
                    help.contains(&needle),
                    "flag `{needle}` missing from help_markdown — \
                     check that its `help_heading` is in SECTION_ORDER \
                     or fall through to `Other`"
                );
            }
            if let Some(env) = arg.get_env() {
                let env = env.to_string_lossy();
                assert!(
                    help.contains(env.as_ref()),
                    "env var `{env}` missing from help_markdown"
                );
            }
        }
    }

    #[test]
    fn help_markdown_groups_table_flags_under_tables_heading() {
        let help = help_markdown();
        let tables_idx = help.find("### Tables").expect("Tables heading missing");
        let table_fit_idx = help.find("--table-fit").expect("--table-fit missing");
        assert!(
            tables_idx < table_fit_idx,
            "--table-fit should appear under the Tables heading"
        );
    }
}
