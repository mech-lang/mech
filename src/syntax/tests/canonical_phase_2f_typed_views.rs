use mech_syntax::document::ast::{
    CanonicalContextBaseSyntax, CanonicalContextCapabilityScopeSyntax,
    CanonicalSourceImportSpecifierSyntax, ContextCapabilityDeclarationSyntax,
    ContextCapabilityPathSyntax, ContextCapabilityScopeSyntax, ContextDeclarationSyntax,
    ExportDeclarationSyntax, ImportDeclarationSyntax, SourceImportSpecifierSyntax,
    SourceImportTailSyntax, SourceMecPathSyntax, SourcePathComponentSyntax,
    UriSourceImportSpecifierSyntax,
};
use mech_syntax::document::parser::canonical::parse_canonical_phase_2f_rule_for_test;
use mech_syntax::document::parser::rules;
use mech_syntax::document::{
    AstNode, DocumentId, ParseConfig, Revision, RuleId, SyntaxKind, SyntaxNode, TextSnapshot,
};

fn source(text: &str) -> TextSnapshot {
    TextSnapshot::new(DocumentId(936), Revision(0), text).unwrap()
}

fn find_node(root: &SyntaxNode, kind: SyntaxKind) -> Option<SyntaxNode> {
    if root.kind() == kind {
        return Some(root.clone());
    }
    root.children().find_map(|child| find_node(&child, kind))
}

fn node<N: AstNode>(input: &str, rule: RuleId, kind: SyntaxKind) -> N {
    let parsed =
        parse_canonical_phase_2f_rule_for_test(source(input), rule, ParseConfig::default())
            .unwrap();
    assert!(parsed.is_strictly_clean(), "{input:?}");
    N::cast(find_node(&parsed.syntax(), kind).unwrap()).unwrap()
}

#[test]
fn source_import_views_expose_only_physical_structure() {
    let tail = node::<SourceImportTailSyntax>(
        "x/dep   ",
        rules::SOURCE_IMPORT_TAIL,
        SyntaxKind::SourceImportTail,
    );
    assert_eq!(
        tail.physical_tokens()
            .into_iter()
            .map(|token| token.text().unwrap())
            .collect::<String>(),
        "x/dep   ",
    );

    let component = node::<SourcePathComponentSyntax>(
        "foo-1_bar.mec",
        rules::SOURCE_PATH_COMPONENT,
        SyntaxKind::SourcePathComponent,
    );
    assert_eq!(component.tokens().len(), 13);

    let path = node::<SourceMecPathSyntax>(
        "lib/foo.mec",
        rules::SOURCE_MEC_PATH,
        SyntaxKind::SourceMecPath,
    );
    assert_eq!(
        path.components()
            .map(|component| component.syntax().text().unwrap())
            .collect::<Vec<_>>(),
        vec!["lib", "foo.mec"],
    );

    let uri = node::<UriSourceImportSpecifierSyntax>(
        "x://path",
        rules::URI_SOURCE_IMPORT_SPECIFIER,
        SyntaxKind::UriSourceImportSpecifier,
    );
    assert_eq!(uri.scheme().unwrap().syntax().text().unwrap(), "x");
    assert_eq!(uri.tail().unwrap().syntax().text().unwrap(), "path");
}

#[test]
fn selected_source_import_views_are_structural() {
    for (input, expected_wildcard) in [("./dep.mec", false), ("./dep.mec/*", true)] {
        let selected = node::<SourceImportSpecifierSyntax>(
            input,
            rules::SOURCE_IMPORT_SPECIFIER,
            SyntaxKind::SourceImportSpecifier,
        )
        .selected()
        .unwrap();
        match selected {
            CanonicalSourceImportSpecifierSyntax::Relative(syntax) => {
                assert!(syntax.path().is_some());
                assert_eq!(syntax.has_wildcard_suffix(), expected_wildcard);
            }
            other => panic!("expected relative selection, got {other:?}"),
        }
    }
    let declaration = node::<ImportDeclarationSyntax>(
        "+> dep.mec",
        rules::IMPORT_DECLARATION,
        SyntaxKind::ImportDeclaration,
    );
    assert!(declaration.specifier().is_some());
}

#[test]
fn declaration_views_expose_context_children_in_source_order() {
    let export = node::<ExportDeclarationSyntax>(
        "<+ value",
        rules::EXPORT_DECLARATION,
        SyntaxKind::ExportDeclaration,
    );
    assert_eq!(export.name().unwrap().syntax().text().unwrap(), "value");

    let context = node::<ContextDeclarationSyntax>(
        "@users := @main{:read(users/*), :write(*)}",
        rules::CONTEXT_DECLARATION,
        SyntaxKind::ContextDeclaration,
    );
    assert_eq!(context.name().unwrap().syntax().text().unwrap(), "users");
    assert!(matches!(
        context.base(),
        Some(CanonicalContextBaseSyntax::Context(_))
    ));
    assert_eq!(
        context
            .capabilities()
            .map(|capability| capability.operation().unwrap().syntax().text().unwrap())
            .collect::<Vec<_>>(),
        vec!["read", "write"],
    );

    let cap = node::<ContextCapabilityDeclarationSyntax>(
        ":read(users/*)",
        rules::CONTEXT_CAPABILITY_DECLARATION,
        SyntaxKind::ContextCapabilityDeclaration,
    );
    assert_eq!(cap.operation().unwrap().syntax().text().unwrap(), "read");
    assert!(cap.scope().is_some());

    let path = node::<ContextCapabilityPathSyntax>(
        "users/*",
        rules::CONTEXT_CAPABILITY_PATH,
        SyntaxKind::ContextCapabilityPath,
    );
    assert_eq!(
        path.tokens()
            .into_iter()
            .map(|token| token.text().unwrap())
            .collect::<String>(),
        "users/*",
    );

    let wildcard = node::<ContextCapabilityScopeSyntax>(
        "*",
        rules::CONTEXT_CAPABILITY_SCOPE,
        SyntaxKind::ContextCapabilityScope,
    );
    assert!(matches!(
        wildcard.selected(),
        Some(CanonicalContextCapabilityScopeSyntax::Wildcard(_))
    ));
}
