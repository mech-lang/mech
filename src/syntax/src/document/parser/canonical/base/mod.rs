use crate::document::{RuleId, SyntaxKind};

pub(crate) mod continuation;
mod exact_tag;
pub(crate) use exact_tag::ExactTag;

use super::super::Parser;
use super::super::rule::rules;
use super::terminal_spec::{FixedTerminalSpec, TerminalSpacing, fixed_terminal_spec};

#[cfg(test)]
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

pub(crate) fn parse_rule(parser: &mut Parser<'_>, rule: RuleId) -> bool {
    let mut continuation = continuation::Continuation::new(rule);
    let mut allowance = u64::MAX;
    loop {
        match continuation.advance(parser, true, &mut allowance) {
            continuation::Progress::Complete(result) => return result,
            continuation::Progress::NeedsProcessing => allowance = u64::MAX,
            continuation::Progress::NeedInput | continuation::Progress::Limited => {
                unreachable!("final canonical base input")
            }
        }
    }
}

pub(crate) fn parse_exact_tag(parser: &mut Parser<'_>, literal: &str, kind: SyntaxKind) -> bool {
    let mut child = ExactTag::new(literal, kind);
    loop {
        let mut allowance = u64::MAX;
        match child.advance(parser, true, &mut allowance) {
            continuation::Progress::Complete(result) => return result,
            continuation::Progress::NeedsProcessing => {}
            _ => unreachable!("sealed TAG input"),
        }
    }
}

fn is_emoji(first: char) -> bool {
    !first.is_alphanumeric() && !first.is_ascii()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn support_table_is_exact_and_duplicate_free() {
        assert_eq!(NON_FIXED_RULES.len(), 41);
        assert_eq!(
            super::super::terminal_spec::FIXED_TERMINAL_COUNT + NON_FIXED_RULES.len(),
            SUPPORTED_RULE_COUNT,
        );
        for (index, rule) in NON_FIXED_RULES.iter().enumerate() {
            assert!(
                NON_FIXED_RULES[..index]
                    .iter()
                    .all(|earlier| earlier != rule),
                "duplicate non-fixed canonical base rule {rule}",
            );
            assert!(supports(*rule));
            assert!(fixed_terminal_spec(*rule).is_none());
        }
    }
}
