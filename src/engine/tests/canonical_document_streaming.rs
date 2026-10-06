#![cfg(feature = "source_default")]

use mech_engine::CanonicalSourceFrontend;
use mech_syntax::document::{
    AstNode, DocumentId, DocumentStream, DocumentSyntax, ParseConfig, StreamInterpretation,
    StreamProgress, SyntaxSnapshot, parse_canonical_document,
};
use std::sync::Arc;
fn streamed(source: &str) -> Arc<SyntaxSnapshot> {
    let mut stream = DocumentStream::new(DocumentId(826), ParseConfig::default());
    for character in source.chars() {
        let chunk = character.to_string();
        let mut update = stream.append(&chunk, 1024).unwrap();
        while update.progress == StreamProgress::NeedsProcessing {
            update = stream.advance(1024);
        }
        assert_eq!(update.progress, StreamProgress::NeedInput);
    }
    let mut update = stream.finish(1024);
    while update.progress == StreamProgress::NeedsProcessing {
        update = stream.advance(1024);
    }
    assert_eq!(update.progress, StreamProgress::Finished);
    assert_eq!(update.view.identity.kind, StreamInterpretation::Finalized);
    stream.materialize().unwrap()
}
#[test]
fn finalized_streams_feed_the_existing_semantic_frontend_and_artifact_compiler() {
    let actual = streamed("answer := 40 + 2\nanswer\n");
    assert!(actual.is_strictly_clean());
    let expected = parse_canonical_document(actual.source.clone(), ParseConfig::default());
    let actual_document = DocumentSyntax::cast(actual.syntax()).unwrap();
    let expected_document = DocumentSyntax::cast(expected.syntax()).unwrap();
    let actual = CanonicalSourceFrontend
        .compile_document(&actual_document)
        .unwrap();
    let expected = CanonicalSourceFrontend
        .compile_document(&expected_document)
        .unwrap();
    assert_eq!(actual.program().nodes.len(), expected.program().nodes.len());
    assert_eq!(
        actual.program().outputs.len(),
        expected.program().outputs.len()
    );
    assert_eq!(actual.program().outputs.len(), 1);
    assert_eq!(actual.source_map().nodes[0].operation, "math/add");
    assert_eq!(
        actual.source_map().nodes[0].operation,
        expected.source_map().nodes[0].operation
    );
    actual
        .compile_artifact()
        .expect("finalized streaming syntax produces the existing artifact");
}
#[test]
fn recovered_final_streams_still_fail_the_existing_semantic_boundary() {
    let snapshot = streamed("answer :=\n");
    assert!(!snapshot.is_strictly_clean());
    let document = DocumentSyntax::cast(snapshot.syntax()).unwrap();
    let error = match CanonicalSourceFrontend.compile_document(&document) {
        Ok(_) => panic!("recovered streamed syntax must not enter semantics"),
        Err(error) => error,
    };
    assert_eq!(error.code, "source-semantics/recovered-syntax");
}
