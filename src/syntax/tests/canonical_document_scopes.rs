use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

#[test]
fn mika_scope_owners_preserve_nesting_and_equal_sibling_bodies_stay_distinct() {
    let source = "~∘~⸢x := 1\n\n╭◉╮⸢x := 2\n⸥\n⸥\n\n~∘~⸢x := 1\n⸥\n";
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(7), Revision(3), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let document = DocumentSyntax::cast(parsed.syntax()).unwrap();
    let scopes = document.mika_scopes();
    assert_eq!(scopes.len(), 3);
    assert_eq!(scopes[0].parent, document.scope_id());
    assert_eq!(scopes[1].parent, scopes[0].section.scope_id());
    assert_eq!(scopes[2].parent, document.scope_id());
    assert_ne!(scopes[0].section.scope_id(), scopes[2].section.scope_id());
    for scope in scopes {
        assert!(scope.section.body().is_some());
        assert_eq!(scope.section.scope_id().document, DocumentId(7));
    }
}
