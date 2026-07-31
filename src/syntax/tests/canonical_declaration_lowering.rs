use mech_core::nodes::{ContextBase, ContextCapabilityScope};
use mech_syntax::document::ast::{
    ContextCapabilityDeclarationSyntax, ContextCapabilityScopeSyntax, ContextDeclarationSyntax,
    ExportDeclarationSyntax,
};
use mech_syntax::document::parser::canonical::parse_canonical_phase_2f_rule_for_test;
use mech_syntax::document::parser::rules;
use mech_syntax::document::{
    AstNode, DocumentId, ParseConfig, Revision, SyntaxKind, SyntaxNode, TextSnapshot,
    lower_legacy_context_capability_declaration, lower_legacy_context_capability_scope,
    lower_legacy_context_declaration, lower_legacy_export_declaration,
};

fn source(text: &str) -> TextSnapshot {
    TextSnapshot::new(DocumentId(935), Revision(0), text).unwrap()
}

fn find_node(root: &SyntaxNode, kind: SyntaxKind) -> Option<SyntaxNode> {
    if root.kind() == kind {
        return Some(root.clone());
    }
    root.children().find_map(|child| find_node(&child, kind))
}

fn node<N: AstNode>(input: &str, rule: mech_syntax::document::RuleId, kind: SyntaxKind) -> N {
    let parsed =
        parse_canonical_phase_2f_rule_for_test(source(input), rule, ParseConfig::default())
            .unwrap();
    assert!(parsed.is_strictly_clean(), "{input:?}");
    N::cast(find_node(&parsed.syntax(), kind).unwrap()).unwrap()
}

#[test]
fn declaration_lowerers_preserve_legacy_values() {
    let export = lower_legacy_export_declaration(&node::<ExportDeclarationSyntax>(
        "<+ value",
        rules::EXPORT_DECLARATION,
        SyntaxKind::ExportDeclaration,
    ))
    .unwrap();
    assert_eq!(export.name.to_string(), "value");

    let context = lower_legacy_context_declaration(&node::<ContextDeclarationSyntax>(
        "@users := @main{:read(users/*), :write(*)}",
        rules::CONTEXT_DECLARATION,
        SyntaxKind::ContextDeclaration,
    ))
    .unwrap();
    assert_eq!(context.name.to_string(), "users");
    assert!(matches!(context.base, ContextBase::Context(_)));
    assert_eq!(context.capabilities.len(), 2);
}

#[test]
fn capability_lowerers_keep_structural_scope_variants() {
    let wildcard = lower_legacy_context_capability_scope(&node::<ContextCapabilityScopeSyntax>(
        "*",
        rules::CONTEXT_CAPABILITY_SCOPE,
        SyntaxKind::ContextCapabilityScope,
    ))
    .unwrap();
    assert!(matches!(wildcard, ContextCapabilityScope::Wildcard(_)));

    let path = lower_legacy_context_capability_scope(&node::<ContextCapabilityScopeSyntax>(
        "users/*",
        rules::CONTEXT_CAPABILITY_SCOPE,
        SyntaxKind::ContextCapabilityScope,
    ))
    .unwrap();
    assert!(matches!(path, ContextCapabilityScope::Path(_)));

    let declaration =
        lower_legacy_context_capability_declaration(&node::<ContextCapabilityDeclarationSyntax>(
            ":write(users/*)",
            rules::CONTEXT_CAPABILITY_DECLARATION,
            SyntaxKind::ContextCapabilityDeclaration,
        ))
        .unwrap();
    assert_eq!(declaration.operation.to_string(), "write");
}
