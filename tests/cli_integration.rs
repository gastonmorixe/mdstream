use std::io::Write;
use std::process::{Command, Stdio};

fn strip_ansi(text: &str) -> String {
    let re = regex::Regex::new(r"\x1b\[[0-9;?]*[ -/]*[@-~]").unwrap();
    re.replace_all(text, "").replace('\r', "")
}

#[test]
fn binary_rejects_invalid_padding_env() {
    // Regression guard. The CLI surface goes through clap, which parses the
    // `MDSTREAM_PADDING` env var via `usize::FromStr`. When the value is not
    // a valid integer, clap exits non-zero with a parser error — matching
    // Python's `int()` raise at mdstream.py:356.
    //
    // The library convenience constructor `StreamingMarkdownRenderer::from_env`
    // is intentionally lenient (silent fallback to 0) for embedding use cases
    // and is NOT exercised here.
    let output = Command::new(env!("CARGO_BIN_EXE_mdstream"))
        .env("MDSTREAM_PADDING", "abc")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "expected non-zero exit on bad MDSTREAM_PADDING, stdout={:?}, stderr={:?}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.to_lowercase().contains("padding") || stderr.contains("MDSTREAM_PADDING"),
        "expected stderr to mention padding, got: {stderr}"
    );
}

#[test]
fn binary_respects_padding_flag() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_mdstream"))
        .arg("--padding")
        .arg("0")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"# Title\n")
        .unwrap();
    let output = child.wait_with_output().unwrap();
    let stdout = strip_ansi(&String::from_utf8(output.stdout).unwrap());

    assert!(stdout.contains("Title\n━━━━━\n"));
}

#[test]
fn binary_respects_no_lineno_env() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_mdstream"))
        .env("MDSTREAM_PADDING", "0")
        .env("MDSTREAM_NO_LINENO", "true")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"```python\nprint('hello')\n```\n")
        .unwrap();
    let output = child.wait_with_output().unwrap();
    let stdout = strip_ansi(&String::from_utf8(output.stdout).unwrap());

    assert!(!stdout.contains("  1  "));
    assert!(stdout.contains("print('hello')"));
}

#[test]
fn help_lists_theme_flag_and_values() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdstream"))
        .arg("--help")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = strip_ansi(&String::from_utf8(output.stdout).unwrap());
    let stderr = String::from_utf8(output.stderr).unwrap();

    assert!(stderr.is_empty());
    assert!(stdout.contains("Streaming Markdown renderer for terminals"));
    assert!(stdout.contains(&format!("Version {}", env!("CARGO_PKG_VERSION"))));
    assert!(stdout.contains("--theme <THEME>"));
    assert!(stdout.contains("--inline-code-color <COLOR>"));
    assert!(stdout.contains("mdstream"));
    assert!(stdout.contains("catppuccin-mocha"));
    assert!(stdout.contains("sublime-snazzy"));
    assert!(stdout.contains("dracula"));
    assert!(stdout.contains("h1"));
    assert!(stdout.contains("h6"));
    assert!(stdout.contains("--code-background"));
    assert!(stdout.contains("solarized-dark"));
    assert!(stdout.contains("--no-code-background"));
    assert!(stdout.contains("License: MIT"));
    assert!(stdout.contains("Creator: Gaston Morixe <gaston@gastonmorixe.com>"));
    assert!(stdout.contains("Repository: https://github.com/gastonmorixe/mdstream"));
    assert!(!stdout.to_lowercase().contains("llm"));
}

#[test]
fn binary_rejects_invalid_theme_value_and_lists_choices() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdstream"))
        .arg("--theme")
        .arg("nope")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap();

    assert!(!output.status.success());

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("invalid value"));
    assert!(stderr.contains("possible values"));
    assert!(stderr.contains("mdstream"));
    assert!(stderr.contains("catppuccin-mocha"));
    assert!(stderr.contains("inspired-github"));
}

#[test]
fn binary_defaults_to_foreground_only_mdstream_theme() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_mdstream"))
        .env("MDSTREAM_PADDING", "0")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"```python\nif value == 1:\n```\n")
        .unwrap();
    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(!stdout.contains("\x1b[48;2;"));
    assert!(stdout.contains("\x1b[38;2;255;97;172m"));
}

#[test]
fn binary_respects_inline_code_color_flag() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_mdstream"))
        .arg("--inline-code-color")
        .arg("h3")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"`code`\n")
        .unwrap();
    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(!stdout.contains("\x1b[48;"));
    assert!(stdout.contains("\x1b[38;2;100;220;100m"));
    assert_eq!(strip_ansi(&stdout), "\ncode\n");
}

/// Cross-reference guard. Every long flag and `env =` declared on
/// `mdstream::cli::Cli` MUST appear in the binary's rendered `--help`
/// output. The hand-curated help screen in `src/help.rs` is now
/// programmatically generated from the clap derive — this test makes
/// sure that pipeline never silently drops anything (e.g. by a flag
/// landing under a `help_heading` that isn't listed in `SECTION_ORDER`
/// — those still surface under `Other`, but if someone ever rewires
/// the renderer to filter sections, this test fails loudly).
#[test]
fn help_screen_lists_every_clap_flag_and_env() {
    use clap::CommandFactory;

    let cmd = mdstream::cli::Cli::command();
    let output = Command::new(env!("CARGO_BIN_EXE_mdstream"))
        .arg("--help")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = strip_ansi(&String::from_utf8(output.stdout).unwrap());

    let mut missing_flags: Vec<String> = Vec::new();
    let mut missing_envs: Vec<String> = Vec::new();
    for arg in cmd.get_arguments() {
        if let Some(long) = arg.get_long() {
            if matches!(long, "help" | "version") {
                continue;
            }
            let needle = format!("--{long}");
            if !stdout.contains(&needle) {
                missing_flags.push(needle);
            }
        }
        if let Some(env) = arg.get_env() {
            let env = env.to_string_lossy().into_owned();
            if !stdout.contains(&env) {
                missing_envs.push(env);
            }
        }
    }

    assert!(
        missing_flags.is_empty() && missing_envs.is_empty(),
        "src/help.rs is out of sync with src/cli.rs.\n  \
         Flags missing from --help: {missing_flags:?}\n  \
         Env vars missing from --help: {missing_envs:?}"
    );
}
