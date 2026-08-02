use std::fmt::Write as _;

/// Render arbitrary UTF-8 as one safe Rust string literal.
pub fn rust_string_literal(input: &str) -> String {
    let mut literal = String::with_capacity(input.len() + 2);
    literal.push('"');
    for character in input.chars() {
        match character {
            '"' => literal.push_str("\\\""),
            '\\' => literal.push_str("\\\\"),
            '\n' => literal.push_str("\\n"),
            '\r' => literal.push_str("\\r"),
            '\t' => literal.push_str("\\t"),
            '\0' => literal.push_str("\\0"),
            character if character.is_control() => {
                write!(&mut literal, "\\u{{{:x}}}", character as u32)
                    .expect("writing to String cannot fail");
            }
            character => literal.push(character),
        }
    }
    literal.push('"');
    literal
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_literals_escape_every_unsafe_character_class() {
        assert_eq!(
            rust_string_literal("quote=\" slash=\\\n\r\t\0\u{7}\u{7f} café"),
            "\"quote=\\\" slash=\\\\\\n\\r\\t\\0\\u{7}\\u{7f} café\""
        );
    }

    #[test]
    fn rust_literals_do_not_create_source_lines() {
        let literal = rust_string_literal("first\nsecond");
        assert_eq!(literal.lines().count(), 1);
        assert_eq!(literal, "\"first\\nsecond\"");
    }
}
