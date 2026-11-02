//! Named result signatures use the same output arrow as expression functions.
use mech_syntax::document::{
    AstNode, DocumentId, FunctionDefineSyntax, ParseConfig, Revision, SyntaxKind, SyntaxNode,
    TextSnapshot, parse_canonical_document,
};

fn definitions(node: &SyntaxNode, output: &mut Vec<FunctionDefineSyntax>) {
    if let Some(function) = FunctionDefineSyntax::cast(node.clone()) {
        output.push(function);
    }
    for child in node.children() {
        definitions(&child, output);
    }
}

#[test]
fn named_output_arrows_retain_statement_bodies_and_output_annotations() {
    for source in [
        "Identity(x<u64>) => (output<u64>) := output := x.\n",
        "Pair(x<i64>) => (first<i64>, second<u64>) := first := x; second := 1u64.\n",
        "frequencies(h<{string:u64}>) => (output<string>) := output := \"example\".\n",
    ] {
        let parsed = parse_canonical_document(
            TextSnapshot::new(DocumentId(0x46554e), Revision(1), source).unwrap(),
            ParseConfig::default(),
        );
        assert!(
            parsed.is_strictly_clean(),
            "{source}\n{:?}",
            parsed.diagnostics
        );
        let mut functions = Vec::new();
        definitions(&parsed.syntax(), &mut functions);
        assert_eq!(functions.len(), 1, "{source}");
        assert!(
            functions[0]
                .syntax()
                .children()
                .any(|child| child.kind() == SyntaxKind::FunctionDefineStatements)
        );
    }
}
