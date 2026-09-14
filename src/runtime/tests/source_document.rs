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
    for text in [
        conflict.to_owned(),
        format!("╭◉╮⸢{conflict}⸥\n"),
        format!("╭◉╮⸢~∘~⸢{conflict}⸥\n⸥\n"),
    ] {
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

#[test]
fn separate_mika_namespaces_can_reuse_address_target_names() {
    let text = "@users := @main{:read(*)}\n\n╭◉╮⸢```mech:users\nx := 1\n```\n⸥\n\n~∘~⸢```mech:users\nx := 2\n```\n⸥\n";
    let index = record(text).index().unwrap();
    assert_eq!(index.root.contexts.len(), 1);
    assert_eq!(index.mika.len(), 2);
}
