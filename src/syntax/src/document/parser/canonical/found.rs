//! Found-syntax classification for canonical grammar diagnostics.

use alloc::string::{String, ToString};

use crate::document::{FoundSyntax, SyntaxKind, TextRange, TextSize};

use super::super::{Cursor, Parser};
use super::combinator::is_grammar_ignored;
use super::terminal_spec::{FIXED_TERMINALS, FixedTerminalSpec};

/// Classify the next logical syntax at `at` using the canonical lexical
/// contracts and the grammar root's ignored-trivia behavior.
pub(crate) fn found_syntax(parser: &Parser<'_>, at: TextSize) -> FoundSyntax {
  let context_end = parser.cursor().context_end();
  if at >= context_end {
    return eof();
  }

  let mut cursor = Cursor::for_range_with_context(
    parser.source(),
    TextRange::new(at, context_end),
    context_end,
  );
  while cursor.peek_char().is_some_and(is_grammar_ignored) {
    let _ = cursor.bump_char();
  }
  if cursor.is_eof() {
    return eof();
  }

  let mut longest = None;
  for spec in FIXED_TERMINALS {
    if cursor
      .filtered_grapheme_literal_end(spec.literal, is_grammar_ignored)
      .is_some()
      && longest.map_or(
        true,
        |current: &FixedTerminalSpec| {
          spec.literal.len() > current.literal.len()
        },
      )
    {
      longest = Some(spec);
    }
  }
  if let Some(spec) = longest {
    return FoundSyntax {
      kind: Some(spec.kind),
      text: Some(spec.literal.to_string()),
    };
  }

  let Some((first, range)) =
    cursor.peek_filtered_grapheme_range(is_grammar_ignored)
  else {
    return eof();
  };
  let kind = if first.is_alphabetic() {
    SyntaxKind::Alpha
  } else if first.is_numeric() {
    SyntaxKind::Digit
  } else if is_canonical_emoji(first) {
    SyntaxKind::Emoji
  } else {
    SyntaxKind::Any
  };
  let text = parser.source().text(range).ok().map(|physical| {
    physical
      .chars()
      .filter(|character| !is_grammar_ignored(*character))
      .collect::<String>()
  });

  FoundSyntax {
    kind: Some(kind),
    text,
  }
}

fn is_canonical_emoji(first: char) -> bool {
  !first.is_alphanumeric() && !first.is_ascii()
}

fn eof() -> FoundSyntax {
  FoundSyntax {
    kind: Some(SyntaxKind::Eof),
    text: None,
  }
}
