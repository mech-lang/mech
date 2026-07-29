use crate::document::{SyntaxKind, TextRange, TextSize};

use super::Cursor;

pub fn is_horizontal_space(character: char) -> bool {
  matches!(character, ' ' | '\t' | '\u{00a0}' | '\u{2009}')
}

pub fn is_newline_start(cursor: &Cursor<'_>) -> bool {
  matches!(cursor.byte(), Some(b'\r' | b'\n'))
}

pub fn newline_range(cursor: &mut Cursor<'_>) -> Option<TextRange> {
  let start = cursor.offset();
  match cursor.byte()? {
    b'\r' if cursor.byte_at(1) == Some(b'\n') => cursor.bump_bytes(2),
    b'\r' | b'\n' => cursor.bump_bytes(1),
    _ => None,
  }
  .or_else(|| Some(TextRange::empty(start)))
}

pub fn is_identifier_start(character: char) -> bool {
  character.is_alphabetic() || is_emoji_like(character)
}

pub fn is_identifier_continue(character: char) -> bool {
  character.is_alphanumeric()
    || is_emoji_like(character)
    || matches!(
      character,
      '&' | '$' | '%' | '/' | '#' | '\\' | '~' | '+' | '-' | '*' | '^'
    )
}

fn is_emoji_like(character: char) -> bool {
  !character.is_ascii()
    && !character.is_whitespace()
    && !matches!(
      character,
      '(' | ')' | '[' | ']' | '{' | '}' | ':' | '=' | ';' | '`'
    )
}

pub fn token_kind_for_char(character: char) -> SyntaxKind {
  match character {
    ' ' | '\t' | '\u{00a0}' | '\u{2009}' => SyntaxKind::Whitespace,
    '\r' | '\n' => SyntaxKind::Newline,
    ':' => SyntaxKind::Colon,
    '=' => SyntaxKind::Equal,
    '+' => SyntaxKind::Plus,
    '(' => SyntaxKind::LeftParen,
    ')' => SyntaxKind::RightParen,
    '.' => SyntaxKind::Period,
    '-' => SyntaxKind::Dash,
    ';' => SyntaxKind::Semicolon,
    character if character.is_ascii_digit() => SyntaxKind::IntegerToken,
    character if is_identifier_start(character) => SyntaxKind::IdentifierToken,
    _ => SyntaxKind::Text,
  }
}

pub fn line_end(source_len: TextSize, mut cursor: Cursor<'_>) -> TextSize {
  while !cursor.is_eof() && !is_newline_start(&cursor) {
    let _ = cursor.bump_char();
  }
  TextSize(cursor.offset().0.min(source_len.0))
}
