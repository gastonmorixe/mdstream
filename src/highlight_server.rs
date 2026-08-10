use std::io::{BufRead, Write};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::highlight::RawCodeHighlighter;
use crate::theme::CodeTheme;

#[derive(Debug, Deserialize)]
struct HighlightRequest {
    id: u64,
    language: String,
    code: String,
}

#[derive(Serialize)]
struct ReadyResponse {
    ready: u8,
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
/// The first line written is `{"ready":1}` after syntect assets have loaded.
/// Each later input line must contain `id`, `language`, and `code`. Requests are
/// handled sequentially and produce exactly one JSON response line.
pub fn run_highlight_server<R: BufRead, W: Write>(
    input: R,
    mut output: W,
    code_theme: CodeTheme,
    show_background: bool,
) -> Result<()> {
    let highlighter = RawCodeHighlighter::new(code_theme, show_background);
    write_json_line(&mut output, &ReadyResponse { ready: 1 })?;

    for line in input.lines() {
        let line = line?;
        match serde_json::from_str::<HighlightRequest>(&line) {
            Ok(request) => match highlighter.highlight(&request.language, &request.code) {
                Ok(ansi) => write_json_line(
                    &mut output,
                    &HighlightResponse {
                        id: request.id,
                        ansi: Some(&ansi),
                        error: None,
                    },
                )?,
                Err(error) => {
                    let error = error.to_string();
                    write_json_line(
                        &mut output,
                        &HighlightResponse {
                            id: request.id,
                            ansi: None,
                            error: Some(&error),
                        },
                    )?;
                }
            },
            Err(error) => {
                let id = serde_json::from_str::<serde_json::Value>(&line)
                    .ok()
                    .and_then(|value| value.get("id").and_then(serde_json::Value::as_u64))
                    .unwrap_or(0);
                let error = error.to_string();
                write_json_line(
                    &mut output,
                    &HighlightResponse {
                        id,
                        ansi: None,
                        error: Some(&error),
                    },
                )?;
            }
        }
    }

    Ok(())
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

    #[test]
    fn emits_ready_and_escaped_json_response_lines() {
        let input = Cursor::new(
            b"{\"id\":7,\"language\":\"typescript\",\"code\":\"const message = \\\"hi\\\";\\n\"}\n",
        );
        let mut output = Vec::new();

        run_highlight_server(input, &mut output, CodeTheme::Mdstream, false).unwrap();

        let lines: Vec<_> = String::from_utf8(output)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str::<Value>(line).unwrap())
            .collect();
        assert_eq!(lines[0], serde_json::json!({ "ready": 1 }));
        assert_eq!(lines[1]["id"], 7);
        assert!(lines[1]["ansi"].as_str().unwrap().ends_with("\u{1b}[0m\n"));
    }
}
