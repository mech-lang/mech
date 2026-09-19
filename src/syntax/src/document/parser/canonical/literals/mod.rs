//! Canonical literal and number leaf productions.
//!
//! The direct leaves originated in Phase 2C. The candidate enclosing `literal`
//! production composes them with recursive kinds in `recursive_core::literals`.

use alloc::string::String;

use crate::document::{
    Diagnostic, DiagnosticAnchor, DiagnosticCode, DiagnosticFix, DiagnosticLabel, DiagnosticPhase,
    DiagnosticTags, ExpectedSyntax, FixApplicability, NodeFlags, RecoveryAction, Severity,
    SyntaxKind, TextEdit, TextRange, TextSize,
};

use super::super::Parser;
use super::super::rule::rules;
use super::base;
use super::combinator::{self, Attempt};

mod continuation;
pub(crate) use continuation::{Continuation, Progress};
pub(crate) fn supports(rule: crate::document::RuleId) -> bool {
    matches!(
        rule,
        rules::EMPTY
            | rules::ATOM
            | rules::BOOLEAN
            | rules::TRUE_LITERAL
            | rules::FALSE_LITERAL
            | rules::NUMBER
            | rules::COMPLEX_NUMBER
            | rules::REAL_NUMBER
            | rules::UNTYPED_REAL_NUMBER
            | rules::RATIONAL_LITERAL
            | rules::SCIENTIFIC_LITERAL
            | rules::FLOAT_DECIMAL_START
            | rules::FLOAT_FULL
            | rules::FLOAT_LITERAL
            | rules::INTEGER_LITERAL
            | rules::TYPED_INTEGER
            | rules::UNTYPED_INTEGER
            | rules::DECIMAL_LITERAL
            | rules::HEXADECIMAL_LITERAL
            | rules::OCTAL_LITERAL
            | rules::BINARY_LITERAL
    )
}
fn drive(parser: &mut Parser<'_>, rule: crate::document::RuleId) -> Attempt {
    let mut continuation = Continuation::new(rule);
    loop {
        let mut allowance = u64::MAX;
        match continuation.advance(parser, true, &mut allowance) {
            Progress::Complete(result) => return result,
            Progress::NeedsProcessing => {}
            _ => unreachable!("final literal input"),
        }
    }
}
pub(crate) fn parse_empty(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::EMPTY)
}
pub(crate) fn parse_atom(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::ATOM)
}
pub(crate) fn parse_string(parser: &mut Parser<'_>) -> Attempt {
    super::strings::parse_rule(parser, rules::STRING)
}
pub(crate) fn parse_utf8_string(parser: &mut Parser<'_>) -> Attempt {
    super::strings::parse_rule(parser, rules::UTF8_STRING)
}
pub(crate) fn parse_raw_string(parser: &mut Parser<'_>) -> Attempt {
    super::strings::parse_rule(parser, rules::RAW_STRING)
}
pub(crate) fn parse_boolean(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::BOOLEAN)
}
pub(crate) fn parse_true_literal(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::TRUE_LITERAL)
}
pub(crate) fn parse_false_literal(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::FALSE_LITERAL)
}
pub(crate) fn parse_number(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::NUMBER)
}
pub(crate) fn parse_complex_number(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::COMPLEX_NUMBER)
}
pub(crate) fn parse_real_number(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::REAL_NUMBER)
}
pub(crate) fn parse_untyped_real_number(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::UNTYPED_REAL_NUMBER)
}
pub(crate) fn parse_rational_literal(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::RATIONAL_LITERAL)
}
pub(crate) fn parse_scientific_literal(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::SCIENTIFIC_LITERAL)
}
pub(crate) fn parse_float_decimal_start(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::FLOAT_DECIMAL_START)
}
pub(crate) fn parse_float_full(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::FLOAT_FULL)
}
pub(crate) fn parse_float_literal(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::FLOAT_LITERAL)
}
pub(crate) fn parse_integer_literal(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::INTEGER_LITERAL)
}
pub(crate) fn parse_typed_integer(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::TYPED_INTEGER)
}
pub(crate) fn parse_untyped_integer(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::UNTYPED_INTEGER)
}
pub(crate) fn parse_decimal_literal(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::DECIMAL_LITERAL)
}
pub(crate) fn parse_hexadecimal_literal(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::HEXADECIMAL_LITERAL)
}
pub(crate) fn parse_octal_literal(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::OCTAL_LITERAL)
}
pub(crate) fn parse_binary_literal(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::BINARY_LITERAL)
}

fn insert_missing_based_payload(
    parser: &mut Parser<'_>,
    payload: &str,
    code: &str,
    missing_token: Option<SyntaxKind>,
) {
    combinator::insert_missing(
        parser,
        code,
        &alloc::format!("expected {payload} after based-number prefix"),
        ExpectedSyntax::Production(String::from(payload)),
        missing_token,
        None,
    );
}

pub(super) fn insert_missing_raw_closer(parser: &mut Parser<'_>) {
    let at = parser.offset();
    let missing = parser.start();
    parser.missing_token(SyntaxKind::Quote);
    parser.missing_token(SyntaxKind::Quote);
    parser.missing_token(SyntaxKind::Quote);
    let missing = missing.complete_with_flags(parser, SyntaxKind::Missing, NodeFlags::MISSING);
    let expected = ExpectedSyntax::Production(String::from("triple closing quote"));
    let diagnostic = Diagnostic {
        id: parser.next_diagnostic_id(),
        code: DiagnosticCode::from("syntax/unclosed-raw-string"),
        phase: DiagnosticPhase::Syntax,
        severity: Severity::Error,
        rule: parser.current_rule(),
        context: parser.current_context(),
        primary: DiagnosticAnchor::Absolute {
            revision: parser.source().revision(),
            range: TextRange::empty(at),
        },
        labels: alloc::vec![],
        expected: alloc::vec![expected.clone()],
        found: Some(parser.found_syntax()),
        fixes: alloc::vec![DiagnosticFix {
            title: String::from("insert `\"\"\"`"),
            applicability: FixApplicability::MachineApplicable,
            edits: alloc::vec![TextEdit::insert(at, "\"\"\"")],
        }],
        related: alloc::vec![],
        recovery: Some(RecoveryAction::Insert {
            syntax: expected,
            at,
        }),
        tags: DiagnosticTags::NONE,
        message: String::from("expected a triple closing quote for raw string"),
    };
    parser.push_diagnostic(
        diagnostic,
        Some(missing.position()),
        TextRange::empty(TextSize::ZERO),
    );
}

pub(super) fn label_opening(parser: &mut Parser<'_>, opening: TextRange, message: &str) {
    let revision = parser.source().revision();
    if let Some(diagnostic) = parser.last_diagnostic_mut() {
        diagnostic.labels.push(DiagnosticLabel {
            anchor: DiagnosticAnchor::Absolute {
                revision,
                range: opening,
            },
            message: String::from(message),
        });
    }
}

/// A failed speculative literal can be rewound only while parsing may continue.
/// On resource exhaustion, retain its partial node and unwind the owner stack.
pub(super) fn failed_literal(
    parser: &mut Parser<'_>,
    marker: super::super::marker::Marker,
    kind: SyntaxKind,
) -> Attempt {
    if parser.is_halted() {
        marker.complete(parser, kind);
        Attempt::Committed
    } else {
        marker.abandon(parser);
        Attempt::NoMatch
    }
}
