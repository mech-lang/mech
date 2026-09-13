#![cfg(feature = "source")]

use mech_runtime::resolver::{
    InMemorySourceResolver, SourceContextBase, SourceContextCapabilityScope, SourceImportAlias,
    SourceImportKind, SourceIndex, SourceResolver, SourceScope, source_request_for_import,
};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

fn document(source: &str) -> DocumentSyntax {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x57a), Revision(3), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(
        parsed.diagnostics.is_empty(),
        "{source:?}: {:?}",
        parsed.diagnostics
    );
    DocumentSyntax::cast(parsed.syntax()).unwrap()
}

fn index(source: &str) -> SourceIndex {
    SourceIndex::from_document(&document(source)).unwrap()
}

#[test]
fn shared_resolver_fixture_indexes_canonical_roles_and_positions() {
    let index = index(include_str!(
        "../../../tests/fixtures/syntax-source-boundary/resolver-index.mec"
    ));
    assert_eq!(index.imports.len(), 1);
    let import = &index.imports[0];
    assert_eq!(import.occurrence.scope, SourceScope::Program);
    assert_eq!(
        import.declaration.alias,
        Some(SourceImportAlias::Context("env".into()))
    );
    assert_eq!(import.declaration.module.as_deref(), Some("cli"));
    assert_eq!(import.declaration.item.as_deref(), Some("env"));
    assert_eq!(import.occurrence.range.as_ref().unwrap().start.row, 2);
    assert_eq!(index.address_references.len(), 1);
    assert_eq!(index.address_references[0].reference.target, "env");
    assert_eq!(index.address_references[0].reference.name, "HOME");
    assert_eq!(
        index.address_references[0]
            .occurrence
            .range
            .as_ref()
            .unwrap()
            .start
            .col,
        9
    );
    assert!(index.interpreter_scopes().is_empty());
}

#[test]
fn canonical_import_requests_resolve_through_the_existing_resolver() {
    let index = index("+> ./lib.mec\n+> math/{sin, cos}\n+> trig := math/trig/sin\n+> stats/*\n");
    let imports = index.program_imports();
    assert_eq!(imports.len(), 5);
    assert_eq!(imports[0].kind, SourceImportKind::DependencyOnly);
    assert_eq!(imports[1].item.as_deref(), Some("sin"));
    assert_eq!(imports[2].item.as_deref(), Some("cos"));
    assert_eq!(
        imports[3].alias,
        Some(SourceImportAlias::Value("trig".into()))
    );
    assert_eq!(imports[3].item.as_deref(), Some("trig/sin"));
    assert_eq!(imports[4].kind, SourceImportKind::Wildcard);
    let resolver = InMemorySourceResolver::new().with_string("app/lib.mec", "answer := 42\n");
    let request = source_request_for_import(&imports[0], Some("memory:app/main.mec"));
    let resolved = resolver.resolve(&request).unwrap().unwrap();
    assert_eq!(resolved.name, "app/lib.mec");
    assert_eq!(resolved.canonical_uri, "memory:app/lib.mec");
}

#[test]
fn scopes_share_named_fences_and_exclude_display_and_disabled_code() {
    let source = "```mech:worker\n+> math\nx := @env/HOME\n```\n\n```mech:worker\n<+ x\n```\n\n```mech:disabled\n+> skipped\n```\n\nShown `@display/VALUE`; evaluated {@live/VALUE}.\n";
    let index = index(source);
    assert_eq!(index.interpreter_scopes().len(), 1);
    let scope = SourceScope::Interpreter(index.interpreter_scopes()[0].clone());
    assert_eq!(index.imports_for_scope(&scope).len(), 1);
    assert_eq!(index.exports_for_scope(&scope)[0].name, "x");
    assert_eq!(index.address_references_for_scope(&scope)[0].target, "env");
    assert_eq!(index.program_address_references().len(), 1);
    assert_eq!(index.program_address_references()[0].target, "live");
    assert!(index.program_imports().is_empty());
    index.validate_address_targets().unwrap();
}

#[test]
fn contexts_preserve_capability_roles_and_use_existing_conflict_validation() {
    let index = index("@users := @main{:read(users/*), :write(*)}\n<+ users\n");
    let context = &index.contexts[0].declaration;
    assert_eq!(context.name, "users");
    assert_eq!(context.base, SourceContextBase::Context("main".into()));
    assert_eq!(context.capabilities[0].operation, "read");
    assert_eq!(
        context.capabilities[0].scope,
        SourceContextCapabilityScope::Path("users/*".into())
    );
    assert_eq!(
        context.capabilities[1].scope,
        SourceContextCapabilityScope::Wildcard
    );
    assert!(index.address_references.is_empty());
    assert_eq!(index.exports[0].declaration.name, "users");
    index.validate_address_targets().unwrap();
    let conflict = self::index("@users := @main{:read(*)}\n\n```mech:users\nx := 1\n```\n");
    assert!(conflict.validate_address_targets().is_err());
}

#[test]
fn index_rejects_unowned_configuration_without_returning_partial_facts() {
    let tree = document("+> math\n\n```mech nonsense\nx := 1\n```\n");
    let error = SourceIndex::from_document(&tree).unwrap_err();
    assert_eq!(error.document, DocumentId(0x57a));
    assert_eq!(error.revision, Revision(3));
    assert_eq!(error.message, "unsupported fence information");
    assert!(error.range.start.0 > 0);
}

#[test]
fn addressed_reads_in_nested_code_keep_the_owning_interpreter_scope() {
    let source = "```mech:worker\n~> @trigger/EVENT { value := @env/HOME }\n```\n";
    let index = index(source);
    let scope = SourceScope::Interpreter(index.interpreter_scopes()[0].clone());
    let references = index.address_references_for_scope(&scope);
    assert_eq!(
        references
            .iter()
            .map(|reference| (reference.target.as_str(), reference.name.as_str()))
            .collect::<Vec<_>>(),
        vec![("trigger", "EVENT"), ("env", "HOME")]
    );
    assert!(index.program_address_references().is_empty());
}

#[test]
fn reindexing_edited_snapshots_updates_scope_and_source_positions() {
    use mech_syntax::document::{DocumentSession, TextEdit, TextRange, TextSize};
    let mut session = DocumentSession::new(
        "```mech:worker\nx := @env/HOME\n```\n",
        ParseConfig::default(),
    );
    let before =
        SourceIndex::from_document(&DocumentSyntax::cast(session.snapshot().syntax()).unwrap())
            .unwrap();
    session.apply_edits(&[
        TextEdit::insert(TextSize(0), "Heading\n\n"),
        TextEdit::replace(TextRange::new(TextSize(8), TextSize(14)), "reader"),
    ]);
    let after =
        SourceIndex::from_document(&DocumentSyntax::cast(session.snapshot().syntax()).unwrap())
            .unwrap();
    assert_eq!(before.interpreter_scopes()[0].namespace_str, "worker");
    assert_eq!(after.interpreter_scopes()[0].namespace_str, "reader");
    assert_eq!(
        before.address_references[0]
            .occurrence
            .range
            .as_ref()
            .unwrap()
            .start
            .row,
        2
    );
    assert_eq!(
        after.address_references[0]
            .occurrence
            .range
            .as_ref()
            .unwrap()
            .start
            .row,
        4
    );
    assert_eq!(
        after.address_references[0].reference,
        before.address_references[0].reference
    );
}

#[test]
fn recovered_documents_do_not_publish_partial_resolver_indexes() {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x57a), Revision(4), "+> math\nx := [1,,2]\n").unwrap(),
        ParseConfig::default(),
    );
    assert!(!parsed.diagnostics.is_empty());
    let document = DocumentSyntax::cast(parsed.syntax()).unwrap();
    let error = SourceIndex::from_document(&document).unwrap_err();
    assert_eq!(error.revision, Revision(4));
    assert_eq!(
        error.message,
        "cannot index a document containing syntax errors"
    );
}

#[test]
fn a_missing_initializer_cannot_publish_an_index_of_earlier_imports() {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x57a), Revision(5), "+> math\nanswer :=\n").unwrap(),
        ParseConfig::default(),
    );
    assert!(!parsed.diagnostics.is_empty());
    let document = DocumentSyntax::cast(parsed.syntax()).unwrap();
    let error = SourceIndex::from_document(&document).unwrap_err();
    assert_eq!(error.revision, Revision(5));
    assert_eq!(
        error.message,
        "cannot index a document containing syntax errors"
    );
}
