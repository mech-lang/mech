use crate::document::{RuleId, SyntaxKind};

use super::super::Parser;
use super::super::rule::rules;
use super::terminal_spec::{
  FixedTerminalSpec, TerminalSpacing, fixed_terminal_spec,
};

pub(crate) const SUPPORTED_RULE_COUNT: usize = 149;

const NON_FIXED_RULES: &[RuleId] = &[
  rules::TRANSITION_OPERATOR,
  rules::OUTPUT_OPERATOR,
  rules::EMOJI_GRAPHEME,
  rules::ALPHA,
  rules::DIGIT,
  rules::ANY,
  rules::ANY_TOKEN,
  rules::FORBIDDEN_EMOJI,
  rules::EMOJI,
  rules::ALPHA_TOKEN,
  rules::DIGIT_TOKEN,
  rules::ALPHANUMERIC,
  rules::UNDERSCORE_DIGIT,
  rules::DIGIT_SEQUENCE,
  rules::GROUPING_SYMBOL,
  rules::PUNCTUATION,
  rules::ESCAPED_CHAR,
  rules::SYMBOL,
  rules::IDENTIFIER_SYMBOL,
  rules::TEXT,
  rules::RAW_TEXT,
  rules::NEW_LINE,
  rules::WHITESPACE,
  rules::WHITESPACE0,
  rules::WHITESPACE1,
  rules::NEWLINE_INDENT,
  rules::WS1E,
  rules::WS0E,
  rules::SPACE_TAB,
  rules::SPACE_TAB0,
  rules::SPACE_TAB1,
  rules::LIST_SEPARATOR,
  rules::ENUM_SEPARATOR,
  rules::IDENTIFIER,
  rules::IDENTIFIER_PATH_SEGMENT_EMOJI,
  rules::IDENTIFIER_PATH_SEGMENT,
  rules::LEFT_ANGLE,
  rules::RIGHT_ANGLE,
  rules::BOX_DRAWING_CHAR,
  rules::BOX_DRAWING_EMOJI,
  rules::TAG,
];

const FORBIDDEN_EMOJI_RULES: &[RuleId] = &[
  rules::BOX_DRAWING_EMOJI,
  rules::NBSP,
  rules::THIN_SPACE,
  rules::MIKA_SECTION_OPEN,
  rules::MIKA_SECTION_CLOSE,
  rules::LEFT_ANGLE2,
  rules::RIGHT_ANGLE2,
];

const GROUPING_SYMBOL_RULES: &[RuleId] = &[
  rules::LEFT_PARENTHESIS,
  rules::RIGHT_PARENTHESIS,
  rules::LEFT_ANGLE,
  rules::RIGHT_ANGLE,
  rules::LEFT_BRACE,
  rules::RIGHT_BRACE,
  rules::LEFT_BRACKET,
  rules::RIGHT_BRACKET,
];

const PUNCTUATION_RULES: &[RuleId] = &[
  rules::PERIOD,
  rules::EXCLAMATION,
  rules::QUESTION,
  rules::COMMA,
  rules::COLON,
  rules::SEMICOLON,
  rules::QUOTE,
  rules::APOSTROPHE,
];

const SYMBOL_RULES: &[RuleId] = &[
  rules::AMPERSAND,
  rules::GRAVE,
  rules::DOLLAR,
  rules::BAR,
  rules::PERCENT,
  rules::AT,
  rules::SLASH,
  rules::HASHTAG,
  rules::EQUAL,
  rules::BACKSLASH,
  rules::TILDE,
  rules::PLUS,
  rules::DASH,
  rules::ASTERISK,
  rules::CARET,
  rules::UNDERSCORE,
];

const IDENTIFIER_SYMBOL_RULES: &[RuleId] = &[
  rules::AMPERSAND,
  rules::DOLLAR,
  rules::PERCENT,
  rules::SLASH,
  rules::HASHTAG,
  rules::BACKSLASH,
  rules::TILDE,
  rules::PLUS,
  rules::DASH,
  rules::ASTERISK,
  rules::CARET,
];

const TEXT_RULES: &[RuleId] = &[
  rules::ALPHA_TOKEN,
  rules::DIGIT_TOKEN,
  rules::EMOJI,
  rules::FORBIDDEN_EMOJI,
  rules::SPACE,
  rules::TAB,
  rules::ESCAPED_CHAR,
  rules::PUNCTUATION,
  rules::GROUPING_SYMBOL,
  rules::SYMBOL,
];

const RAW_TEXT_RULES: &[RuleId] = &[
  rules::ALPHA_TOKEN,
  rules::DIGIT_TOKEN,
  rules::EMOJI,
  rules::FORBIDDEN_EMOJI,
  rules::SPACE,
  rules::TAB,
  rules::PUNCTUATION,
  rules::GROUPING_SYMBOL,
  rules::SYMBOL,
];

const PATH_EMOJI_EXCLUSIONS: &[RuleId] = &[
  rules::SLASH,
  rules::ASTERISK,
  rules::COMMA,
  rules::COLON,
  rules::EQUAL,
  rules::LEFT_BRACE,
  rules::RIGHT_BRACE,
  rules::UNDERSCORE,
  rules::SPACE,
  rules::TAB,
  rules::NEW_LINE,
];

const BOX_DRAWING_CHAR_RULES: &[RuleId] = &[
  rules::BOX_TL,
  rules::BOX_BL,
  rules::BOX_TR,
  rules::BOX_TL_BOLD,
  rules::BOX_BL_BOLD,
  rules::BOX_TR_BOLD,
  rules::BOX_TR_ROUND,
  rules::BOX_BL_ROUND,
  rules::BOX_VERT,
  rules::BOX_CROSS,
  rules::BOX_HORZ,
  rules::BOX_T_LEFT,
  rules::BOX_T_RIGHT,
  rules::BOX_T_TOP,
  rules::BOX_T_BOTTOM,
];

const BOX_DRAWING_EMOJI_RULES: &[RuleId] = &[
  rules::BOX_VERT_BOLD,
  rules::BOX_TL,
  rules::BOX_BL,
  rules::BOX_TR,
  rules::BOX_TL_BOLD,
  rules::BOX_BL_BOLD,
  rules::BOX_TR_BOLD,
  rules::BOX_TL_ROUND,
  rules::BOX_BR_ROUND,
  rules::BOX_TR_ROUND,
  rules::BOX_BL_ROUND,
  rules::BOX_VERT,
  rules::BOX_CROSS,
  rules::BOX_HORZ,
  rules::BOX_T_LEFT,
  rules::BOX_T_RIGHT,
  rules::BOX_T_TOP,
  rules::BOX_T_BOTTOM,
];

pub(crate) fn supports(rule: RuleId) -> bool {
  fixed_terminal_spec(rule).is_some() || NON_FIXED_RULES.contains(&rule)
}

pub(crate) fn parse_rule(
  parser: &mut Parser<'_>,
  rule: RuleId,
) -> bool {
  if !supports(rule) {
    return false;
  }
  let checkpoint = parser.checkpoint();
  let matched = parser.with_canonical_rule(rule, |parser| {
    parse_rule_inner(parser, rule)
  });
  if !matched {
    parser.rewind(checkpoint);
  }
  matched
}

pub(crate) fn parse_exact_tag(
  parser: &mut Parser<'_>,
  literal: &str,
  kind: SyntaxKind,
) -> bool {
  if literal.is_empty() || !kind.is_token() {
    return false;
  }
  let checkpoint = parser.checkpoint();
  let matched = parser.with_canonical_rule(rules::TAG, |parser| {
    consume_exact_token(parser, literal, kind)
  });
  if !matched {
    parser.rewind(checkpoint);
  }
  matched
}

fn parse_rule_inner(parser: &mut Parser<'_>, rule: RuleId) -> bool {
  if let Some(spec) = fixed_terminal_spec(rule) {
    return parse_fixed_terminal(parser, spec);
  }

  match rule {
    rules::TRANSITION_OPERATOR => parse_choice(
      parser,
      &[rules::TRANSITION_OPERATOR_A, rules::TRANSITION_OPERATOR_U],
    ),
    rules::OUTPUT_OPERATOR => parse_choice(
      parser,
      &[rules::OUTPUT_OPERATOR_A, rules::OUTPUT_OPERATOR_U],
    ),
    rules::EMOJI_GRAPHEME => {
      consume_classified_grapheme(parser, SyntaxKind::Emoji, is_emoji)
    }
    rules::ALPHA => consume_classified_grapheme(
      parser,
      SyntaxKind::Alpha,
      char::is_alphabetic,
    ),
    rules::DIGIT => consume_classified_grapheme(
      parser,
      SyntaxKind::Digit,
      char::is_numeric,
    ),
    rules::ANY => consume_any_grapheme(parser),
    rules::ANY_TOKEN => parse_rule(parser, rules::ANY),
    rules::FORBIDDEN_EMOJI => {
      parse_choice(parser, FORBIDDEN_EMOJI_RULES)
    }
    rules::EMOJI => parse_emoji(parser),
    rules::ALPHA_TOKEN => parse_rule(parser, rules::ALPHA),
    rules::DIGIT_TOKEN => parse_rule(parser, rules::DIGIT),
    rules::ALPHANUMERIC => parse_choice(
      parser,
      &[rules::ALPHA_TOKEN, rules::DIGIT_TOKEN],
    ),
    rules::UNDERSCORE_DIGIT => {
      parse_rule(parser, rules::UNDERSCORE)
        && parse_rule(parser, rules::DIGIT_TOKEN)
    }
    rules::DIGIT_SEQUENCE => parse_digit_sequence(parser),
    rules::GROUPING_SYMBOL => {
      parse_choice(parser, GROUPING_SYMBOL_RULES)
    }
    rules::PUNCTUATION => parse_choice(parser, PUNCTUATION_RULES),
    rules::ESCAPED_CHAR => parse_escaped_char(parser),
    rules::SYMBOL => parse_choice(parser, SYMBOL_RULES),
    rules::IDENTIFIER_SYMBOL => {
      parse_choice(parser, IDENTIFIER_SYMBOL_RULES)
    }
    rules::TEXT => parse_choice(parser, TEXT_RULES),
    rules::RAW_TEXT => parse_choice(parser, RAW_TEXT_RULES),
    rules::NEW_LINE => parse_choice(
      parser,
      &[
        rules::CARRIAGE_RETURN_NEW_LINE,
        rules::NEW_LINE_CHAR,
        rules::CARRIAGE_RETURN,
      ],
    ),
    rules::WHITESPACE => parse_choice(
      parser,
      &[rules::SPACE, rules::TAB, rules::NEW_LINE],
    ),
    rules::WHITESPACE0 => {
      parse_zero_or_more(parser, rules::WHITESPACE);
      true
    }
    rules::WHITESPACE1 => {
      parse_one_or_more(parser, rules::WHITESPACE)
    }
    rules::NEWLINE_INDENT => {
      parse_rule(parser, rules::NEW_LINE)
        && {
          parse_zero_or_more(parser, rules::SPACE_TAB);
          true
        }
    }
    rules::WS1E | rules::SPACE_TAB1 => {
      parse_one_or_more(parser, rules::SPACE_TAB)
    }
    rules::WS0E | rules::SPACE_TAB0 => {
      parse_zero_or_more(parser, rules::SPACE_TAB);
      true
    }
    rules::SPACE_TAB => parse_choice(
      parser,
      &[rules::SPACE, rules::TAB, rules::NBSP, rules::THIN_SPACE],
    ),
    rules::LIST_SEPARATOR => {
      parse_rule(parser, rules::WHITESPACE0)
        && parse_rule(parser, rules::COMMA)
        && parse_rule(parser, rules::WHITESPACE0)
    }
    rules::ENUM_SEPARATOR => {
      parse_rule(parser, rules::WHITESPACE0)
        && parse_rule(parser, rules::BAR)
        && parse_rule(parser, rules::WHITESPACE0)
    }
    rules::IDENTIFIER => parse_identifier(parser),
    rules::IDENTIFIER_PATH_SEGMENT_EMOJI => {
      parse_identifier_path_segment_emoji(parser)
    }
    rules::IDENTIFIER_PATH_SEGMENT => {
      parse_identifier_path_segment(parser)
    }
    rules::LEFT_ANGLE => parse_choice(
      parser,
      &[rules::LEFT_ANGLE1, rules::LEFT_ANGLE2],
    ),
    rules::RIGHT_ANGLE => parse_choice(
      parser,
      &[rules::RIGHT_ANGLE1, rules::RIGHT_ANGLE2],
    ),
    rules::BOX_DRAWING_CHAR => {
      parse_choice(parser, BOX_DRAWING_CHAR_RULES)
    }
    rules::BOX_DRAWING_EMOJI => {
      parse_choice(parser, BOX_DRAWING_EMOJI_RULES)
    }
    // `tag` requires the literal supplied by its caller.
    rules::TAG => false,
    _ => false,
  }
}

fn parse_fixed_terminal(
  parser: &mut Parser<'_>,
  spec: &FixedTerminalSpec,
) -> bool {
  if spec.spacing == TerminalSpacing::Whitespace0Both
    && !parse_rule(parser, rules::WHITESPACE0)
  {
    return false;
  }
  if !consume_exact_token(parser, spec.literal, spec.kind) {
    return false;
  }
  spec.spacing != TerminalSpacing::Whitespace0Both
    || parse_rule(parser, rules::WHITESPACE0)
}

fn consume_exact_token(
  parser: &mut Parser<'_>,
  literal: &str,
  kind: SyntaxKind,
) -> bool {
  let Ok(len) = u32::try_from(literal.len()) else {
    return false;
  };
  if len == 0 || parser.cursor().grapheme_literal_end(literal).is_none() {
    return false;
  }
  parser.bump_bytes_token(len, kind).is_some()
}

fn consume_any_grapheme(parser: &mut Parser<'_>) -> bool {
  let Some(range) = parser.bump_grapheme_raw() else {
    return false;
  };
  parser.token(SyntaxKind::Any, range);
  true
}

fn consume_classified_grapheme(
  parser: &mut Parser<'_>,
  kind: SyntaxKind,
  classify: impl FnOnce(char) -> bool,
) -> bool {
  let Some(first) = parser.cursor().peek_char() else {
    return false;
  };
  if !classify(first) {
    return false;
  }
  let Some(range) = parser.bump_grapheme_raw() else {
    return false;
  };
  parser.token(kind, range);
  true
}

fn is_emoji(first: char) -> bool {
  !first.is_alphanumeric() && !first.is_ascii()
}

fn parse_choice(parser: &mut Parser<'_>, choices: &[RuleId]) -> bool {
  for rule in choices {
    if parse_rule(parser, *rule) {
      return true;
    }
    if parser.is_halted() {
      return false;
    }
  }
  false
}

fn matches_without_consuming(
  parser: &mut Parser<'_>,
  rule: RuleId,
) -> bool {
  let checkpoint = parser.checkpoint();
  let matched = parse_rule(parser, rule);
  parser.rewind(checkpoint);
  matched
}

fn parse_emoji(parser: &mut Parser<'_>) -> bool {
  if matches_without_consuming(parser, rules::FORBIDDEN_EMOJI) {
    return false;
  }
  parse_rule(parser, rules::EMOJI_GRAPHEME)
}

fn parse_zero_or_more(parser: &mut Parser<'_>, rule: RuleId) {
  loop {
    let before = parser.offset();
    if !parse_rule(parser, rule) {
      break;
    }
    if parser.offset() == before || parser.is_halted() {
      break;
    }
  }
}

fn parse_one_or_more(parser: &mut Parser<'_>, rule: RuleId) -> bool {
  if !parse_rule(parser, rule) {
    return false;
  }
  parse_zero_or_more(parser, rule);
  true
}

fn parse_digit_sequence(parser: &mut Parser<'_>) -> bool {
  let marker = parser.start();
  if !parse_rule(parser, rules::DIGIT_TOKEN) {
    return false;
  }
  loop {
    let before = parser.offset();
    if !parse_choice(
      parser,
      &[rules::UNDERSCORE_DIGIT, rules::DIGIT_TOKEN],
    ) {
      break;
    }
    if parser.offset() == before || parser.is_halted() {
      break;
    }
  }
  marker.complete(parser, SyntaxKind::DigitSequence);
  true
}

fn parse_escaped_char(parser: &mut Parser<'_>) -> bool {
  let marker = parser.start();
  if !parse_rule(parser, rules::BACKSLASH)
    || !consume_escaped_value(parser)
  {
    return false;
  }
  marker.complete(parser, SyntaxKind::EscapedCharacter);
  true
}

fn consume_escaped_value(parser: &mut Parser<'_>) -> bool {
  let Some(first) = parser.cursor().peek_char() else {
    return false;
  };
  let accepted = first.is_alphabetic()
    || matches!(
      first,
      '&' | '`' | '$' | '|' | '%' | '@' | '/' | '#' | '=' | '\\'
        | '~' | '+' | '-' | '*' | '^' | '_' | '.' | '!' | '?' | ','
        | ':' | ';' | '"' | '\''
    );
  if !accepted {
    return false;
  }
  let Some(range) = parser.bump_grapheme_raw() else {
    return false;
  };
  parser.token(SyntaxKind::EscapedChar, range);
  true
}

fn parse_identifier(parser: &mut Parser<'_>) -> bool {
  let marker = parser.start();
  if !parse_choice(parser, &[rules::ALPHA_TOKEN, rules::EMOJI]) {
    return false;
  }
  loop {
    let before = parser.offset();
    if !parse_choice(
      parser,
      &[
        rules::ALPHA_TOKEN,
        rules::DIGIT_TOKEN,
        rules::IDENTIFIER_SYMBOL,
        rules::EMOJI,
      ],
    ) {
      break;
    }
    if parser.offset() == before || parser.is_halted() {
      break;
    }
  }
  marker.complete(parser, SyntaxKind::Identifier);
  true
}

fn parse_identifier_path_segment_emoji(
  parser: &mut Parser<'_>,
) -> bool {
  if PATH_EMOJI_EXCLUSIONS
    .iter()
    .any(|rule| matches_without_consuming(parser, *rule))
  {
    return false;
  }
  parse_rule(parser, rules::EMOJI)
}

fn parse_identifier_path_segment(parser: &mut Parser<'_>) -> bool {
  let marker = parser.start();
  if !parse_choice(
    parser,
    &[rules::ALPHA_TOKEN, rules::IDENTIFIER_PATH_SEGMENT_EMOJI],
  ) {
    return false;
  }
  loop {
    let before = parser.offset();
    if !parse_choice(
      parser,
      &[
        rules::ALPHA_TOKEN,
        rules::DIGIT_TOKEN,
        rules::DASH,
        rules::IDENTIFIER_PATH_SEGMENT_EMOJI,
      ],
    ) {
      break;
    }
    if parser.offset() == before || parser.is_halted() {
      break;
    }
  }
  marker.complete(parser, SyntaxKind::IdentifierPathSegment);
  true
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn support_table_is_exact_and_duplicate_free() {
    assert_eq!(NON_FIXED_RULES.len(), 41);
    assert_eq!(
      super::super::terminal_spec::FIXED_TERMINAL_COUNT
        + NON_FIXED_RULES.len(),
      SUPPORTED_RULE_COUNT,
    );
    for (index, rule) in NON_FIXED_RULES.iter().enumerate() {
      assert!(
        NON_FIXED_RULES[..index].iter().all(|earlier| earlier != rule),
        "duplicate non-fixed canonical base rule {rule}",
      );
      assert!(supports(*rule));
      assert!(fixed_terminal_spec(*rule).is_none());
    }
  }
}
