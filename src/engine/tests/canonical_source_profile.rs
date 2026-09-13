#![cfg(all(feature = "source", not(feature = "math_add")))]

use mech_engine::CanonicalSourceFrontend;
use mech_syntax::document::parser::canonical::parse_canonical_phase_2i_rule_for_test;
use mech_syntax::document::parser::rules;
use mech_syntax::document::{
    AstNode, DocumentId, ExpressionSyntax, ParseConfig, Revision, SyntaxKind, SyntaxNode,
    TextSnapshot,
};

fn find(node: SyntaxNode, kind: SyntaxKind) -> Option<SyntaxNode> {
    if node.kind() == kind {
        return Some(node);
    }
    node.children().find_map(|child| find(child, kind))
}

fn expression(source: &str) -> ExpressionSyntax {
    let parsed = parse_canonical_phase_2i_rule_for_test(
        TextSnapshot::new(DocumentId(0x544), Revision(1), source).unwrap(),
        rules::EXPRESSION,
        ParseConfig::default(),
    )
    .unwrap();
    assert!(parsed.is_strictly_clean(), "{source:?}");
    find(parsed.syntax(), SyntaxKind::Expression)
        .and_then(ExpressionSyntax::cast)
        .expect("canonical Expression")
}

#[test]
fn maintained_source_types_do_not_depend_on_an_engine_feature_mirror() {
    let compiled = CanonicalSourceFrontend
        .compile_expression(&expression("1 + 2"))
        .expect("the maintained arithmetic declaration must resolve");
    assert_eq!(
        compiled.program().nodes[0]
            .operation()
            .expect("ordinary source operation")
            .canonical_name(),
        "math/add"
    );
    assert!(compiled.contracts()[0].is_some());
}
