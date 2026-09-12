use mech_syntax::document::ast::{ComplexNumberSyntax, RealNumberSyntax, UntypedRealNumberSyntax};
use mech_syntax::document::parser::canonical::parse_canonical_phase_2c_rule_for_test;
use mech_syntax::document::parser::rules;
use mech_syntax::document::{
    AstNode, DocumentId, ParseConfig, Revision, RuleId, SyntaxKind, SyntaxNode, TextSnapshot,
};

fn source(text: &str) -> TextSnapshot {
    TextSnapshot::new(DocumentId(923), Revision(0), text).unwrap()
}

fn find_node(root: &SyntaxNode, kind: SyntaxKind) -> Option<SyntaxNode> {
    if root.kind() == kind {
        return Some(root.clone());
    }
    root.children().find_map(|child| find_node(&child, kind))
}

fn parse_typed<N: AstNode>(input: &str, rule: RuleId, kind: SyntaxKind) -> N {
    let parsed =
        parse_canonical_phase_2c_rule_for_test(source(input), rule, ParseConfig::default())
            .unwrap();
    assert!(parsed.is_strictly_clean(), "{rule:?} on {input:?}");
    assert_eq!(parsed.consumed.end, parsed.source.byte_len(), "{input:?}");
    let node = find_node(&parsed.syntax(), kind)
        .unwrap_or_else(|| panic!("{rule:?} did not emit {kind:?} for {input:?}"));
    N::cast(node).unwrap_or_else(|| panic!("{kind:?} did not cast for {input:?}"))
}

fn component_text(value: Option<UntypedRealNumberSyntax>) -> Option<String> {
    value.map(|value| value.syntax().text().unwrap())
}

#[test]
fn complex_component_views_distinguish_pure_imaginary_and_two_component_shapes() {
    for (input, real, imaginary) in [
        ("2i", None, Some("2")),
        ("1+2i", Some("1"), Some("2")),
        ("1-2i", Some("1"), Some("2")),
        ("1+-2i", Some("1"), Some("-2")),
        ("1--2i", Some("1"), Some("-2")),
    ] {
        let complex = parse_typed::<ComplexNumberSyntax>(
            input,
            rules::COMPLEX_NUMBER,
            SyntaxKind::ComplexNumber,
        );
        assert_eq!(
            complex.components().len(),
            usize::from(real.is_some()) + 1,
            "{input:?}"
        );
        assert_eq!(component_text(complex.real()), real.map(str::to_owned));
        assert_eq!(
            component_text(complex.imaginary()),
            imaginary.map(str::to_owned),
            "{input:?}"
        );
    }
}

#[test]
fn real_number_negation_uses_only_the_direct_leading_dash() {
    for (input, expected) in [
        ("1.0e-3", false),
        ("-1.0e3", true),
        ("1", false),
        ("-1", true),
    ] {
        let number =
            parse_typed::<RealNumberSyntax>(input, rules::REAL_NUMBER, SyntaxKind::RealNumber);
        assert_eq!(number.is_negated(), expected, "{input:?}");
    }
}

#[test]
fn untyped_real_number_negation_uses_only_the_direct_leading_dash() {
    for (input, expected) in [
        ("1.0e-3", false),
        ("-1.0e3", true),
        ("1", false),
        ("-1", true),
    ] {
        let number = parse_typed::<UntypedRealNumberSyntax>(
            input,
            rules::UNTYPED_REAL_NUMBER,
            SyntaxKind::UntypedRealNumber,
        );
        assert_eq!(number.is_negated(), expected, "{input:?}");
    }
}
