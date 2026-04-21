pub mod cli;
pub mod help;
pub mod renderer;
pub mod theme;

use anyhow::Result;
use std::io::{self, IsTerminal, Write};
use theme::DEFAULT_SHOW_CODE_BACKGROUND;

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

    let show_code_background = if cli.code_background {
        true
    } else if cli.no_code_background {
        false
    } else {
        DEFAULT_SHOW_CODE_BACKGROUND
    };

    let mut renderer = renderer::StreamingMarkdownRenderer::with_code_theme_and_inline_code_color(
        cli.padding,
        !cli.no_lineno,
        !cli.no_list_guides,
        cli.theme,
        show_code_background,
        cli.inline_code_color,
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
    write!(stderr, "{}", crate::help::tty_banner())?;
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
        assert!(banner.contains("mdstream"));
        assert!(banner.contains("Streaming Markdown renderer for terminals"));
        assert!(banner.contains("mdstream < input.md"));
        assert!(banner.contains("mdstream --help"));
        assert!(!banner.contains("llm"));
    }
}
