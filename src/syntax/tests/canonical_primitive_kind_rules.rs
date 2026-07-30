use std::fmt::Debug;

use mech_core::nodes::Kind;
use mech_syntax::document::ast::kinds::{KindAnySyntax, KindAtomSyntax, KindEmptySyntax};
use mech_syntax::document::parser::canonical::parse_canonical_phase_2c_rule_for_test;
use mech_syntax::document::parser::rules;
use mech_syntax::document::{
    lower_legacy_kind_any, lower_legacy_kind_atom, lower_legacy_kind_empty,
    reconstruct_source_range, AstNode, DocumentId, ParseConfig, Revision, RuleId, SyntaxKind,
    SyntaxNode, TextRange, TextSize, TextSnapshot,
};

fn source(text: &str) -> TextSnapshot {
    TextSnapshot::new(DocumentId(924), Revision(0), text).unwrap()
}

fn parse(
    text: &str,
    rule: RuleId,
) -> mech_syntax::document::parser::canonical::CanonicalSourceRuleSnapshot {
    parse_canonical_phase_2c_rule_for_test(source(text), rule, ParseConfig::default())
        .unwrap_or_else(|| panic!("{rule:?} is not a Phase 2C direct rule"))
}

fn find_node(root: &SyntaxNode, kind: SyntaxKind) -> Option<SyntaxNode> {
    if root.kind() == kind {
        return Some(root.clone());
    }
    root.children().find_map(|child| find_node(&child, kind))
}

fn canonical_node(input: &str, rule: RuleId, kind: SyntaxKind) -> SyntaxNode {
    let parsed = parse(input, rule);
    assert!(parsed.is_strictly_clean(), "{rule:?} on {input:?}");
    assert_eq!(parsed.consumed.end, parsed.source.byte_len(), "{input:?}");
    assert_eq!(
        reconstruct_source_range(&parsed.root, &parsed.source, parsed.consumed).unwrap(),
        input,
        "{input:?}",
    );
    find_node(&parsed.syntax(), kind)
        .unwrap_or_else(|| panic!("{rule:?} did not emit {kind:?} for {input:?}"))
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

fn assert_exact_legacy_value<T>(canonical: T, legacy: T, input: &str)
where
    T: Debug + Eq,
{
    assert_eq!(canonical, legacy, "{input:?}");
}

#[test]
fn every_primitive_kind_rule_has_a_clean_direct_contract() {
    for (rule, input, kind) in [
        (rules::KIND_ANY, "*", SyntaxKind::KindAny),
        (rules::KIND_EMPTY, "_", SyntaxKind::KindEmpty),
        (rules::KIND_EMPTY, "___", SyntaxKind::KindEmpty),
        (rules::KIND_ATOM, ":status", SyntaxKind::KindAtom),
        (rules::KIND_ATOM, ":💡", SyntaxKind::KindAtom),
    ] {
        let node = canonical_node(input, rule, kind);
        assert_eq!(node.text().unwrap(), input, "{input:?}");
    }
}

#[test]
fn primitive_kind_lowerers_match_legacy_values_exactly() {
    let any = canonical_node("*", rules::KIND_ANY, SyntaxKind::KindAny);
    let canonical = lower_legacy_kind_any(&KindAnySyntax::cast(any).unwrap()).unwrap();
    let legacy = legacy_value("*", mech_syntax::kind_any);
    assert_exact_legacy_value(canonical, legacy, "*");

    for input in ["_", "___"] {
        let empty = canonical_node(input, rules::KIND_EMPTY, SyntaxKind::KindEmpty);
        let canonical = lower_legacy_kind_empty(&KindEmptySyntax::cast(empty).unwrap()).unwrap();
        let legacy = legacy_value(input, mech_syntax::kind_empty);
        assert_exact_legacy_value(canonical, legacy, input);
    }

    for input in [":status", ":💡"] {
        let atom = canonical_node(input, rules::KIND_ATOM, SyntaxKind::KindAtom);
        let canonical = lower_legacy_kind_atom(&KindAtomSyntax::cast(atom).unwrap()).unwrap();
        let legacy = legacy_value(input, mech_syntax::kind_atom);
        assert_exact_legacy_value(canonical, legacy, input);
    }
}

#[test]
fn primitive_kind_views_expose_their_meaningful_children() {
    let any =
        KindAnySyntax::cast(canonical_node("*", rules::KIND_ANY, SyntaxKind::KindAny)).unwrap();
    assert_eq!(any.asterisk().unwrap().text().unwrap(), "*");

    let empty = KindEmptySyntax::cast(canonical_node(
        "___",
        rules::KIND_EMPTY,
        SyntaxKind::KindEmpty,
    ))
    .unwrap();
    assert_eq!(empty.underscores().len(), 3);

    let atom = KindAtomSyntax::cast(canonical_node(
        ":status",
        rules::KIND_ATOM,
        SyntaxKind::KindAtom,
    ))
    .unwrap();
    assert_eq!(atom.name().unwrap().syntax().text().unwrap(), "status");
}

#[test]
fn primitive_kind_candidates_rewind_cleanly_when_incomplete() {
    for (rule, input) in [
        (rules::KIND_ANY, "_"),
        (rules::KIND_EMPTY, ""),
        (rules::KIND_ATOM, ":"),
    ] {
        let parsed = parse(input, rule);
        assert!(!parsed.matched, "{rule:?} on {input:?}");
        assert!(parsed.diagnostics.is_empty(), "{rule:?} on {input:?}");
        assert_eq!(
            parsed.consumed,
            TextRange::empty(TextSize::ZERO),
            "{input:?}",
        );
    }
}

#[test]
fn primitive_kind_variants_are_not_a_temporary_parent_dispatcher() {
    let any = lower_legacy_kind_any(
        &KindAnySyntax::cast(canonical_node("*", rules::KIND_ANY, SyntaxKind::KindAny)).unwrap(),
    )
    .unwrap();
    let empty = lower_legacy_kind_empty(
        &KindEmptySyntax::cast(canonical_node(
            "_",
            rules::KIND_EMPTY,
            SyntaxKind::KindEmpty,
        ))
        .unwrap(),
    )
    .unwrap();
    let atom = lower_legacy_kind_atom(
        &KindAtomSyntax::cast(canonical_node(
            ":status",
            rules::KIND_ATOM,
            SyntaxKind::KindAtom,
        ))
        .unwrap(),
    )
    .unwrap();

    assert!(matches!(any, Kind::Any));
    assert!(matches!(empty, Kind::Empty));
    assert!(matches!(atom, Kind::Atom(_)));
}
