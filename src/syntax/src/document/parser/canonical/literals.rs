//! Canonical literal and number productions for the Phase 2C closed island.
//!
//! This module intentionally ports only the direct literal children selected
//! for Phase 2C.  The enclosing `literal` production remains absent because
//! its optional kind annotation reaches the still-unported recursive closure.

use alloc::string::String;

use crate::document::{
    Diagnostic, DiagnosticAnchor, DiagnosticCode, DiagnosticFix, DiagnosticLabel, DiagnosticPhase,
    DiagnosticTags, ExpectedSyntax, FixApplicability, NodeFlags, RecoveryAction, Severity,
    SyntaxKind, TextEdit, TextRange, TextSize,
};

use super::super::rule::rules;
use super::super::Parser;
use super::base;
use super::combinator::{self, Attempt};

macro_rules! first_accepted {
    ($parser:expr, $($parse:path),+ $(,)?) => {{
        let mut result = Attempt::NoMatch;
        $(
            if result == Attempt::NoMatch {
                result = $parse($parser);
            }
        )+
        result
    }};
}

/// Parse one or more underscore tokens as an `empty` literal.
pub(crate) fn parse_empty(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::EMPTY, |parser| {
        let empty = parser.start();
        if !base::parse_rule(parser, rules::UNDERSCORE) {
            empty.abandon(parser);
            return Attempt::NoMatch;
        }

        while base::parse_rule(parser, rules::UNDERSCORE) {
            if parser.is_halted() {
                break;
            }
        }

        empty.complete(parser, SyntaxKind::EmptyLiteral);
        Attempt::Matched
    })
}

/// Parse the closed `:identifier` atom production.
///
/// A bare colon is deliberately a noncommitting candidate.  The full literal
/// dispatcher is not present in this phase, so speculative recovery would
/// incorrectly leak into later parent alternatives.
pub(crate) fn parse_atom(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::ATOM, |parser| {
        let atom = parser.start();
        if !base::parse_rule(parser, rules::COLON) || !base::parse_rule(parser, rules::IDENTIFIER) {
            atom.abandon(parser);
            return Attempt::NoMatch;
        }
        atom.complete(parser, SyntaxKind::AtomLiteral);
        Attempt::Matched
    })
}

/// Parse `string` using complete candidates before attempting recovery.
///
/// The order is important: an incomplete raw candidate must be entirely
/// rewound before a valid UTF-8 prefix is considered.
pub(crate) fn parse_string(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::STRING, |parser| {
        let string = parser.start();

        if parse_raw_string_candidate(parser).accepted()
            || parse_utf8_string_candidate(parser).accepted()
        {
            string.complete(parser, SyntaxKind::StringLiteral);
            return Attempt::Matched;
        }

        if !starts_with_exact(parser, "\"") {
            string.abandon(parser);
            return Attempt::NoMatch;
        }

        let recovered = parse_utf8_string_recovery(parser);
        debug_assert!(recovered.accepted());
        string.complete(parser, SyntaxKind::StringLiteral);
        recovered
    })
}

/// Parse the exact `utf8-string` production, including its direct-rule
/// recovery for an unclosed delimiter.
pub(crate) fn parse_utf8_string(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::UTF8_STRING, |parser| {
        let candidate = parse_utf8_string_candidate(parser);
        if candidate.accepted() {
            return candidate;
        }
        if !starts_with_exact(parser, "\"") {
            return Attempt::NoMatch;
        }
        parse_utf8_string_recovery(parser)
    })
}

/// Parse the exact `raw-string` production, including its direct-rule
/// recovery for an unclosed triple delimiter.
pub(crate) fn parse_raw_string(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::RAW_STRING, |parser| {
        let candidate = parse_raw_string_candidate(parser);
        if candidate.accepted() {
            return candidate;
        }
        if !starts_with_exact(parser, "\"\"\"") {
            return Attempt::NoMatch;
        }
        parse_raw_string_recovery(parser)
    })
}

/// A complete UTF-8 string candidate with no recovery side effects.
fn parse_utf8_string_candidate(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::UTF8_STRING, |parser| {
        let string = parser.start();
        if !base::parse_rule(parser, rules::QUOTE) {
            string.abandon(parser);
            return Attempt::NoMatch;
        }

        while !parser.is_eof() && !starts_with_exact(parser, "\"") {
            let before = parser.offset();
            if !base::parse_rule(parser, rules::TEXT) && !base::parse_rule(parser, rules::NEW_LINE)
            {
                break;
            }
            if parser.offset() == before || parser.is_halted() {
                break;
            }
        }

        if !base::parse_rule(parser, rules::QUOTE) {
            string.abandon(parser);
            return Attempt::NoMatch;
        }

        string.complete(parser, SyntaxKind::Utf8String);
        Attempt::Matched
    })
}

/// A complete raw string candidate with no recovery side effects.
fn parse_raw_string_candidate(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::RAW_STRING, |parser| {
        let string = parser.start();
        if !consume_quotes(parser, 3) {
            string.abandon(parser);
            return Attempt::NoMatch;
        }

        while !parser.is_eof() && !starts_with_exact(parser, "\"\"\"") {
            let before = parser.offset();
            if !base::parse_rule(parser, rules::RAW_TEXT)
                && !base::parse_rule(parser, rules::NEW_LINE)
            {
                break;
            }
            if parser.offset() == before || parser.is_halted() {
                break;
            }
        }

        if !consume_quotes(parser, 3) {
            string.abandon(parser);
            return Attempt::NoMatch;
        }

        string.complete(parser, SyntaxKind::RawString);
        Attempt::Matched
    })
}

fn parse_utf8_string_recovery(parser: &mut Parser<'_>) -> Attempt {
    let string = parser.start();
    let opening_start = parser.offset();
    if !base::parse_rule(parser, rules::QUOTE) {
        string.abandon(parser);
        return Attempt::NoMatch;
    }
    let opening = TextRange::new(opening_start, parser.offset());

    while !parser.is_eof() && !starts_with_exact(parser, "\"") {
        let before = parser.offset();
        if !base::parse_rule(parser, rules::TEXT) && !base::parse_rule(parser, rules::NEW_LINE) {
            break;
        }
        if parser.offset() == before || parser.is_halted() {
            break;
        }
    }

    combinator::insert_missing(
        parser,
        "syntax/unclosed-utf8-string",
        "expected a closing quote for UTF-8 string",
        ExpectedSyntax::Token(SyntaxKind::Quote),
        Some(SyntaxKind::Quote),
        Some("\""),
    );
    label_opening(parser, opening, "opening quote is here");
    string.complete(parser, SyntaxKind::Utf8String);
    Attempt::Committed
}

fn parse_raw_string_recovery(parser: &mut Parser<'_>) -> Attempt {
    let string = parser.start();
    let opening_start = parser.offset();
    if !consume_quotes(parser, 3) {
        string.abandon(parser);
        return Attempt::NoMatch;
    }
    let opening = TextRange::new(opening_start, parser.offset());

    while !parser.is_eof() && !starts_with_exact(parser, "\"\"\"") {
        let before = parser.offset();
        if !base::parse_rule(parser, rules::RAW_TEXT) && !base::parse_rule(parser, rules::NEW_LINE)
        {
            break;
        }
        if parser.offset() == before || parser.is_halted() {
            break;
        }
    }

    insert_missing_raw_closer(parser);
    label_opening(parser, opening, "raw string starts here");
    string.complete(parser, SyntaxKind::RawString);
    Attempt::Committed
}

/// Parse the transparent `boolean` production.
pub(crate) fn parse_boolean(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::BOOLEAN, |parser| {
        first_accepted!(parser, parse_true_literal, parse_false_literal)
    })
}

/// Parse the transparent `true-literal` production.
pub(crate) fn parse_true_literal(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::TRUE_LITERAL, |parser| {
        if base::parse_rule(parser, rules::ENGLISH_TRUE_LITERAL)
            || base::parse_rule(parser, rules::CHECK_MARK)
        {
            Attempt::Matched
        } else {
            Attempt::NoMatch
        }
    })
}

/// Parse the transparent `false-literal` production.
pub(crate) fn parse_false_literal(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::FALSE_LITERAL, |parser| {
        if base::parse_rule(parser, rules::ENGLISH_FALSE_LITERAL)
            || base::parse_rule(parser, rules::CROSS)
        {
            Attempt::Matched
        } else {
            Attempt::NoMatch
        }
    })
}

/// Parse the closed `number` production.
pub(crate) fn parse_number(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::NUMBER, |parser| {
        let number = parser.start();
        let result = first_accepted!(parser, parse_complex_number, parse_real_number);
        if result == Attempt::NoMatch {
            number.abandon(parser);
            return Attempt::NoMatch;
        }
        number.complete(parser, SyntaxKind::Number);
        result
    })
}

/// Parse the closed complex-number candidate.
///
/// This production deliberately has no recovery: if its required imaginary
/// suffix is absent, the complete candidate is rewound and `real-number` gets
/// to select the same prefix.
pub(crate) fn parse_complex_number(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::COMPLEX_NUMBER, |parser| {
        let complex = parser.start();
        if parse_untyped_real_number_candidate(parser) != Attempt::Matched {
            complex.abandon(parser);
            return Attempt::NoMatch;
        }

        if consume_imaginary_unit(parser) {
            complex.complete(parser, SyntaxKind::ComplexNumber);
            return Attempt::Matched;
        }

        let has_sign =
            base::parse_rule(parser, rules::PLUS) || base::parse_rule(parser, rules::DASH);
        if has_sign
            && parse_untyped_real_number_candidate(parser) == Attempt::Matched
            && consume_imaginary_unit(parser)
        {
            complex.complete(parser, SyntaxKind::ComplexNumber);
            return Attempt::Matched;
        }

        complex.abandon(parser);
        Attempt::NoMatch
    })
}

/// Parse a real number, preserving a leading dash as syntax.
pub(crate) fn parse_real_number(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::REAL_NUMBER, |parser| {
        parse_real_number_inner(parser, false, true)
    })
}

/// Parse the restricted real-number form used by complex components.
pub(crate) fn parse_untyped_real_number(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::UNTYPED_REAL_NUMBER, |parser| {
        parse_real_number_inner(parser, true, true)
    })
}

fn parse_untyped_real_number_candidate(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::UNTYPED_REAL_NUMBER, |parser| {
        parse_real_number_inner(parser, true, false)
    })
}

fn parse_real_number_inner(
    parser: &mut Parser<'_>,
    untyped: bool,
    recover_based_prefix: bool,
) -> Attempt {
    let number = parser.start();
    let _ = base::parse_rule(parser, rules::DASH);

    let result = if untyped {
        if recover_based_prefix {
            first_accepted!(
                parser,
                parse_hexadecimal_literal,
                parse_decimal_literal,
                parse_octal_literal,
                parse_binary_literal,
                parse_scientific_literal,
                parse_rational_literal,
                parse_float_literal,
                parse_untyped_integer,
            )
        } else {
            first_accepted!(
                parser,
                parse_hexadecimal_literal_candidate,
                parse_decimal_literal_candidate,
                parse_octal_literal_candidate,
                parse_binary_literal_candidate,
                parse_scientific_literal,
                parse_rational_literal,
                parse_float_literal,
                parse_untyped_integer,
            )
        }
    } else if recover_based_prefix {
        first_accepted!(
            parser,
            parse_hexadecimal_literal,
            parse_decimal_literal,
            parse_octal_literal,
            parse_binary_literal,
            parse_scientific_literal,
            parse_rational_literal,
            parse_float_literal,
            parse_integer_literal,
        )
    } else {
        first_accepted!(
            parser,
            parse_hexadecimal_literal_candidate,
            parse_decimal_literal_candidate,
            parse_octal_literal_candidate,
            parse_binary_literal_candidate,
            parse_scientific_literal,
            parse_rational_literal,
            parse_float_literal,
            parse_integer_literal,
        )
    };

    if result == Attempt::NoMatch {
        number.abandon(parser);
        return Attempt::NoMatch;
    }

    number.complete(
        parser,
        if untyped {
            SyntaxKind::UntypedRealNumber
        } else {
            SyntaxKind::RealNumber
        },
    );
    result
}

/// Parse a tight integer slash integer rational candidate.
pub(crate) fn parse_rational_literal(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::RATIONAL_LITERAL, |parser| {
        let rational = parser.start();
        if !parse_integer_literal(parser).accepted()
            || !base::parse_rule(parser, rules::SLASH)
            || !parse_integer_literal(parser).accepted()
        {
            rational.abandon(parser);
            return Attempt::NoMatch;
        }
        rational.complete(parser, SyntaxKind::RationalLiteral);
        Attempt::Matched
    })
}

/// Parse a canonical scientific literal.
pub(crate) fn parse_scientific_literal(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::SCIENTIFIC_LITERAL, |parser| {
        let scientific = parser.start();
        if !first_accepted!(parser, parse_float_literal, parse_integer_literal).accepted()
            || !consume_scientific_marker(parser)
        {
            scientific.abandon(parser);
            return Attempt::NoMatch;
        }

        let _ = base::parse_rule(parser, rules::PLUS);
        let _ = base::parse_rule(parser, rules::DASH);

        if !first_accepted!(parser, parse_float_literal, parse_integer_literal).accepted() {
            scientific.abandon(parser);
            return Attempt::NoMatch;
        }

        scientific.complete(parser, SyntaxKind::ScientificLiteral);
        Attempt::Matched
    })
}

/// Parse `.digits`.
pub(crate) fn parse_float_decimal_start(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::FLOAT_DECIMAL_START, |parser| {
        let float = parser.start();
        if !base::parse_rule(parser, rules::PERIOD)
            || !base::parse_rule(parser, rules::DIGIT_SEQUENCE)
        {
            float.abandon(parser);
            return Attempt::NoMatch;
        }
        float.complete(parser, SyntaxKind::FloatDecimalStart);
        Attempt::Matched
    })
}

/// Parse `digits.digits`.
pub(crate) fn parse_float_full(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::FLOAT_FULL, |parser| {
        let float = parser.start();
        if !base::parse_rule(parser, rules::DIGIT_SEQUENCE)
            || !base::parse_rule(parser, rules::PERIOD)
            || !base::parse_rule(parser, rules::DIGIT_SEQUENCE)
        {
            float.abandon(parser);
            return Attempt::NoMatch;
        }
        float.complete(parser, SyntaxKind::FloatFull);
        Attempt::Matched
    })
}

/// Parse either closed float spelling in the canonical order.
pub(crate) fn parse_float_literal(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::FLOAT_LITERAL, |parser| {
        let float = parser.start();
        let result = first_accepted!(parser, parse_float_decimal_start, parse_float_full);
        if result == Attempt::NoMatch {
            float.abandon(parser);
            return Attempt::NoMatch;
        }
        float.complete(parser, SyntaxKind::FloatLiteral);
        result
    })
}

/// Parse a typed integer before considering an untyped one.
pub(crate) fn parse_integer_literal(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::INTEGER_LITERAL, |parser| {
        let integer = parser.start();
        let result = first_accepted!(parser, parse_typed_integer, parse_untyped_integer);
        if result == Attempt::NoMatch {
            integer.abandon(parser);
            return Attempt::NoMatch;
        }
        integer.complete(parser, SyntaxKind::IntegerLiteral);
        result
    })
}

/// Parse `digit-sequence identifier` without semantic suffix validation.
pub(crate) fn parse_typed_integer(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::TYPED_INTEGER, |parser| {
        let integer = parser.start();
        if !base::parse_rule(parser, rules::DIGIT_SEQUENCE)
            || !base::parse_rule(parser, rules::IDENTIFIER)
        {
            integer.abandon(parser);
            return Attempt::NoMatch;
        }
        integer.complete(parser, SyntaxKind::TypedInteger);
        Attempt::Matched
    })
}

/// Parse a bare `digit-sequence` integer.
pub(crate) fn parse_untyped_integer(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::UNTYPED_INTEGER, |parser| {
        let integer = parser.start();
        if !base::parse_rule(parser, rules::DIGIT_SEQUENCE) {
            integer.abandon(parser);
            return Attempt::NoMatch;
        }
        integer.complete(parser, SyntaxKind::UntypedInteger);
        Attempt::Matched
    })
}

/// Parse `0d` followed by a digit sequence.
pub(crate) fn parse_decimal_literal(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::DECIMAL_LITERAL, |parser| {
        parse_decimal_literal_inner(parser, true)
    })
}

fn parse_decimal_literal_candidate(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::DECIMAL_LITERAL, |parser| {
        parse_decimal_literal_inner(parser, false)
    })
}

fn parse_decimal_literal_inner(parser: &mut Parser<'_>, recover: bool) -> Attempt {
    let decimal = parser.start();
    if !consume_anonymous_literal(parser, "0d") {
        decimal.abandon(parser);
        return Attempt::NoMatch;
    }
    if !base::parse_rule(parser, rules::DIGIT_SEQUENCE) {
        if !recover {
            decimal.abandon(parser);
            return Attempt::NoMatch;
        }
        insert_missing_based_payload(
            parser,
            "decimal digits",
            "syntax/missing-decimal-digits",
            Some(SyntaxKind::Digit),
        );
        decimal.complete(parser, SyntaxKind::DecimalLiteral);
        return Attempt::Committed;
    }
    decimal.complete(parser, SyntaxKind::DecimalLiteral);
    Attempt::Matched
}

/// Parse `0x` followed by one or more digit, underscore, or alpha tokens.
pub(crate) fn parse_hexadecimal_literal(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::HEXADECIMAL_LITERAL, |parser| {
        parse_hexadecimal_literal_inner(parser, true)
    })
}

fn parse_hexadecimal_literal_candidate(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::HEXADECIMAL_LITERAL, |parser| {
        parse_hexadecimal_literal_inner(parser, false)
    })
}

fn parse_hexadecimal_literal_inner(parser: &mut Parser<'_>, recover: bool) -> Attempt {
    let hexadecimal = parser.start();
    if !consume_anonymous_literal(parser, "0x") {
        hexadecimal.abandon(parser);
        return Attempt::NoMatch;
    }

    let mut payload = 0_u32;
    loop {
        let before = parser.offset();
        if !(base::parse_rule(parser, rules::DIGIT_TOKEN)
            || base::parse_rule(parser, rules::UNDERSCORE)
            || base::parse_rule(parser, rules::ALPHA_TOKEN))
        {
            break;
        }
        payload = payload.saturating_add(1);
        if parser.offset() == before || parser.is_halted() {
            break;
        }
    }

    if payload == 0 {
        if !recover {
            hexadecimal.abandon(parser);
            return Attempt::NoMatch;
        }
        insert_missing_based_payload(
            parser,
            "hexadecimal digits",
            "syntax/missing-hexadecimal-digits",
            None,
        );
        hexadecimal.complete(parser, SyntaxKind::HexadecimalLiteral);
        return Attempt::Committed;
    }

    hexadecimal.complete(parser, SyntaxKind::HexadecimalLiteral);
    Attempt::Matched
}

/// Parse `0o` followed by a digit sequence.
pub(crate) fn parse_octal_literal(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::OCTAL_LITERAL, |parser| {
        parse_octal_literal_inner(parser, true)
    })
}

fn parse_octal_literal_candidate(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::OCTAL_LITERAL, |parser| {
        parse_octal_literal_inner(parser, false)
    })
}

fn parse_octal_literal_inner(parser: &mut Parser<'_>, recover: bool) -> Attempt {
    let octal = parser.start();
    if !consume_anonymous_literal(parser, "0o") {
        octal.abandon(parser);
        return Attempt::NoMatch;
    }
    if !base::parse_rule(parser, rules::DIGIT_SEQUENCE) {
        if !recover {
            octal.abandon(parser);
            return Attempt::NoMatch;
        }
        insert_missing_based_payload(
            parser,
            "octal digits",
            "syntax/missing-octal-digits",
            Some(SyntaxKind::Digit),
        );
        octal.complete(parser, SyntaxKind::OctalLiteral);
        return Attempt::Committed;
    }
    octal.complete(parser, SyntaxKind::OctalLiteral);
    Attempt::Matched
}

/// Parse `0b` followed by a digit sequence.
pub(crate) fn parse_binary_literal(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::BINARY_LITERAL, |parser| {
        parse_binary_literal_inner(parser, true)
    })
}

fn parse_binary_literal_candidate(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::BINARY_LITERAL, |parser| {
        parse_binary_literal_inner(parser, false)
    })
}

fn parse_binary_literal_inner(parser: &mut Parser<'_>, recover: bool) -> Attempt {
    let binary = parser.start();
    if !consume_anonymous_literal(parser, "0b") {
        binary.abandon(parser);
        return Attempt::NoMatch;
    }
    if !base::parse_rule(parser, rules::DIGIT_SEQUENCE) {
        if !recover {
            binary.abandon(parser);
            return Attempt::NoMatch;
        }
        insert_missing_based_payload(
            parser,
            "binary digits",
            "syntax/missing-binary-digits",
            Some(SyntaxKind::Digit),
        );
        binary.complete(parser, SyntaxKind::BinaryLiteral);
        return Attempt::Committed;
    }
    binary.complete(parser, SyntaxKind::BinaryLiteral);
    Attempt::Matched
}

fn consume_anonymous_literal(parser: &mut Parser<'_>, literal: &str) -> bool {
    base::parse_exact_tag(parser, literal, SyntaxKind::Text)
}

fn consume_scientific_marker(parser: &mut Parser<'_>) -> bool {
    consume_anonymous_literal(parser, "e") || consume_anonymous_literal(parser, "E")
}

fn consume_imaginary_unit(parser: &mut Parser<'_>) -> bool {
    consume_anonymous_literal(parser, "i") || consume_anonymous_literal(parser, "j")
}

fn starts_with_exact(parser: &Parser<'_>, literal: &str) -> bool {
    parser.cursor().grapheme_literal_end(literal).is_some()
}

fn consume_quotes(parser: &mut Parser<'_>, count: usize) -> bool {
    for _ in 0..count {
        if !base::parse_rule(parser, rules::QUOTE) {
            return false;
        }
    }
    true
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

fn insert_missing_raw_closer(parser: &mut Parser<'_>) {
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

fn label_opening(parser: &mut Parser<'_>, opening: TextRange, message: &str) {
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
