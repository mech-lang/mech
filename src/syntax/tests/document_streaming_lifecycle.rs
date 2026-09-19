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
    let identity = stream.identity();
    let work = stream.work();
    let repeated = stream.cancel();
    assert_eq!(repeated.progress, StreamProgress::Cancelled);
    assert_eq!(repeated.view.identity, identity);
    assert_eq!(stream.work(), work);
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
fn unchanged_session_roundtrip_does_not_reuse_stream_identity() {
    let session =
        DocumentSession::new_with_document(DocumentId(826), "x := 1\n", ParseConfig::default());
    let mut first = session.into_stream();
    assert_eq!(first.finish(u64::MAX).progress, StreamProgress::Finished);
    let first_snapshot = first.materialize().unwrap();
    let first_identity = first.identity();
    let (mut session, _) = first.into_session().unwrap();
    let mut previous_identity = first_identity;
    let mut previous_root = first_snapshot.root.id;

    for _ in 0..3 {
        let mut next = session.into_stream();
        assert!(next.identity().interpretation > previous_identity.interpretation);
        assert_eq!(next.finish(u64::MAX).progress, StreamProgress::Finished);
        let snapshot = next.materialize().unwrap();
        assert_eq!(first_snapshot.document, snapshot.document);
        assert_eq!(first_snapshot.revision, snapshot.revision);
        assert_eq!(snapshot.source.to_contiguous_string(), "x := 1\n");
        assert_ne!(first_snapshot.root.id, snapshot.root.id);
        assert_ne!(first_identity, next.identity());
        assert_ne!(previous_root, snapshot.root.id);
        assert!(next.identity().interpretation > previous_identity.interpretation);
        previous_identity = next.identity();
        previous_root = snapshot.root.id;
        session = next.into_session().unwrap().0;
    }
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
    let (retained, lookup_work) = next.completed_node_with_work(event);
    let retained = retained.expect("retained node at stable event");
    assert!(lookup_work > 1);
    assert!(lookup_work <= u64::from(usize::BITS - next.event_count().leading_zeros()) + 2);
    assert_eq!(node.id(), retained.id());
    assert!(Arc::ptr_eq(node.green(), retained.green()));
    let snapshot = finish(&mut stream);
    assert_eq!(snapshot.nodes.node(node.id()).unwrap().range, node.range());
    assert_eq!(node.text().unwrap(), "1");
}

#[test]
fn source_lookup_work_survives_append_and_edits_without_counting_public_reads() {
    let mut stream = DocumentStream::new(DocumentId(826), ParseConfig::default());
    let mut previous = 0;
    for chunk in ["x := 1\n", "y := 2\n", "z := 3\n"] {
        let update = stream.append(chunk, 100_000).unwrap();
        let view = drain(&mut stream, update).view;
        let work = stream.work();
        assert!(work.source_lookup_steps > previous);
        previous = work.source_lookup_steps;
        assert_eq!(
            view.source.to_contiguous_string(),
            stream.source().to_contiguous_string()
        );
        for index in 0..view.event_count() {
            if let Some(node) = view.completed_node(index) {
                let _ = node.text().unwrap();
            }
        }
        assert_eq!(
            stream.work(),
            work,
            "public reads cannot charge live parsing"
        );
    }
    let update = stream
        .apply_edits(
            &[TextEdit::replace(
                TextRange::new(TextSize(5), TextSize(6)),
                "4",
            )],
            100_000,
        )
        .unwrap();
    drain(&mut stream, update);
    assert!(stream.work().source_lookup_steps > previous);
    let snapshot = finish(&mut stream);
    let work = stream.work();
    reconstruct_source(&snapshot.root, &snapshot.source).unwrap();
    assert_eq!(stream.work(), work);
}

#[test]
fn session_conversion_reports_retained_storage_and_shared_export_copy_volume() {
    let mut stream = DocumentStream::new(DocumentId(826), ParseConfig::default());
    stream.append("x :=\ny :=\n", 100_000).unwrap();
    let snapshot = finish(&mut stream);
    assert!(!snapshot.diagnostics.is_empty());
    let before = stream.work();
    let minimum = snapshot.nodes.node_count()
        + snapshot.nodes.token_count()
        + snapshot.restarts.as_slice().len()
        + snapshot.diagnostics.len();
    let (session, work) = stream.into_session().unwrap();
    assert!(
        work.export_work - before.export_work > minimum as u64,
        "diagnostic payloads also count"
    );
    let stream = session.into_stream();
    assert_eq!(
        stream.work().retained_source_bytes,
        snapshot.source.byte_len().0
    );
    assert_eq!(
        stream.work().peak_source_pieces,
        snapshot.source.piece_count()
    );
}
