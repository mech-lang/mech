#![cfg(all(feature = "source", not(feature = "math_add")))]

use mech_engine::CanonicalSourceFrontend;
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
    let parsed = mech_syntax::document::parse_canonical_document(
        TextSnapshot::new(DocumentId(0x544), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean(), "{source:?}");
    let syntax = find(parsed.syntax(), SyntaxKind::Expression)
        .and_then(ExpressionSyntax::cast)
        .expect("canonical Expression");
    assert_eq!(syntax.syntax().range(), parsed.source.full_range());
    syntax
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
