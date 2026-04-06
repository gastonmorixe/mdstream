#![allow(dead_code)]

use mdstream::renderer::StreamingMarkdownRenderer;
use std::io::Cursor;

pub fn strip_ansi(text: &str) -> String {
    let re = regex::Regex::new(r"\x1b\[[0-9;?]*[ -/]*[@-~]").unwrap();
    re.replace_all(text, "").replace('\r', "")
}

#[derive(Default)]
pub struct FakeTerminal {
    lines: Vec<String>,
    current: String,
}

impl FakeTerminal {
    pub fn rendered(&self) -> String {
        if self.current.is_empty() {
            self.lines.join("\n")
        } else {
            self.lines
                .iter()
                .cloned()
                .chain(std::iter::once(self.current.clone()))
                .collect::<Vec<_>>()
                .join("\n")
        }
    }
}

impl std::io::Write for FakeTerminal {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let ansi = regex::Regex::new(r"\x1b\[[0-9;?]*[ -/]*[@-~]").unwrap();
        let text = std::str::from_utf8(buf).unwrap();
        let mut i = 0;
        while i < text.len() {
            if text[i..].starts_with("\r\x1b[K") {
                self.current.clear();
                i += 4;
                continue;
            }

            let ch = text[i..].chars().next().unwrap();
            if ch == '\n' {
                self.lines.push(self.current.clone());
                self.current.clear();
            } else if ch == '\r' {
                self.current.clear();
            } else if ch == '\x1b' {
                if let Some(mat) = ansi.find(&text[i..]) {
                    i += mat.end();
                    continue;
                }
            } else {
                self.current.push(ch);
            }
            i += ch.len_utf8();
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub fn run_reader(renderer: &mut StreamingMarkdownRenderer, bytes: Vec<u8>) -> String {
    let mut input = Cursor::new(bytes);
    let mut out = FakeTerminal::default();
    renderer.run_reader(&mut input, &mut out).unwrap();
    out.rendered()
}
