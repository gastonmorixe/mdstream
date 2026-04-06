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
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("--theme <THEME>"));
    assert!(stdout.contains("base16-ocean-dark"));
    assert!(stdout.contains("solarized-dark"));
    assert!(stdout.contains("--no-code-background"));
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
    assert!(stderr.contains("base16-ocean-dark"));
    assert!(stderr.contains("inspired-github"));
}
