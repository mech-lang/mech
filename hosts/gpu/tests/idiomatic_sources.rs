//! Source-language qualification precedes library and native-code qualification.
use mech_syntax::document::{
    DocumentId, ParseConfig, Revision, SyntaxKind, SyntaxNode, TextSnapshot,
    parse_canonical_document,
};

fn count(node: &SyntaxNode, kind: SyntaxKind) -> usize {
    usize::from(node.kind() == kind)
        + node.children().map(|child| count(&child, kind)).sum::<usize>()
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
    assert_eq!(count(&syntax, SyntaxKind::FunctionDefine), 3);
    assert_eq!(count(&syntax, SyntaxKind::FunctionDefineStatements), 3);
    assert_eq!(count(&syntax, SyntaxKind::ContextSend), 3);
    assert_eq!(count(&syntax, SyntaxKind::MatrixComprehension), 4);
}
