use std::io::{BufRead, Write};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::diff::{DiffColors, DiffStyle, highlight_unified_diff};
use crate::highlight::{RawCodeHighlighter, bg_escape};
use crate::theme::CodeTheme;

/// Supported request modes, advertised in the ready line so a client never
/// wastes a sacrificial request probing capability. Only modes with a live
/// handler are advertised.
pub const MODES: &[&str] = &["raw", "diff-wash", "unified-diff"];

#[derive(Debug, Deserialize)]
struct HighlightRequest {
    id: u64,
    #[serde(default)]
    mode: Option<String>,
    language: String,
    code: String,
    #[serde(default)]
    wash_per_line: Option<Vec<Option<String>>>,
    #[serde(default)]
    colors: Option<DiffColors>,
    #[serde(default, rename = "diffStyle")]
    diff_style: Option<String>,
}

#[derive(Serialize)]
struct ReadyResponse<'a> {
    ready: u8,
    protocol: u8,
    modes: &'a [&'a str],
}

#[derive(Serialize)]
struct HighlightResponse<'a> {
    id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    ansi: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<&'a str>,
}

/// Runs the raw-code JSONL highlighting protocol until `input` reaches EOF.
///
/// The first line written is `{"ready":1,"protocol":2,"modes":[...]}` after
/// syntect assets have loaded. Each later input line must contain `id`,
/// `language`, and `code`. Requests are handled sequentially and produce
/// exactly one JSON response line. Absent `mode` (or `mode:"raw"`) is
/// byte-identical to protocol v1.
pub fn run_highlight_server<R: BufRead, W: Write>(
    input: R,
    mut output: W,
    code_theme: CodeTheme,
    show_background: bool,
) -> Result<()> {
    let highlighter = RawCodeHighlighter::new(code_theme, show_background);
    write_json_line(
        &mut output,
        &ReadyResponse {
            ready: 1,
            protocol: 2,
            modes: MODES,
        },
    )?;

    for line in input.lines() {
        let line = line?;
        handle_line(
            &highlighter,
            code_theme,
            show_background,
            &line,
            &mut output,
        )?;
    }

    Ok(())
}

fn handle_line<W: Write>(
    highlighter: &RawCodeHighlighter,
    code_theme: CodeTheme,
    show_background: bool,
    line: &str,
    output: &mut W,
) -> Result<()> {
    let request = match serde_json::from_str::<HighlightRequest>(line) {
        Ok(request) => request,
        Err(error) => {
            let id = parse_lenient_id(line);
            return write_error(output, id, error.to_string());
        }
    };

    match request.mode.as_deref() {
        None | Some("raw") => write_raw(highlighter, &request, output),
        Some("diff-wash") => write_diff_wash(highlighter, &request, output),
        Some("unified-diff") => write_unified_diff(&request, code_theme, show_background, output),
        Some(other) => write_error(output, request.id, format!("unknown mode: {other}")),
    }
}

fn write_raw<W: Write>(
    highlighter: &RawCodeHighlighter,
    request: &HighlightRequest,
    output: &mut W,
) -> Result<()> {
    match highlighter.highlight(&request.language, &request.code) {
        Ok(ansi) => write_json_line(
            output,
            &HighlightResponse {
                id: request.id,
                ansi: Some(&ansi),
                error: None,
            },
        ),
        Err(error) => write_error(output, request.id, error.to_string()),
    }
}

fn write_diff_wash<W: Write>(
    highlighter: &RawCodeHighlighter,
    request: &HighlightRequest,
    output: &mut W,
) -> Result<()> {
    let washes = match request.wash_per_line.as_deref() {
        Some(washes) => {
            let count = logical_line_count(&request.code);
            if washes.len() != count {
                return write_error(
                    output,
                    request.id,
                    format!(
                        "wash_per_line length {} does not match code line count {count}",
                        washes.len()
                    ),
                );
            }
            let parsed: Result<Vec<Option<String>>, String> = washes
                .iter()
                .map(|w| match w {
                    None => Ok(None),
                    Some(hex) => bg_escape(hex)
                        .map(Some)
                        .ok_or_else(|| format!("invalid wash color: {hex}")),
                })
                .collect();
            match parsed {
                Ok(parsed) => parsed,
                Err(message) => return write_error(output, request.id, message),
            }
        }
        None => {
            return write_error(
                output,
                request.id,
                "diff-wash mode requires wash_per_line (array of hex or null)".to_string(),
            );
        }
    };

    match highlighter.highlight_with_washes(&request.language, &request.code, Some(&washes)) {
        Ok(ansi) => write_json_line(
            output,
            &HighlightResponse {
                id: request.id,
                ansi: Some(&ansi),
                error: None,
            },
        ),
        Err(error) => write_error(output, request.id, error.to_string()),
    }
}

/// Phase 2: unified-diff composition. Reuses the dual-stream engine in
/// `crate::diff`. Fails closed on invalid colors / unknown diffStyle.
fn write_unified_diff<W: Write>(
    request: &HighlightRequest,
    code_theme: CodeTheme,
    show_background: bool,
    output: &mut W,
) -> Result<()> {
    let style = match request.diff_style.as_deref() {
        None | Some("marker-fg") => DiffStyle::MarkerFg,
        Some("bg-wash") => DiffStyle::BgWash,
        Some(other) => {
            return write_error(output, request.id, format!("unknown diffStyle: {other}"));
        }
    };

    let colors = match request.colors.as_ref() {
        Some(c) => c.clone(),
        None => DiffColors::default(),
    };

    match highlight_unified_diff(
        &request.language,
        &request.code,
        style,
        &colors,
        code_theme,
        show_background,
    ) {
        Ok(ansi) => write_json_line(
            output,
            &HighlightResponse {
                id: request.id,
                ansi: Some(&ansi),
                error: None,
            },
        ),
        Err(error) => write_error(output, request.id, error.to_string()),
    }
}

fn write_error<W: Write>(output: &mut W, id: u64, message: String) -> Result<()> {
    write_json_line(
        output,
        &HighlightResponse {
            id,
            ansi: None,
            error: Some(&message),
        },
    )
}

fn logical_line_count(code: &str) -> usize {
    if code.is_empty() {
        0
    } else {
        code.lines().count()
    }
}

fn parse_lenient_id(line: &str) -> u64 {
    serde_json::from_str::<serde_json::Value>(line)
        .ok()
        .and_then(|value| value.get("id").and_then(serde_json::Value::as_u64))
        .unwrap_or(0)
}

fn write_json_line<W: Write, T: Serialize>(output: &mut W, value: &T) -> Result<()> {
    serde_json::to_writer(&mut *output, value)?;
    output.write_all(b"\n")?;
    output.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use serde_json::Value;

    use super::*;

    fn run_once(input_bytes: &[u8]) -> Vec<Value> {
        let input = Cursor::new(input_bytes.to_vec());
        let mut output = Vec::new();
        run_highlight_server(input, &mut output, CodeTheme::Mdstream, false).unwrap();
        String::from_utf8(output)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str::<Value>(line).unwrap())
            .collect()
    }

    #[test]
    fn emits_ready_with_protocol_and_modes() {
        let lines = run_once(b"");
        assert_eq!(lines[0]["ready"], 1);
        assert_eq!(lines[0]["protocol"], 2);
        assert_eq!(
            lines[0]["modes"],
            serde_json::json!(["raw", "diff-wash", "unified-diff"])
        );
    }

    #[test]
    fn raw_mode_is_byte_identical_to_v1() {
        let lines = run_once(
            b"{\"id\":7,\"language\":\"typescript\",\"code\":\"const message = \\\"hi\\\";\\n\"}\n",
        );
        assert_eq!(lines[0]["ready"], 1);
        assert_eq!(lines[1]["id"], 7);
        assert!(lines[1]["ansi"].as_str().unwrap().ends_with("\u{1b}[0m\n"));
    }

    #[test]
    fn explicit_raw_mode_matches_absent_mode() {
        let absent = run_once(b"{\"id\":1,\"language\":\"rust\",\"code\":\"let x = 1;\\n\"}\n");
        let explicit = run_once(
            b"{\"id\":1,\"language\":\"rust\",\"code\":\"let x = 1;\\n\",\"mode\":\"raw\"}\n",
        );
        assert_eq!(absent[1]["ansi"], explicit[1]["ansi"]);
    }

    #[test]
    fn diff_wash_paints_given_lines() {
        let lines = run_once(
            b"{\"id\":3,\"mode\":\"diff-wash\",\"language\":\"plaintext\",\"code\":\"a\\nb\\n\",\"wash_per_line\":[null,\"#ff5d7a\"]}\n",
        );
        let ansi = lines[1]["ansi"].as_str().unwrap();
        let parts: Vec<&str> = ansi.split('\n').collect();
        assert_eq!(parts.len(), 3);
        assert!(
            !parts[0].contains("\u{1b}[48;2;255;93;122m"),
            "first line must be unwashed"
        );
        assert!(
            parts[1].starts_with("\u{1b}[48;2;255;93;122m"),
            "second line must be washed"
        );
        assert!(
            parts[1].ends_with("\u{1b}[0m"),
            "washed line must end reset"
        );
    }

    #[test]
    fn diff_wash_rejects_mismatched_length() {
        let lines = run_once(
            b"{\"id\":4,\"mode\":\"diff-wash\",\"language\":\"plaintext\",\"code\":\"a\\nb\\n\",\"wash_per_line\":[null]}\n",
        );
        assert_eq!(lines[1]["id"], 4);
        assert!(lines[1]["error"].as_str().unwrap().contains("length"));
        assert!(lines[1].get("ansi").is_none());
    }

    #[test]
    fn unknown_mode_fails_closed_with_id_echo() {
        let lines =
            run_once(b"{\"id\":9,\"mode\":\"bogus\",\"language\":\"rust\",\"code\":\"x\"}\n");
        assert_eq!(lines[1]["id"], 9);
        assert!(lines[1]["error"].as_str().unwrap().contains("unknown mode"));
    }

    #[test]
    fn malformed_line_echoes_lenient_id() {
        let lines = run_once(b"not json at all\n{\"id\":5,\"language\":\"rust\"\n");
        assert_eq!(lines[1]["id"], 0, "non-JSON line should echo id 0");
        assert!(lines[1]["error"].is_string());
        // Truncated JSON is not recoverable by serde, so id falls back to 0.
        assert_eq!(lines[2]["id"], 0, "truncated JSON should echo id 0");
        assert!(lines[2]["error"].is_string());
    }
}
