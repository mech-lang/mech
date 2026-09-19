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
fn colonless_and_repeated_prefix_fences_share_the_named_scope() {
    let index = index(
        "```mechworker\n+> math\n```\n\n```mech:worker\n<+ x\n```\n\n```mechmechmec🤖worker\nx := @env/HOME\n```\n",
    );
    assert_eq!(index.interpreter_scopes().len(), 1);
    let scope = SourceScope::Interpreter(index.interpreter_scopes()[0].clone());
    assert_eq!(index.interpreter_scopes()[0].namespace_str, "worker");
    assert_eq!(index.imports_for_scope(&scope).len(), 1);
    assert_eq!(index.exports_for_scope(&scope).len(), 1);
    assert_eq!(index.address_references_for_scope(&scope).len(), 1);
    assert!(index.program_imports().is_empty());
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

#[test]
fn presentation_options_do_not_change_resolver_scope_or_dependencies() {
    let index = index(
        "```mech:worker{output: false, color: red}\n+> @env := cli/env\nx := @env/HOME\n```\n",
    );
    let scope = SourceScope::Interpreter(index.interpreter_scopes()[0].clone());
    assert_eq!(
        index.imports_for_scope(&scope)[0].alias,
        Some(SourceImportAlias::Context("env".into()))
    );
    assert_eq!(index.address_references_for_scope(&scope)[0].target, "env");
    assert!(index.program_imports().is_empty());
}

#[test]
fn source_import_occurrences_cover_only_the_retained_specifier() {
    for specifier in [
        "math",
        "./lib.mec",
        "/lib.mec",
        "https://example.com/lib.mec",
    ] {
        let index = index(&format!("  +> {specifier}\n"));
        assert_eq!(index.imports.len(), 1);
        let range = index.imports[0].occurrence.range.as_ref().unwrap();
        assert_eq!((range.start.row, range.start.col), (1, 6), "{specifier}");
        assert_eq!(
            (range.end.row, range.end.col),
            (1, 6 + specifier.len()),
            "{specifier}"
        );
    }
}

#[test]
fn resolver_locations_count_graphemes_across_source_pieces() {
    for (prefix, column) in [("e\u{301}", 12), ("👩‍💻", 12)] {
        let source = format!("x := \"{prefix}\" + @env/HOME\r\n");
        let contiguous = TextSnapshot::new(DocumentId(1), Revision(1), source.clone()).unwrap();
        let mut pieces = TextSnapshot::new(DocumentId(1), Revision(1), "").unwrap();
        for ch in source.chars() {
            pieces = pieces.append(ch.to_string()).unwrap();
        }
        for snapshot in [contiguous, pieces] {
            let parsed = parse_canonical_document(snapshot, ParseConfig::default());
            assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
            let index = SourceIndex::from_document(&DocumentSyntax::cast(parsed.syntax()).unwrap())
                .unwrap();
            let range = index.address_references[0]
                .occurrence
                .range
                .as_ref()
                .unwrap();
            assert_eq!((range.start.row, range.start.col), (1, column));
            assert_eq!((range.end.row, range.end.col), (1, column + 9));
        }
    }
}

#[test]
fn mika_indexes_share_retained_owners_and_keep_local_resolution_separate() {
    use mech_runtime::resolver::CanonicalDocumentIndex;
    let source = "+> ./root.mec\n\n~∘~⸢+> ./child.mec\nx := @env/HOME\n\n╭◉╮⸢+> ./nested.mec\ny := @nested/VALUE\n⸥\n\n```mechworker{output: false}\n+> ./worker.mec\nz := @worker/VALUE\n```\n⸥\n";
    let document = document(source);
    let index = CanonicalDocumentIndex::from_document(&document).unwrap();
    let owners = document.mika_scopes();
    assert_eq!(index.owner, document.scope_id());
    assert_eq!(index.mika.len(), 2);
    assert_eq!(index.root.imports.len(), 1);
    assert!(index.root.address_references.is_empty());
    for (child, owner) in index.mika.iter().zip(&owners) {
        assert_eq!(child.owner.section.scope_id(), owner.section.scope_id());
        assert_eq!(child.owner.parent, owner.parent);
    }
    assert_eq!(index.mika[0].index.imports.len(), 2);
    assert_eq!(index.mika[1].index.imports.len(), 1);
    assert_eq!(
        index.mika[0].index.program_address_references()[0].target,
        "env"
    );
    assert_eq!(
        index.mika[1].index.program_address_references()[0].target,
        "nested"
    );
    let scope = SourceScope::Interpreter(index.mika[0].index.interpreter_scopes()[0].clone());
    assert_eq!(
        index.mika[0].index.address_references_for_scope(&scope)[0].target,
        "worker"
    );
    let resolver = InMemorySourceResolver::new().with_string("app/child.mec", "value := 42\n");
    let import = &index.mika[0].index.program_imports()[0];
    let resolved = resolver
        .resolve(&source_request_for_import(
            import,
            Some("memory:app/main.mec"),
        ))
        .unwrap()
        .unwrap();
    assert_eq!(resolved.canonical_uri, "memory:app/child.mec");
}

#[test]
fn a_missing_mika_closer_cannot_publish_its_clean_body_index() {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(1), Revision(1), "~∘~⸢+> ./child.mec\n").unwrap(),
        ParseConfig::default(),
    );
    assert!(!parsed.diagnostics.is_empty());
    let document = DocumentSyntax::cast(parsed.syntax()).unwrap();
    assert!(SourceIndex::from_mika_section(&document.mika_scopes()[0].section).is_err());
}

#[test]
fn export_occurrences_cover_the_name_without_indent_or_sigil() {
    for (name, columns) in [("value", 5), ("e\u{301}", 1)] {
        let index = index(&format!("  <+ {name}\r\n"));
        assert_eq!(index.exports.len(), 1);
        assert_eq!(index.exports[0].declaration.name, name);
        let range = index.exports[0].occurrence.range.as_ref().unwrap();
        assert_eq!((range.start.row, range.start.col), (1, 6));
        assert_eq!((range.end.row, range.end.col), (1, 6 + columns));
    }
}

#[test]
fn context_occurrences_span_name_through_last_semantic_role() {
    for (declaration, semantic) in [
        ("@users := @main", "users := @main"),
        ("@users := @main   ", "users := @main"),
        ("@users := file://data/users", "users := file://data/users"),
        ("@users := @main{:read(*)}", "users := @main{:read(*"),
        (
            "@users := @main{:read(users/*), :write(*)}",
            "users := @main{:read(users/*), :write(*",
        ),
        (
            "@users := @main{:read(*), :write(users/*), }",
            "users := @main{:read(*), :write(users/*",
        ),
    ] {
        let index = index(&format!("  {declaration}\r\n"));
        assert_eq!(index.contexts.len(), 1, "{declaration}");
        let range = index.contexts[0].occurrence.range.as_ref().unwrap();
        assert_eq!((range.start.row, range.start.col), (1, 4), "{declaration}");
        assert_eq!(
            (range.end.row, range.end.col),
            (1, 4 + semantic.len()),
            "{declaration}"
        );
    }
}

#[test]
fn context_send_destinations_are_not_indexed_as_addressed_reads() {
    for (source, end_column) in [
        ("@sink/value <- @env/HOME<u8>\n", 29),
        ("@sink/value <- @env/HOME[1]\n", 28),
    ] {
        let index = index(source);
        assert_eq!(index.address_references.len(), 1, "{source}");
        let reference = &index.address_references[0];
        assert_eq!(reference.reference.target, "env");
        assert_eq!(reference.reference.name, "HOME");
        let range = reference.occurrence.range.as_ref().unwrap();
        assert_eq!((range.start.row, range.start.col), (1, 16), "{source}");
        assert_eq!((range.end.row, range.end.col), (1, end_column), "{source}");
    }
}

#[test]
fn context_definition_destinations_are_not_indexed_as_addressed_reads() {
    let context_index = index("@local/value := 1\n42\n");
    assert!(context_index.address_references.is_empty());

    let index = index("answer := @env/HOME\n42\n");
    assert_eq!(index.address_references.len(), 1);
    let reference = &index.address_references[0];
    assert_eq!(reference.reference.target, "env");
    assert_eq!(reference.reference.name, "HOME");
}

#[test]
fn assignment_selectors_and_values_are_indexed_without_the_written_base() {
    let index = index(
        "values := [1 2 3]\nreplacement := 4\nvalues[@env/index] = @env/replacement\nvalues[@live/index] += @live/replacement\n",
    );
    assert_eq!(
        index
            .program_address_references()
            .into_iter()
            .map(|reference| (reference.target, reference.name))
            .collect::<Vec<_>>(),
        [
            ("env".to_owned(), "index".to_owned()),
            ("env".to_owned(), "replacement".to_owned()),
            ("live".to_owned(), "index".to_owned()),
            ("live".to_owned(), "replacement".to_owned()),
        ]
    );
}

#[test]
fn fsm_formal_inputs_and_specifications_are_not_resolver_reads() {
    let source = "#Counter(@formal/input) -> :Count(@start/value)\n:Count(n)\n| @guard/enabled -> :Done(@body/value).\n\n#Shape(@spec/input) => <u64> :=\n| :Done(n).\n";
    let index = index(source);
    assert_eq!(
        index
            .address_references
            .iter()
            .map(|reference| (
                reference.reference.target.as_str(),
                reference.reference.name.as_str()
            ))
            .collect::<Vec<_>>(),
        vec![("start", "value"), ("guard", "enabled"), ("body", "value"),]
    );
}

#[test]
fn many_same_line_occurrences_share_the_batched_location_projection() {
    let reads = std::iter::repeat_n("@env/VALUE", 1_024)
        .collect::<Vec<_>>()
        .join(" + ");
    let index = index(&format!("value := {reads}\n"));
    assert_eq!(index.address_references.len(), 1_024);
    assert!(index.address_references.iter().all(|reference| {
        reference.reference.target == "env" && reference.reference.name == "VALUE"
    }));
}

#[test]
fn finalized_streams_preserve_configured_scopes_import_spans_and_grapheme_columns() {
    use mech_syntax::document::{DocumentStream, StreamProgress};
    let source = "  +> ./lib.mec\r\n\r\n```mechworker{output: false, label: \"a\\n\"}\r\n+> @env := cli/env\r\nx := \"é👩‍💻\" + @env/HOME\r\n```\r\n\r\n```mech:worker\r\n<+ x\r\n```\r\n";
    let expected = index(source);
    let mut stream = DocumentStream::new(DocumentId(826), ParseConfig::default());
    for ch in source.chars() {
        let mut progress = stream.append(&ch.to_string(), 19).unwrap().progress;
        for _ in 0..100_000 {
            if progress != StreamProgress::NeedsProcessing {
                break;
            }
            progress = stream.advance(19).progress;
        }
        assert_eq!(progress, StreamProgress::NeedInput);
    }
    let mut progress = stream.finish(19).progress;
    for _ in 0..100_000 {
        if progress != StreamProgress::NeedsProcessing {
            break;
        }
        progress = stream.advance(19).progress;
    }
    assert_eq!(progress, StreamProgress::Finished);
    let snapshot = stream.materialize().unwrap();
    assert!(snapshot.is_strictly_clean());
    let actual =
        SourceIndex::from_document(&DocumentSyntax::cast(snapshot.syntax()).unwrap()).unwrap();
    assert_eq!(actual, expected);
}

fn streamed_document(source: &str) -> DocumentSyntax {
    use mech_syntax::document::{DocumentStream, StreamProgress};
    let mut stream = DocumentStream::new(DocumentId(826), ParseConfig::default());
    for ch in source.chars() {
        let mut progress = stream.append(&ch.to_string(), 19).unwrap().progress;
        while progress == StreamProgress::NeedsProcessing {
            progress = stream.advance(19).progress;
        }
        assert_eq!(progress, StreamProgress::NeedInput);
    }
    let mut progress = stream.finish(19).progress;
    while progress == StreamProgress::NeedsProcessing {
        progress = stream.advance(19).progress;
    }
    assert_eq!(progress, StreamProgress::Finished);
    let snapshot = stream.materialize().unwrap();
    assert!(snapshot.is_strictly_clean());
    DocumentSyntax::cast(snapshot.syntax()).unwrap()
}

#[test]
fn finalized_streams_mika_indexes_share_retained_owners_and_keep_local_resolution_separate() {
    use mech_runtime::resolver::CanonicalDocumentIndex;
    let source = "+> ./root.mec\n\n~∘~⸢+> ./child.mec\nx := @env/HOME\n\n╭◉╮⸢+> ./nested.mec\ny := @nested/VALUE\n⸥\n\n```mechworker{output: false}\n+> ./worker.mec\nz := @worker/VALUE\n```\n⸥\n";
    let document = streamed_document(source);
    let index = CanonicalDocumentIndex::from_document(&document).unwrap();
    let owners = document.mika_scopes();
    assert_eq!(index.owner, document.scope_id());
    assert_eq!(index.mika.len(), 2);
    assert_eq!(index.root.imports.len(), 1);
    assert!(index.root.address_references.is_empty());
    for (child, owner) in index.mika.iter().zip(&owners) {
        assert_eq!(child.owner.section.scope_id(), owner.section.scope_id());
        assert_eq!(child.owner.parent, owner.parent);
    }
    assert_eq!(index.mika[0].index.imports.len(), 2);
    assert_eq!(index.mika[1].index.imports.len(), 1);
    assert_eq!(
        index.mika[0].index.program_address_references()[0].target,
        "env"
    );
    assert_eq!(
        index.mika[1].index.program_address_references()[0].target,
        "nested"
    );
    let scope = SourceScope::Interpreter(index.mika[0].index.interpreter_scopes()[0].clone());
    assert_eq!(
        index.mika[0].index.address_references_for_scope(&scope)[0].target,
        "worker"
    );
    let resolver = InMemorySourceResolver::new().with_string("app/child.mec", "value := 42\n");
    let import = &index.mika[0].index.program_imports()[0];
    let resolved = resolver
        .resolve(&source_request_for_import(
            import,
            Some("memory:app/main.mec"),
        ))
        .unwrap()
        .unwrap();
    assert_eq!(resolved.canonical_uri, "memory:app/child.mec");
}
