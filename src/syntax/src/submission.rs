//! Source-level terminal metadata for interactive submissions.
//!
//! Hosts must not reimplement Mech's lexical rules to decide whether a final
//! semicolon suppresses automatic display. This module owns that decision at
//! the syntax boundary next to `code_terminal`.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SubmissionTerminal {
    pub byte_offset: usize,
    pub suppresses_value: bool,
}

pub fn submission_terminal(source: &str) -> Option<SubmissionTerminal> {
    #[derive(Clone, Copy)]
    enum State {
        Code,
        String,
        RawString,
        LineComment,
    }

    let bytes = source.as_bytes();
    let mut state = State::Code;
    let mut last_code = None;
    let mut index = 0;
    while index < bytes.len() {
        match state {
            State::Code => {
                if bytes[index..].starts_with(b"\"\"\"") {
                    last_code = Some(index + 2);
                    state = State::RawString;
                    index += 3;
                } else if bytes[index] == b'"' {
                    last_code = Some(index);
                    state = State::String;
                    index += 1;
                } else if begins_line_comment(bytes, index) {
                    state = State::LineComment;
                    index += 2;
                } else {
                    if !bytes[index].is_ascii_whitespace() {
                        last_code = Some(index);
                    }
                    index += 1;
                }
            }
            State::String => {
                if bytes[index] == b'\\' {
                    index = (index + 2).min(bytes.len());
                } else {
                    if bytes[index] == b'"' {
                        last_code = Some(index);
                        state = State::Code;
                    }
                    index += 1;
                }
            }
            State::RawString => {
                if bytes[index..].starts_with(b"\"\"\"") {
                    last_code = Some(index + 2);
                    state = State::Code;
                    index += 3;
                } else {
                    index += 1;
                }
            }
            State::LineComment => {
                if bytes[index] == b'\n' || bytes[index] == b'\r' {
                    state = State::Code;
                }
                index += 1;
            }
        }
    }
    last_code.map(|byte_offset| SubmissionTerminal {
        byte_offset,
        suppresses_value: bytes[byte_offset] == b';',
    })
}

fn begins_line_comment(bytes: &[u8], index: usize) -> bool {
    (bytes[index..].starts_with(b"--") || bytes[index..].starts_with(b"//"))
        && !inside_resource_uri_token(bytes, index)
}

fn inside_resource_uri_token(bytes: &[u8], index: usize) -> bool {
    let token_start = bytes[..index]
        .iter()
        .rposition(|byte| {
            byte.is_ascii_whitespace()
                || matches!(*byte, b'(' | b')' | b'{' | b'}' | b'[' | b']' | b',' | b';')
        })
        .map_or(0, |delimiter| delimiter + 1);
    (index > token_start && bytes[index - 1] == b':')
        || bytes[token_start..=index]
            .windows(3)
            .any(|window| window == b"://")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_metadata_follows_comments_strings_and_resource_uris() {
        for source in [
            "1 + 1; -- suppressed\n",
            "1 + 1; // suppressed\n",
            "1 + 1;\n-- later comment\n",
        ] {
            assert!(submission_terminal(source).unwrap().suppresses_value);
        }
        for source in [
            "1 + 1 -- comment ;\n",
            "1 + 1-- comment ;\n",
            "1 + 1// comment ;\n",
            "\"text; -- still text\"\n",
            "@out := console://repl/output{:write(line)}\n",
            "@out := console://repl//output-part{:write(line)}\n",
        ] {
            assert!(!submission_terminal(source).unwrap().suppresses_value);
        }
    }
}
