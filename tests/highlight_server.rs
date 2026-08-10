use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use serde_json::{Value, json};

struct Server {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl Server {
    fn start(args: &[&str]) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_mdstream"))
            .args(args)
            .arg("--highlight-server")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        Self {
            child,
            stdin,
            stdout,
        }
    }

    fn read(&mut self) -> Value {
        let mut line = String::new();
        self.stdout.read_line(&mut line).unwrap();
        assert!(!line.is_empty(), "highlight server closed stdout");
        serde_json::from_str(&line).unwrap()
    }

    fn request(&mut self, request: Value) -> Value {
        serde_json::to_writer(&mut self.stdin, &request).unwrap();
        self.stdin.write_all(b"\n").unwrap();
        self.stdin.flush().unwrap();
        self.read()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn strip_ansi(text: &str) -> String {
    let ansi = regex::Regex::new(r"\x1b\[[0-9;?]*[ -/]*[@-~]").unwrap();
    ansi.replace_all(text, "").into_owned()
}

#[test]
fn ready_handshake_precedes_responses() {
    let mut server = Server::start(&[]);
    assert_eq!(server.read(), json!({ "ready": 1 }));
}

#[test]
fn highlights_typescript_without_markdown_frame_or_padding() {
    let mut server = Server::start(&["--padding", "12"]);
    assert_eq!(server.read(), json!({ "ready": 1 }));

    let response = server.request(json!({
        "id": 1,
        "language": "typescript",
        "code": "const value: number = 42;"
    }));
    let ansi = response["ansi"].as_str().unwrap();

    assert_eq!(response["id"], 1);
    assert_eq!(strip_ansi(ansi), "const value: number = 42;");
    assert!(ansi.starts_with("\u{1b}["));
    assert!(ansi.ends_with("\u{1b}[0m"));
    assert!(!ansi.contains('─'));
    assert!(!ansi.starts_with(' '));
}

#[test]
fn unknown_language_falls_back_to_plaintext() {
    let mut server = Server::start(&[]);
    server.read();
    let code = "not syntax <&>\nsecond line\n";
    let response = server.request(json!({
        "id": 2,
        "language": "totally-unknown-language",
        "code": code
    }));

    assert!(response.get("error").is_none());
    assert_eq!(strip_ansi(response["ansi"].as_str().unwrap()), code);
}

#[test]
fn protocol_round_trips_json_escaping_newlines_and_trailing_shape() {
    let mut server = Server::start(&[]);
    server.read();

    for (id, code) in [
        (10, "const text = \"quoted\\\\value\";"),
        (11, "line one\nline two\n"),
        (12, "line one\nline two"),
        (13, "line one\n\n"),
    ] {
        let response = server.request(json!({
            "id": id,
            "language": "typescript",
            "code": code
        }));
        let ansi = response["ansi"].as_str().unwrap();
        assert_eq!(strip_ansi(ansi), code);
        assert_eq!(ansi.ends_with('\n'), code.ends_with('\n'));
    }
}

#[test]
fn multiline_request_keeps_lexical_state() {
    let mut server = Server::start(&[]);
    server.read();
    let code = "/* opening\nstill a comment */\nconst done = true;";
    let response = server.request(json!({
        "id": 20,
        "language": "typescript",
        "code": code
    }));
    let ansi = response["ansi"].as_str().unwrap();

    assert_eq!(strip_ansi(ansi), code);
    assert!(ansi.contains("\u{1b}[38;2;113;123;148m"));
    assert!(ansi.contains("\u{1b}[38;2;123;167;255m"));
}
