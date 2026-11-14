//! Canonical base productions shared by execution and predictive analysis.
use super::*;
pub(in super::super) enum Spec {
    Alias(RuleId),
    Choice(&'static [RuleId]),
    Sequence(&'static [RuleId]),
    Repeat {
        required: bool,
        choices: &'static [RuleId],
    },
    RepeatedNode {
        kind: SyntaxKind,
        first: &'static [RuleId],
        rest: &'static [RuleId],
        single_first: bool,
    },
    Classified(SyntaxKind, fn(char) -> bool),
    Raw(Option<SyntaxKind>),
    Unless {
        excluded: &'static [RuleId],
        rule: RuleId,
    },
    NewlineIndent,
    Escape,
    Never,
}
pub(in super::super) fn specification(rule: RuleId) -> Option<Spec> {
    Some(match rule {
        rules::TRANSITION_OPERATOR => {
            Spec::Choice(&[rules::TRANSITION_OPERATOR_A, rules::TRANSITION_OPERATOR_U])
        }
        rules::OUTPUT_OPERATOR => {
            Spec::Choice(&[rules::OUTPUT_OPERATOR_A, rules::OUTPUT_OPERATOR_U])
        }
        rules::EMOJI_GRAPHEME => Spec::Classified(SyntaxKind::Emoji, is_emoji),
        rules::ALPHA => Spec::Classified(SyntaxKind::Alpha, char::is_alphabetic),
        rules::DIGIT => Spec::Classified(SyntaxKind::Digit, char::is_numeric),
        rules::ANY => Spec::Raw(Some(SyntaxKind::Any)),
        rules::ANY_TOKEN => Spec::Alias(rules::ANY),
        rules::ALPHA_TOKEN => Spec::Alias(rules::ALPHA),
        rules::DIGIT_TOKEN => Spec::Alias(rules::DIGIT),
        rules::FORBIDDEN_EMOJI => Spec::Choice(FORBIDDEN_EMOJI_RULES),
        rules::EMOJI => Spec::Unless {
            excluded: &[rules::FORBIDDEN_EMOJI],
            rule: rules::EMOJI_GRAPHEME,
        },
        rules::ALPHANUMERIC => Spec::Choice(&[rules::ALPHA_TOKEN, rules::DIGIT_TOKEN]),
        rules::UNDERSCORE_DIGIT => Spec::Sequence(&[rules::UNDERSCORE, rules::DIGIT_TOKEN]),
        rules::DIGIT_SEQUENCE => Spec::RepeatedNode {
            kind: SyntaxKind::DigitSequence,
            first: &[rules::DIGIT_TOKEN],
            rest: &[rules::UNDERSCORE_DIGIT, rules::DIGIT_TOKEN],
            single_first: true,
        },
        rules::GROUPING_SYMBOL => Spec::Choice(GROUPING_SYMBOL_RULES),
        rules::PUNCTUATION => Spec::Choice(PUNCTUATION_RULES),
        rules::ESCAPED_CHAR => Spec::Escape,
        rules::SYMBOL => Spec::Choice(SYMBOL_RULES),
        rules::IDENTIFIER_SYMBOL => Spec::Choice(IDENTIFIER_SYMBOL_RULES),
        rules::TEXT => Spec::Choice(TEXT_RULES),
        rules::RAW_TEXT => Spec::Choice(RAW_TEXT_RULES),
        rules::NEW_LINE => Spec::Choice(NEW_LINE_RULES),
        rules::WHITESPACE => Spec::Choice(WHITESPACE_RULES),
        rules::WHITESPACE0 => Spec::Repeat {
            required: false,
            choices: &[rules::WHITESPACE],
        },
        rules::WHITESPACE1 => Spec::Repeat {
            required: true,
            choices: &[rules::WHITESPACE],
        },
        rules::NEWLINE_INDENT => Spec::NewlineIndent,
        rules::WS1E | rules::SPACE_TAB1 => Spec::Repeat {
            required: true,
            choices: &[rules::SPACE_TAB],
        },
        rules::WS0E | rules::SPACE_TAB0 => Spec::Repeat {
            required: false,
            choices: &[rules::SPACE_TAB],
        },
        rules::SPACE_TAB => Spec::Choice(SPACE_TAB_RULES),
        rules::LIST_SEPARATOR => {
            Spec::Sequence(&[rules::WHITESPACE0, rules::COMMA, rules::WHITESPACE0])
        }
        rules::ENUM_SEPARATOR => {
            Spec::Sequence(&[rules::WHITESPACE0, rules::BAR, rules::WHITESPACE0])
        }
        rules::IDENTIFIER => Spec::RepeatedNode {
            kind: SyntaxKind::Identifier,
            first: &[rules::ALPHA_TOKEN, rules::EMOJI],
            rest: &[
                rules::ALPHA_TOKEN,
                rules::DIGIT_TOKEN,
                rules::UNDERSCORE,
                rules::IDENTIFIER_SYMBOL,
                rules::EMOJI,
            ],
            single_first: false,
        },
        rules::IDENTIFIER_PATH_SEGMENT_EMOJI => Spec::Unless {
            excluded: PATH_EMOJI_EXCLUSIONS,
            rule: rules::EMOJI,
        },
        rules::IDENTIFIER_PATH_SEGMENT => Spec::RepeatedNode {
            kind: SyntaxKind::IdentifierPathSegment,
            first: &[rules::ALPHA_TOKEN, rules::IDENTIFIER_PATH_SEGMENT_EMOJI],
            rest: &[
                rules::ALPHA_TOKEN,
                rules::DIGIT_TOKEN,
                rules::DASH,
                rules::IDENTIFIER_PATH_SEGMENT_EMOJI,
            ],
            single_first: false,
        },
        rules::LEFT_ANGLE => Spec::Choice(&[rules::LEFT_ANGLE1, rules::LEFT_ANGLE2]),
        rules::RIGHT_ANGLE => Spec::Choice(&[rules::RIGHT_ANGLE1, rules::RIGHT_ANGLE2]),
        rules::BOX_DRAWING_CHAR => Spec::Choice(BOX_DRAWING_CHAR_RULES),
        rules::BOX_DRAWING_EMOJI => Spec::Choice(BOX_DRAWING_EMOJI_RULES),
        rules::TAG => Spec::Never,
        _ => return None,
    })
}
