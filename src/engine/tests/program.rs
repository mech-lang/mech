#![cfg(feature = "source")]

use mech_syntax::document::{
    AstNode, CanonicalContextBaseSyntax, ContextDeclarationSyntax, DocumentId, DocumentSyntax,
    ParseConfig, PrefixedContextPathSyntax, Revision, SliceStemSyntax, SyntaxKind, SyntaxNode,
    SyntaxSnapshot, TextSnapshot, VariableAssignSyntax, parse_canonical_document,
};

fn parsed(source: &str) -> SyntaxSnapshot {
    parse_canonical_document(
        TextSnapshot::new(DocumentId(0x620), Revision(1), source).unwrap(),
        ParseConfig::default(),
    )
}

fn document(source: &str) -> DocumentSyntax {
    let parsed = parsed(source);
    assert!(
        parsed.diagnostics.is_empty(),
        "{source:?}: {:?}",
        parsed.diagnostics
    );
    DocumentSyntax::cast(parsed.syntax()).expect("canonical document")
}

fn find(node: SyntaxNode, kind: SyntaxKind) -> Option<SyntaxNode> {
    if node.kind() == kind {
        return Some(node);
    }
    node.children().find_map(|child| find(child, kind))
}

fn text(node: &SyntaxNode) -> String {
    node.text().expect("retained syntax text")
}

#[test]
fn program_browser_resource_binding_declaration() {
    let document = document("@browser := browser://dom/");
    let context = find(document.syntax().clone(), SyntaxKind::ContextDeclaration)
        .and_then(ContextDeclarationSyntax::cast)
        .expect("context declaration");
    assert_eq!(text(context.name().unwrap().syntax()), "browser");
    assert_eq!(context.capabilities().count(), 0);
    let CanonicalContextBaseSyntax::ResourceUri(uri) = context.base().unwrap() else {
        panic!("expected resource URI")
    };
    assert_eq!(text(uri.syntax()), "browser://dom/");
}

#[test]
fn program_browser_resource_read() {
    let document = document("x := @browser/body/content/input/_value");
    let path = find(document.syntax().clone(), SyntaxKind::PrefixedContextPath)
        .and_then(PrefixedContextPathSyntax::cast)
        .expect("context-addressed read");
    assert_eq!(text(path.context().unwrap().syntax()), "browser");
    assert_eq!(
        text(path.address().unwrap().syntax()),
        "body/content/input/_value"
    );
}

#[test]
fn program_browser_resource_write() {
    let document = document("@browser/body/content/output/_value = \"Hello\"");
    let assignment = find(document.syntax().clone(), SyntaxKind::VariableAssign)
        .and_then(VariableAssignSyntax::cast)
        .expect("context-addressed assignment");
    let Some(SliceStemSyntax::Context(path)) = assignment.target().unwrap().stem() else {
        panic!("expected context-addressed assignment target")
    };
    assert_eq!(text(path.context().unwrap().syntax()), "browser");
    assert_eq!(
        text(path.address().unwrap().syntax()),
        "body/content/output/_value"
    );
}

#[test]
fn program_browser_resource_define_syntax_is_rejected() {
    let parsed = parsed("@browser/title := \"Hello\"");
    let document = DocumentSyntax::cast(parsed.syntax()).expect("canonical document");
    let error = mech_engine::CanonicalSourceFrontend
        .compile_document(&document)
        .err()
        .expect("a context-addressed definition must be rejected");
    assert_eq!(error.code, "source-semantics/invalid-definition-target");
    assert_eq!(
        error.anchor.range,
        find(document.syntax().clone(), SyntaxKind::PrefixedContextPath)
            .unwrap()
            .range()
    );
}
