#![cfg(feature = "source")]

use mech_runtime::resolver::SourceDocument;
use mech_syntax::document::{
    DocumentId, ParseConfig, ParseLimits, Revision, TextEdit, TextRange, TextSize, TextSnapshot,
    reconstruct_source, validate_lossless,
};

fn record(text: &str) -> SourceDocument {
    SourceDocument::parse(
        TextSnapshot::new(DocumentId(828), Revision(7), text).unwrap(),
        ParseConfig::default(),
    )
}

#[test]
fn retained_revision_preserves_raw_source_and_grapheme_locations() {
    let text = "  <+ e\u{301}\r\n";
    let record = record(text);
    assert!(record.is_strictly_clean());
    assert_eq!(record.source().document(), DocumentId(828));
    assert_eq!(record.source().revision(), Revision(7));
    assert_eq!(
        record.source().text(record.source().full_range()).unwrap(),
        text
    );
    let index = record.index().unwrap();
    let range = index.root.exports[0].occurrence.range.as_ref().unwrap();
    assert_eq!((range.start.row, range.start.col), (1, 6));
    assert_eq!((range.end.row, range.end.col), (1, 7));
    let clone = record.clone();
    assert!(std::ptr::eq(record.snapshot(), clone.snapshot()));
}

#[test]
fn malformed_records_remain_lossless_without_publishing_partial_facts() {
    for text in ["+> ./ok.mec\nx := [1,,2]\n", "+> ./ok.mec\nx :=\n"] {
        let record = record(text);
        assert!(!record.is_strictly_clean());
        assert!(record.index().is_err());
        assert!(!record.snapshot().diagnostics.is_empty());
        validate_lossless(&record.snapshot().root, record.source()).unwrap();
        assert_eq!(
            reconstruct_source(&record.snapshot().root, record.source()).unwrap(),
            text
        );
    }
}

#[cfg(feature = "mika")]
#[test]
fn local_owners_and_named_scopes_stay_attached_to_the_retained_document() {
    let record =
        record("+> ./root.mec\n\n╭◉╮⸢+> ./child.mec\n\n```mechworker\nx := @env/HOME\n```\n⸥\n");
    let index = record.index().unwrap();
    assert_eq!(index.owner, record.document().scope_id());
    assert_eq!(index.root.imports.len(), 1);
    assert_eq!(index.mika.len(), 1);
    assert_eq!(index.mika[0].index.imports.len(), 1);
    assert_eq!(index.mika[0].index.interpreter_scopes().len(), 1);
    assert_eq!(
        index.mika[0].owner.section.scope_id(),
        record.document().mika_scopes()[0].section.scope_id()
    );
}

#[test]
fn resource_limited_records_fail_strict_admission() {
    let record = SourceDocument::parse(
        TextSnapshot::new(DocumentId(829), Revision(0), "x := 1\n").unwrap(),
        ParseConfig {
            limits: ParseLimits {
                fuel: 1,
                ..ParseLimits::default()
            },
        },
    );
    assert!(!record.is_strictly_clean());
    assert!(record.index().is_err());
    validate_lossless(&record.snapshot().root, record.source()).unwrap();
}

#[test]
fn replacing_a_revision_does_not_mutate_historical_source_or_indexes() {
    let before = record("<+ old\n");
    let source = before
        .source()
        .apply_edits(&[TextEdit::replace(
            TextRange::new(TextSize(3), TextSize(6)),
            "new",
        )])
        .unwrap();
    let after = SourceDocument::parse(source, ParseConfig::default());
    assert_ne!(after.source().revision(), before.source().revision());
    assert_eq!(
        before.index().unwrap().root.exports[0].declaration.name,
        "old"
    );
    assert_eq!(
        after.index().unwrap().root.exports[0].declaration.name,
        "new"
    );
}

#[test]
fn conflicts_in_any_local_owner_reject_the_entire_index() {
    use mech_runtime::resolver::SourceDocumentIndexError;
    let conflict = "@users := @main{:read(*)}\n\n```mech:users\nx := 1\n```\n";
    let mut sources = vec![conflict.to_owned()];
    if cfg!(feature = "mika") {
        sources.extend([
            format!("╭◉╮⸢{conflict}⸥\n"),
            format!("╭◉╮⸢~∘~⸢{conflict}⸥\n⸥\n"),
        ]);
    }
    for text in sources {
        let record = record(&text);
        assert!(record.is_strictly_clean(), "{text:?}");
        let SourceDocumentIndexError::AddressTargets { owner, error } = record.index().unwrap_err()
        else {
            panic!("expected retained address-target conflict");
        };
        assert_eq!(error.kind_name(), "AddressTargetNameConflict");
        assert!(error.kind_message().contains("users"));
        let document = record.document();
        let owners = document.mika_scopes();
        assert_eq!(
            owner,
            owners
                .last()
                .map_or(document.scope_id(), |local| local.section.scope_id())
        );
    }
}

#[cfg(feature = "mika")]
#[test]
fn separate_mika_namespaces_can_reuse_address_target_names() {
    let text = "@users := @main{:read(*)}\n\n╭◉╮⸢```mech:users\nx := 1\n```\n⸥\n\n~∘~⸢```mech:users\nx := 2\n```\n⸥\n";
    let index = record(text).index().unwrap();
    assert_eq!(index.root.contexts.len(), 1);
    assert_eq!(index.mika.len(), 2);
}

#[test]
fn finalized_stream_adoption_preserves_snapshot_identity_and_parser_work() {
    use mech_syntax::document::{DocumentStream, StreamProgress};
    for text in ["  <+ e\u{301}\r\n", "  <+ e\u{301}\r\nx := [1,,2]\r\n"] {
        let mut stream = DocumentStream::new(DocumentId(830), ParseConfig::default());
        stream.append(text, u64::MAX).unwrap();
        assert_eq!(stream.finish(u64::MAX).progress, StreamProgress::Finished);
        let snapshot = stream.materialize().unwrap();
        let work = stream.work();
        let adopted = SourceDocument::from_finished_stream(&mut stream).unwrap();
        assert!(std::ptr::eq(adopted.snapshot(), snapshot.as_ref()));
        assert_eq!(
            stream.work(),
            work,
            "adoption must not parse or export again"
        );
        assert_eq!(adopted.source().to_contiguous_string(), text);
        assert_eq!(adopted.source().document(), snapshot.document);
        assert_eq!(adopted.source().revision(), snapshot.revision);
        assert_eq!(adopted.document().scope_id().node, snapshot.syntax().id());
        if text.contains("[1,,2]") {
            assert!(!adopted.snapshot().diagnostics.is_empty());
            assert!(adopted.index().is_err());
        } else {
            let index = adopted.index().unwrap();
            let range = index.root.exports[0].occurrence.range.as_ref().unwrap();
            assert_eq!((range.start.row, range.start.col), (1, 6));
        }
    }
}

#[test]
fn open_preview_finishing_cancelled_and_limited_streams_cannot_be_adopted() {
    use mech_syntax::document::{DocumentStream, StreamError, StreamLimits, StreamState};
    let mut stream = DocumentStream::new(DocumentId(831), ParseConfig::default());
    stream.append("x := 1\n", 0).unwrap();
    assert_eq!(
        SourceDocument::from_finished_stream(&mut stream).unwrap_err(),
        StreamError::NotFinal
    );
    assert!(stream.preview().snapshot.is_strictly_clean());
    assert_eq!(
        SourceDocument::from_finished_stream(&mut stream).unwrap_err(),
        StreamError::NotFinal
    );
    stream.finish(0);
    assert_eq!(stream.state(), StreamState::Finishing);
    assert_eq!(
        SourceDocument::from_finished_stream(&mut stream).unwrap_err(),
        StreamError::NotFinal
    );
    stream.cancel();
    assert_eq!(
        SourceDocument::from_finished_stream(&mut stream).unwrap_err(),
        StreamError::Closed(StreamState::Cancelled)
    );

    let mut limited = DocumentStream::with_limits(
        DocumentId(832),
        ParseConfig::default(),
        StreamLimits {
            max_parser_work: 1,
            ..StreamLimits::default()
        },
    );
    limited.append("x := 1\n", u64::MAX).unwrap();
    limited.finish(u64::MAX);
    assert_eq!(limited.state(), StreamState::Limited);
    // Even materializing its diagnostic envelope cannot grant finality.
    limited.materialize().unwrap();
    assert_eq!(
        SourceDocument::from_finished_stream(&mut limited).unwrap_err(),
        StreamError::Closed(StreamState::Limited)
    );
}

#[test]
fn finite_session_adoption_preserves_revision_identity_across_later_edits() {
    use mech_syntax::document::DocumentSession;
    let mut session =
        DocumentSession::new_with_document(DocumentId(833), "<+ old\n", ParseConfig::default());
    let before = SourceDocument::from_session(&session);
    assert_eq!(
        before.snapshot().syntax().id(),
        session.snapshot().syntax().id()
    );
    session.apply_edits(&[TextEdit::replace(
        TextRange::new(TextSize(3), TextSize(6)),
        "new",
    )]);
    let after = SourceDocument::from_session(&session);
    assert_eq!(
        after.snapshot().syntax().id(),
        session.snapshot().syntax().id()
    );
    assert_eq!(after.source().revision(), session.snapshot().revision);
    assert_ne!(before.source().revision(), after.source().revision());
    assert_eq!(
        before.index().unwrap().root.exports[0].declaration.name,
        "old"
    );
    assert_eq!(
        after.index().unwrap().root.exports[0].declaration.name,
        "new"
    );
}

#[cfg(not(feature = "mika"))]
#[test]
fn disabled_mika_source_is_retained_but_cannot_publish_an_index() {
    let text = "╭◉╮⸢<+ name\n⸥\n";
    let document = record(text);
    assert_eq!(document.source().to_contiguous_string(), text);
    assert!(!document.is_strictly_clean());
    assert!(document.index().is_err());
}

#[test]
fn session_to_stream_identity_graph_cannot_alias_a_retained_revision() {
    use mech_core::MechSourceCode;
    use mech_runtime::{
        InMemorySourceResolver, InMemoryStore, MechStore, ModuleRecord, ModuleVersionId,
        ModuleVersionRecord, ResolvedSource, module_id,
    };
    use mech_syntax::document::{DocumentSession, StreamProgress};
    let uri = "memory:identity.mec";
    let session = DocumentSession::new_with_document(
        DocumentId(mech_core::hash_str(uri)),
        "answer := 1\n",
        ParseConfig::default(),
    );
    let finite = SourceDocument::from_session(&session);
    let mut stream = session.into_stream();
    assert_eq!(stream.finish(u64::MAX).progress, StreamProgress::Finished);
    let streamed = SourceDocument::from_finished_stream(&mut stream).unwrap();
    assert_eq!(finite.source().revision(), streamed.source().revision());
    assert_eq!(
        finite.source().to_contiguous_string(),
        streamed.source().to_contiguous_string()
    );
    assert_ne!(finite.document().scope_id(), streamed.document().scope_id());
    assert_eq!(finite, finite.clone());
    assert_ne!(finite, streamed);
    let resolved = |document: SourceDocument| {
        ResolvedSource::new(
            "identity.mec",
            uri,
            MechSourceCode::String(document.source().to_contiguous_string()),
        )
        .with_kind(mech_runtime::SourceKind::Mech)
        .with_source_document(document)
        .unwrap()
    };
    let mut resolver = InMemorySourceResolver::new();
    resolver
        .insert_source("identity.mec", resolved(finite.clone()))
        .unwrap();
    assert!(
        resolver
            .insert_source("identity.mec", resolved(streamed.clone()))
            .is_err()
    );
    let mut runtime = mech_runtime::MechRuntime::builder().build().unwrap();
    let options = mech_runtime::ModuleBuildOptions::new("test", "v0.4", "native", &[], &[]);
    runtime
        .store_resolved_module_source(resolved(finite.clone()), options)
        .unwrap();
    assert!(
        runtime
            .store_resolved_module_source(resolved(streamed.clone()), options)
            .is_err()
    );
    let mut store = InMemoryStore::new();
    store
        .put_module(ModuleRecord::new(module_id(uri), uri))
        .unwrap();
    for (id, document, accepted) in [(1, finite, true), (2, streamed, false)] {
        let version = ModuleVersionRecord::new(ModuleVersionId(id), module_id(uri), 1)
            .with_source(MechSourceCode::String(
                document.source().to_contiguous_string(),
            ))
            .with_source_document(Some(document));
        assert_eq!(store.put_module_version(version).is_ok(), accepted);
    }
}

#[test]
fn resolved_source_rejects_conflicting_nominal_origins() {
    use mech_core::{CanonicalNominalPath, MechSourceCode};
    use mech_runtime::{ResolvedSource, SourceKind};

    let uri = "memory:nominal.mec";
    let source = "<event> := :open | :closed\n";
    let origin = |package: &str| CanonicalNominalPath::new(vec![package.to_owned()]).unwrap();
    let document = SourceDocument::parse_resolved(uri, Revision(0), source, ParseConfig::default())
        .unwrap()
        .with_nominal_origin(origin("defining-package"));
    let resolved = ResolvedSource::new("nominal.mec", uri, MechSourceCode::String(source.into()))
        .with_kind(SourceKind::Mech)
        .with_nominal_origin(origin("other-package"));
    assert!(resolved.with_source_document(document).is_err());
}

#[cfg(feature = "pretty_print")]
#[test]
fn repl_renderer_rejects_all_heading_owners_alongside_executable_code() {
    use mech_runtime::CanonicalDocumentRenderer;
    for heading in ["Heading\n====\n", "1. Heading\n----\n", "(1.2) Heading\n"] {
        let source = format!("{heading}\nanswer := 1\n");
        let document = record(&source).document();
        assert!(document.contains_executable_source());
        assert!(
            CanonicalDocumentRenderer
                .render_repl_source_html(&document)
                .unwrap()
                .is_none(),
            "{source:?}"
        );
    }
    assert!(
        CanonicalDocumentRenderer
            .render_repl_source_html(&record("-- comment\nanswer := 1\n").document())
            .unwrap()
            .is_some()
    );
}

#[cfg(feature = "pretty_print")]
#[test]
fn completed_results_cannot_be_relabelled_after_session_to_stream_transition() {
    use mech_engine::CanonicalSourceFrontend;
    use mech_runtime::{CanonicalRenderScope, CanonicalScopeResults};
    use mech_syntax::document::{DocumentSession, StreamProgress};
    let session = DocumentSession::new_with_document(
        DocumentId(835),
        "answer := 1\nanswer\n",
        ParseConfig::default(),
    );
    let finite = SourceDocument::from_session(&session);
    let program = CanonicalSourceFrontend
        .compile_document(&finite.document())
        .unwrap();
    let mut stream = session.into_stream();
    assert_eq!(stream.finish(u64::MAX).progress, StreamProgress::Finished);
    let streamed = SourceDocument::from_finished_stream(&mut stream).unwrap();
    assert_ne!(finite.document().scope_id(), streamed.document().scope_id());
    let values = vec![mech_runtime::RuntimeValueSnapshot::empty()];
    assert!(
        CanonicalScopeResults::from_values(
            finite.document().scope_id(),
            CanonicalRenderScope::Root,
            &program,
            &values
        )
        .is_ok()
    );
    assert!(
        CanonicalScopeResults::from_values(
            streamed.document().scope_id(),
            CanonicalRenderScope::Root,
            &program,
            &values
        )
        .is_err()
    );
}
