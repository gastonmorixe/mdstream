pub mod cli;
pub mod renderer;
pub mod theme;

use anyhow::Result;
use std::io::{self, IsTerminal, Write};

/// Sentinel error returned when stdin is attached to a terminal instead of a
/// pipe. The binary entry point catches this and exits non-zero without
/// double-printing the help banner that `handle_tty_check` already wrote.
/// Mirrors Python `mdstream.py:913` (`sys.exit(1)` after the banner).
#[derive(Debug)]
pub struct StdinIsTerminal;

impl std::fmt::Display for StdinIsTerminal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("stdin is a terminal; pipe markdown into mdstream")
    }
}

impl std::error::Error for StdinIsTerminal {}

pub fn run(cli: cli::Cli) -> Result<()> {
    let is_tty = io::stdin().is_terminal();
    let mut stderr = io::stderr().lock();
    handle_tty_check(is_tty, &mut stderr)?;

    let mut renderer = renderer::StreamingMarkdownRenderer::with_code_theme(
        cli.padding,
        !cli.no_lineno,
        !cli.no_list_guides,
        cli.theme,
        !cli.no_code_background,
    );
    let mut stdin = io::stdin().lock();
    let mut stdout = io::stdout().lock();

    renderer.run_reader(&mut stdin, &mut stdout)?;
    stdout.flush()?;
    Ok(())
}

/// Writes the usage banner to `stderr` and returns `StdinIsTerminal` when
/// `is_tty` is true. Returns `Ok(())` otherwise. Split out so the TTY branch
/// is testable without spawning a real PTY.
pub fn handle_tty_check<W: Write>(is_tty: bool, stderr: &mut W) -> Result<()> {
    if !is_tty {
        return Ok(());
    }
    writeln!(stderr, "mdstream — Streaming Markdown Renderer")?;
    writeln!(stderr)?;
    writeln!(
        stderr,
        "  Real-time token streaming with rendered markdown output."
    )?;
    writeln!(
        stderr,
        "  Partial lines stream raw; completed lines render fully."
    )?;
    writeln!(stderr)?;
    writeln!(stderr, "Usage:")?;
    writeln!(stderr, "  mdstream < input.md")?;
    writeln!(stderr, "  llm \"prompt\" | mdstream")?;
    writeln!(stderr)?;
    writeln!(stderr, "Environment:")?;
    writeln!(
        stderr,
        "  MDSTREAM_PADDING         Left padding in spaces (default: 0)"
    )?;
    writeln!(
        stderr,
        "  MDSTREAM_NO_LINENO       Disable line numbers in fenced code blocks"
    )?;
    writeln!(
        stderr,
        "  MDSTREAM_NO_LIST_GUIDES  Disable vertical indent guides for nested lists"
    )?;
    writeln!(
        stderr,
        "  MDSTREAM_THEME           Syntect theme for fenced code blocks"
    )?;
    writeln!(
        stderr,
        "  MDSTREAM_NO_CODE_BACKGROUND  Disable themed code-block backgrounds"
    )?;
    writeln!(stderr)?;
    writeln!(
        stderr,
        "Themes: {}",
        crate::theme::CodeTheme::all_names_csv()
    )?;
    Err(StdinIsTerminal.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn handle_tty_check_returns_ok_and_writes_nothing_when_not_tty() {
        let mut stderr = Cursor::new(Vec::new());
        let result = handle_tty_check(false, &mut stderr);
        assert!(result.is_ok());
        assert!(stderr.into_inner().is_empty());
    }

    #[test]
    fn handle_tty_check_writes_banner_and_returns_stdin_is_terminal_error() {
        let mut stderr = Cursor::new(Vec::new());
        let result = handle_tty_check(true, &mut stderr);
        let err = result.expect_err("expected an error in TTY mode");
        assert!(
            err.is::<StdinIsTerminal>(),
            "expected StdinIsTerminal error, got: {err:?}"
        );

        let banner = String::from_utf8(stderr.into_inner()).unwrap();
        assert!(banner.contains("mdstream — Streaming Markdown Renderer"));
        assert!(banner.contains("MDSTREAM_PADDING"));
        assert!(banner.contains("MDSTREAM_NO_LINENO"));
        assert!(banner.contains("MDSTREAM_NO_LIST_GUIDES"));
        assert!(banner.contains("MDSTREAM_THEME"));
        assert!(banner.contains("MDSTREAM_NO_CODE_BACKGROUND"));
        assert!(banner.contains("base16-ocean-dark"));
    }
}
