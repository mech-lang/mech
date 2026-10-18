//! Source-language qualification precedes library and native-code qualification.
use mech_syntax::document::{
    DocumentId, ParseConfig, Revision, SyntaxKind, SyntaxNode, TextSnapshot,
    parse_canonical_document,
};

fn count(node: &SyntaxNode, kind: SyntaxKind) -> usize {
    usize::from(node.kind() == kind)
        + node
            .children()
            .map(|child| count(&child, kind))
            .sum::<usize>()
}

fn assert_named_function_arrows(node: &SyntaxNode) -> usize {
    let current = usize::from(node.kind() == SyntaxKind::FunctionDefine);
    if current != 0 {
        let body = node
            .children()
            .find(|child| child.kind() == SyntaxKind::FunctionDefineStatements)
            .expect("the target's named functions retain statement bodies");
        let body = body.text().unwrap();
        assert!(body.trim_start().starts_with("=>"), "{body}");
    }
    current
        + node
            .children()
            .map(|child| assert_named_function_arrows(&child))
            .sum::<usize>()
}

#[test]
fn knucleotide_target_is_canonical_code_with_named_output_arrows() {
    let source = include_str!("fixtures/native/knucleotide-idiomatic.mec");
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x4b4e5543), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
    let syntax = parsed.syntax();
    assert!(assert_named_function_arrows(&syntax) > 0);
    assert_eq!(count(&syntax, SyntaxKind::ContextSend), 3);
    assert_eq!(count(&syntax, SyntaxKind::MatrixComprehension), 4);
}
