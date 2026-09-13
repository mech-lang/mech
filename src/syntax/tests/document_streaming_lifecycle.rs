use mech_syntax::document::*;
use std::sync::Arc;

fn drain(stream: &mut DocumentStream, mut update: StreamUpdate) -> StreamUpdate {
    let mut calls = 0;
    while update.progress == StreamProgress::NeedsProcessing {
        update = stream.advance(7);
        calls += 1;
        assert!(calls < 1_000_000);
    }
    update
}
fn finish(stream: &mut DocumentStream) -> Arc<SyntaxSnapshot> {
    let update = stream.finish(7);
    assert_eq!(drain(stream, update).progress, StreamProgress::Finished);
    stream.materialize().unwrap()
}
#[test]
fn lifecycle_distinguishes_scheduling_input_and_finality() {
    let mut stream = DocumentStream::new(DocumentId(826), ParseConfig::default());
    let update = stream.append("x := \"hel", 1).unwrap();
    assert_eq!(update.progress, StreamProgress::NeedsProcessing);
    assert_eq!(
        drain(&mut stream, update).progress,
        StreamProgress::NeedInput
    );
    assert_eq!(stream.state(), StreamState::Open);
    let preview = stream.preview();
    assert_eq!(preview.identity.state, StreamState::Open);
    assert_eq!(preview.identity.kind, StreamInterpretation::FinitePreview);
    assert_ne!(preview.identity, stream.identity());
    assert!(!preview.snapshot.is_strictly_clean());
    let update = stream.append("lo\"\n", 7).unwrap();
    drain(&mut stream, update);
    let before = stream.identity();
    let final_snapshot = finish(&mut stream);
    assert!(final_snapshot.is_strictly_clean());
    assert_eq!(before.revision, stream.identity().revision);
    assert_ne!(before, stream.identity());
    assert_eq!(
        reconstruct_source(&final_snapshot.root, &final_snapshot.source).unwrap(),
        "x := \"hello\"\n"
    );
    let work = stream.work();
    stream.finish(1);
    assert_eq!(stream.work(), work);
    assert!(Arc::ptr_eq(&stream.materialize().unwrap(), &final_snapshot));
    assert_eq!(
        stream.append("x", 1).unwrap_err(),
        StreamError::Closed(StreamState::Finished)
    );
}
#[test]
fn empty_append_preserves_identity_work_and_pending_progress() {
    let mut stream = DocumentStream::new(DocumentId(1), ParseConfig::default());
    let update = stream.append("x := 1\n", 1).unwrap();
    let identity = update.view.identity;
    let work = stream.work();
    let empty = stream.append("", 100_000).unwrap();
    assert_eq!(empty.progress, StreamProgress::NeedsProcessing);
    assert_eq!(empty.view.identity, identity);
    assert_eq!(empty.work, work);
    assert_eq!(empty.syntax.from, empty.syntax.new_len);
    assert_eq!(empty.diagnostics.from, empty.diagnostics.new_len);
}
#[test]
fn cancellation_and_source_capacity_preserve_every_accepted_byte() {
    let mut stream = DocumentStream::with_limits(
        DocumentId(1),
        ParseConfig::default(),
        StreamLimits {
            max_source_bytes: 10,
            ..StreamLimits::default()
        },
    );
    stream.append("x := \"abc", 1).unwrap();
    let before = stream.identity();
    let work = stream.work();
    assert_eq!(
        stream.append("def", 10).unwrap_err(),
        StreamError::SourceLimit
    );
    assert_eq!(stream.identity(), before);
    assert_eq!(stream.work(), work);
    let update = stream.cancel();
    assert_eq!(update.progress, StreamProgress::Cancelled);
    assert_eq!(update.view.source.to_contiguous_string(), "x := \"abc");
    assert_eq!(
        stream.append("", 1).unwrap_err(),
        StreamError::Closed(StreamState::Cancelled)
    );
    assert_eq!(stream.materialize().unwrap_err(), StreamError::NotFinal);
}
#[test]
fn historical_event_and_diagnostic_views_are_immutable() {
    let mut stream = DocumentStream::new(DocumentId(1), ParseConfig::default());
    let update = stream.append("x := [1,", 100_000).unwrap();
    let old = drain(&mut stream, update).view;
    let events = format!("{:?}", old.events().collect::<Vec<_>>());
    let source = old.source.to_contiguous_string();
    stream.append("2]\ny := 3\n", 100_000).unwrap();
    finish(&mut stream);
    assert_eq!(format!("{:?}", old.events().collect::<Vec<_>>()), events);
    assert_eq!(old.source.to_contiguous_string(), source);
    assert_eq!(old.identity.state, StreamState::Open);
}

#[test]
fn edits_invalidate_continuations_and_transfer_between_stream_and_session() {
    let mut stream = DocumentStream::new(DocumentId(826), ParseConfig::default());
    stream.append("x := \"old", 100_000).unwrap();
    let old = stream.preview();
    let work = stream.work().total();
    let update = stream
        .apply_edits(
            &[TextEdit::replace(
                TextRange::new(TextSize(6), TextSize(9)),
                "new",
            )],
            7,
        )
        .unwrap();
    drain(&mut stream, update);
    assert!(stream.work().total() > work);
    assert_eq!(stream.work().full_document_restarts, 1);
    let update = stream.append("\"\n", 7).unwrap();
    drain(&mut stream, update);
    let snapshot = finish(&mut stream);
    assert_eq!(snapshot.source.to_contiguous_string(), "x := \"new\"\n");
    assert!(snapshot.is_strictly_clean());
    assert_eq!(old.snapshot.source.to_contiguous_string(), "x := \"old");
    let (mut session, work) = stream.into_session().unwrap();
    assert!(work.export_work > 0);
    session.apply_edits(&[TextEdit::replace(
        TextRange::new(TextSize(6), TextSize(9)),
        "next",
    )]);
    let mut stream = session.into_stream();
    let update = stream.append("y := 2\n", 7).unwrap();
    drain(&mut stream, update);
    let snapshot = finish(&mut stream);
    assert_eq!(
        snapshot.source.to_contiguous_string(),
        "x := \"next\"\ny := 2\n"
    );
    assert!(snapshot.is_strictly_clean());
}

#[test]
fn retained_completed_nodes_keep_identity_and_typed_access_through_finalization() {
    let mut stream = DocumentStream::new(DocumentId(826), ParseConfig::default());
    let update = stream.append("x := 1\ny := 2\nz := ", 100_000).unwrap();
    let view = drain(&mut stream, update).view;
    let (event, node) = (0..view.event_count())
        .filter_map(|index| view.completed_node(index).map(|node| (index, node)))
        .find(|(_, node)| node.kind() == SyntaxKind::Number && node.text().unwrap() == "1")
        .expect("settled canonical number");
    let update = stream.append("3\n", 100_000).unwrap();
    let next = drain(&mut stream, update).view;
    let retained = next
        .completed_node(event)
        .expect("retained node at stable event");
    assert_eq!(node.id(), retained.id());
    assert!(Arc::ptr_eq(node.green(), retained.green()));
    let snapshot = finish(&mut stream);
    assert_eq!(snapshot.nodes.node(node.id()).unwrap().range, node.range());
    assert_eq!(node.text().unwrap(), "1");
}
