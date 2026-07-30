use std::fmt::Debug;
use std::panic::{catch_unwind, AssertUnwindSafe};

use mech_syntax::document::ast::{
    AtomLiteralSyntax, BinaryLiteralSyntax, DecimalLiteralSyntax, EmptyLiteralSyntax,
    FloatDecimalStartSyntax, FloatFullSyntax, FloatLiteralSyntax, HexadecimalLiteralSyntax,
    IntegerLiteralSyntax, NumberSyntax, OctalLiteralSyntax, RationalLiteralSyntax, RawStringSyntax,
    RealNumberSyntax, ScientificLiteralSyntax, StringLiteralSyntax, TypedIntegerSyntax,
    UntypedIntegerSyntax, UntypedRealNumberSyntax, Utf8StringSyntax,
};
use mech_syntax::document::parser::canonical::parse_canonical_phase_2c_rule_for_test;
use mech_syntax::document::parser::rules;
use mech_syntax::document::{
    lower_legacy_atom, lower_legacy_binary_literal, lower_legacy_complex_number,
    lower_legacy_decimal_literal, lower_legacy_empty, lower_legacy_float_decimal_start,
    lower_legacy_float_full, lower_legacy_float_literal, lower_legacy_hexadecimal_literal,
    lower_legacy_integer_literal, lower_legacy_number, lower_legacy_octal_literal,
    lower_legacy_rational_literal, lower_legacy_raw_string, lower_legacy_real_number,
    lower_legacy_scientific_literal, lower_legacy_string, lower_legacy_typed_integer,
    lower_legacy_untyped_integer, lower_legacy_untyped_real_number, lower_legacy_utf8_string,
    AstNode, DocumentId, ParseConfig, Revision, RuleId, SyntaxKind, SyntaxNode, TextSnapshot,
};

fn source(text: &str) -> TextSnapshot {
    TextSnapshot::new(DocumentId(922), Revision(0), text).unwrap()
}

fn canonical_node(input: &str, rule: RuleId, kind: SyntaxKind) -> SyntaxNode {
    let parsed =
        parse_canonical_phase_2c_rule_for_test(source(input), rule, ParseConfig::default())
            .unwrap();
    assert!(parsed.is_strictly_clean(), "{rule:?} on {input:?}");
    assert_eq!(parsed.consumed.end, parsed.source.byte_len(), "{input:?}");
    find_node(&parsed.syntax(), kind)
        .unwrap_or_else(|| panic!("{rule:?} did not emit {kind:?} for {input:?}"))
}

fn find_node(root: &SyntaxNode, kind: SyntaxKind) -> Option<SyntaxNode> {
    if root.kind() == kind {
        return Some(root.clone());
    }
    root.children().find_map(|child| find_node(&child, kind))
}

fn legacy_value<Output>(
    input: &str,
    parser: for<'source> fn(
        mech_syntax::ParseString<'source>,
    ) -> mech_syntax::ParseResult<'source, Output>,
) -> Output {
    let graphemes = mech_syntax::graphemes::init_tag(input);
    let (remaining, value) = parser(mech_syntax::ParseString::new(&graphemes)).unwrap();
    assert_eq!(remaining.cursor, graphemes.len(), "{input:?}");
    assert!(remaining.error_log.is_empty(), "{input:?}");
    value
}

fn assert_exact<T: Debug + Eq>(canonical: T, legacy: T, input: &str) {
    assert_eq!(canonical, legacy, "{input:?}");
}

#[test]
fn primitive_literal_values_match_legacy_exactly() {
    for input in ["_", "___"] {
        let node = canonical_node(input, rules::EMPTY, SyntaxKind::EmptyLiteral);
        let canonical = lower_legacy_empty(&EmptyLiteralSyntax::cast(node).unwrap()).unwrap();
        assert_exact(canonical, legacy_value(input, mech_syntax::empty), input);
    }

    for input in [":atom", ":💡"] {
        let node = canonical_node(input, rules::ATOM, SyntaxKind::AtomLiteral);
        let canonical = lower_legacy_atom(&AtomLiteralSyntax::cast(node).unwrap()).unwrap();
        assert_exact(canonical, legacy_value(input, mech_syntax::atom), input);
    }

    for input in ["\"text\"", r#""escaped \n""#, "\"\"\"raw\\text\"\"\""] {
        let string = canonical_node(input, rules::STRING, SyntaxKind::StringLiteral);
        let canonical = lower_legacy_string(&StringLiteralSyntax::cast(string).unwrap()).unwrap();
        assert_exact(canonical, legacy_value(input, mech_syntax::string), input);
    }

    for input in ["\"text\"", "\"\""] {
        let node = canonical_node(input, rules::UTF8_STRING, SyntaxKind::Utf8String);
        let canonical = lower_legacy_utf8_string(&Utf8StringSyntax::cast(node).unwrap()).unwrap();
        assert_exact(
            canonical,
            legacy_value(input, mech_syntax::utf8_string),
            input,
        );
    }

    for input in ["\"\"\"raw\"\"\"", "\"\"\"\"\"\""] {
        let node = canonical_node(input, rules::RAW_STRING, SyntaxKind::RawString);
        let canonical = lower_legacy_raw_string(&RawStringSyntax::cast(node).unwrap()).unwrap();
        assert_exact(
            canonical,
            legacy_value(input, mech_syntax::raw_string),
            input,
        );
    }
}

#[test]
fn ordinary_number_values_match_legacy_exactly() {
    for input in [
        "1", "1u8", "1.0", ".5", "1/2", "0d12", "0xG_", "0o9", "0b9", "-1",
    ] {
        let node = canonical_node(input, rules::NUMBER, SyntaxKind::Number);
        let canonical = lower_legacy_number(&NumberSyntax::cast(node).unwrap()).unwrap();
        assert_exact(canonical, legacy_value(input, mech_syntax::number), input);
    }

    for input in ["2i", "1+2i", "1-2i", "1+-2i", "1--2i"] {
        let node = canonical_node(input, rules::COMPLEX_NUMBER, SyntaxKind::ComplexNumber);
        let canonical = lower_legacy_complex_number(
            &mech_syntax::document::ast::ComplexNumberSyntax::cast(node).unwrap(),
        )
        .unwrap();
        assert_exact(
            canonical,
            legacy_value(input, mech_syntax::complex_number),
            input,
        );
    }

    for input in ["1", "-1", "0xFF", "1.5", "1/2"] {
        let node = canonical_node(input, rules::REAL_NUMBER, SyntaxKind::RealNumber);
        let canonical = lower_legacy_real_number(&RealNumberSyntax::cast(node).unwrap()).unwrap();
        assert_exact(
            canonical,
            legacy_value(input, mech_syntax::real_number),
            input,
        );
    }

    for input in ["1", "-1", "0o9", ".5", "1/2"] {
        let node = canonical_node(
            input,
            rules::UNTYPED_REAL_NUMBER,
            SyntaxKind::UntypedRealNumber,
        );
        let canonical =
            lower_legacy_untyped_real_number(&UntypedRealNumberSyntax::cast(node).unwrap())
                .unwrap();
        assert_exact(
            canonical,
            legacy_value(input, mech_syntax::untyped_real_number),
            input,
        );
    }
}

#[test]
fn direct_numeric_lowerers_match_legacy_on_their_ordinary_domains() {
    for input in ["1/2"] {
        let node = canonical_node(input, rules::RATIONAL_LITERAL, SyntaxKind::RationalLiteral);
        let canonical =
            lower_legacy_rational_literal(&RationalLiteralSyntax::cast(node).unwrap()).unwrap();
        assert_exact(
            canonical,
            legacy_value(input, mech_syntax::rational_literal),
            input,
        );
    }

    for input in ["1.0e3", "1.0e+3", "1.0e-3", "1.0e+-3"] {
        let node = canonical_node(
            input,
            rules::SCIENTIFIC_LITERAL,
            SyntaxKind::ScientificLiteral,
        );
        let canonical =
            lower_legacy_scientific_literal(&ScientificLiteralSyntax::cast(node).unwrap()).unwrap();
        assert_exact(
            canonical,
            legacy_value(input, mech_syntax::scientific_literal),
            input,
        );
    }

    for input in [".5"] {
        let node = canonical_node(
            input,
            rules::FLOAT_DECIMAL_START,
            SyntaxKind::FloatDecimalStart,
        );
        let canonical =
            lower_legacy_float_decimal_start(&FloatDecimalStartSyntax::cast(node).unwrap())
                .unwrap();
        assert_exact(
            canonical,
            legacy_value(input, mech_syntax::float_decimal_start),
            input,
        );
    }
    for input in ["1.5"] {
        let node = canonical_node(input, rules::FLOAT_FULL, SyntaxKind::FloatFull);
        let canonical = lower_legacy_float_full(&FloatFullSyntax::cast(node).unwrap()).unwrap();
        assert_exact(
            canonical,
            legacy_value(input, mech_syntax::float_full),
            input,
        );
    }
    for input in [".5", "1.5"] {
        let node = canonical_node(input, rules::FLOAT_LITERAL, SyntaxKind::FloatLiteral);
        let canonical =
            lower_legacy_float_literal(&FloatLiteralSyntax::cast(node).unwrap()).unwrap();
        assert_exact(
            canonical,
            legacy_value(input, mech_syntax::float_literal),
            input,
        );
    }
    for input in ["1u8", "1"] {
        let node = canonical_node(input, rules::INTEGER_LITERAL, SyntaxKind::IntegerLiteral);
        let canonical =
            lower_legacy_integer_literal(&IntegerLiteralSyntax::cast(node).unwrap()).unwrap();
        assert_exact(
            canonical,
            legacy_value(input, mech_syntax::integer_literal),
            input,
        );
    }
    for input in ["1u8", "1foo"] {
        let node = canonical_node(input, rules::TYPED_INTEGER, SyntaxKind::TypedInteger);
        let canonical =
            lower_legacy_typed_integer(&TypedIntegerSyntax::cast(node).unwrap()).unwrap();
        assert_exact(
            canonical,
            legacy_value(input, mech_syntax::typed_integer),
            input,
        );
    }
    for input in ["1", "1_000"] {
        let node = canonical_node(input, rules::UNTYPED_INTEGER, SyntaxKind::UntypedInteger);
        let canonical =
            lower_legacy_untyped_integer(&UntypedIntegerSyntax::cast(node).unwrap()).unwrap();
        assert_exact(
            canonical,
            legacy_value(input, mech_syntax::untyped_integer),
            input,
        );
    }
    for input in ["0d12", "0d٣"] {
        let node = canonical_node(input, rules::DECIMAL_LITERAL, SyntaxKind::DecimalLiteral);
        let canonical =
            lower_legacy_decimal_literal(&DecimalLiteralSyntax::cast(node).unwrap()).unwrap();
        assert_exact(
            canonical,
            legacy_value(input, mech_syntax::decimal_literal),
            input,
        );
    }
    for input in ["0xFF", "0xG_"] {
        let node = canonical_node(
            input,
            rules::HEXADECIMAL_LITERAL,
            SyntaxKind::HexadecimalLiteral,
        );
        let canonical =
            lower_legacy_hexadecimal_literal(&HexadecimalLiteralSyntax::cast(node).unwrap())
                .unwrap();
        assert_exact(
            canonical,
            legacy_value(input, mech_syntax::hexadecimal_literal),
            input,
        );
    }
    for input in ["0o9"] {
        let node = canonical_node(input, rules::OCTAL_LITERAL, SyntaxKind::OctalLiteral);
        let canonical =
            lower_legacy_octal_literal(&OctalLiteralSyntax::cast(node).unwrap()).unwrap();
        assert_exact(
            canonical,
            legacy_value(input, mech_syntax::octal_literal),
            input,
        );
    }
    for input in ["0b9"] {
        let node = canonical_node(input, rules::BINARY_LITERAL, SyntaxKind::BinaryLiteral);
        let canonical =
            lower_legacy_binary_literal(&BinaryLiteralSyntax::cast(node).unwrap()).unwrap();
        assert_exact(
            canonical,
            legacy_value(input, mech_syntax::binary_literal),
            input,
        );
    }
}

#[test]
fn typed_rational_components_discard_kind_annotations() {
    let input = "1u8/2u16";
    let node = canonical_node(input, rules::RATIONAL_LITERAL, SyntaxKind::RationalLiteral);
    let canonical =
        lower_legacy_rational_literal(&RationalLiteralSyntax::cast(node).unwrap()).unwrap();

    let mech_core::RealNumber::Rational((numerator, denominator)) = canonical else {
        panic!("expected canonical typed rational components to lower as Rational")
    };
    assert_eq!(numerator.to_string(), "1");
    assert_eq!(denominator.to_string(), "2");
}

#[test]
fn typed_scientific_exponent_is_lowered_deterministically_without_a_panic() {
    let input = "1.0e3u8";
    let node = canonical_node(
        input,
        rules::SCIENTIFIC_LITERAL,
        SyntaxKind::ScientificLiteral,
    );
    let syntax = ScientificLiteralSyntax::cast(node).unwrap();
    let first = lower_legacy_scientific_literal(&syntax).unwrap();
    let second = lower_legacy_scientific_literal(&syntax).unwrap();
    assert_eq!(first, second);

    let legacy = catch_unwind(AssertUnwindSafe(|| {
        legacy_value(input, mech_syntax::scientific_literal)
    }));
    assert!(
        legacy.is_err(),
        "the historical typed-exponent path must panic"
    );
}
