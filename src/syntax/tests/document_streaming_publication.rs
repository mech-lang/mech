use mech_syntax::document::*;
#[path = "support/stream_mirror.rs"]
mod stream_mirror;
use stream_mirror::Mirror;

fn drain(
    stream: &mut DocumentStream,
    mut update: StreamUpdate,
    mirror: &mut Mirror,
) -> StreamProgress {
    loop {
        mirror.apply(&update);
        mirror.assert_matches(&update.view);
        if update.progress != StreamProgress::NeedsProcessing {
            return update.progress;
        }
        update = stream.advance(1);
    }
}

#[test]
fn every_delta_maintains_the_consumer_through_rewinds_recovery_eof_and_edits() {
    let mut stream = DocumentStream::new(DocumentId(826), ParseConfig::default());
    let mut mirror = Mirror::default();
    for text in [
        "x := {\"key",
        "\": 1}\n",
        "y := [1,\n",
        "Prose after recovery.\n\n",
    ] {
        let update = stream.append(text, 1).unwrap();
        assert_eq!(
            drain(&mut stream, update, &mut mirror),
            StreamProgress::NeedInput
        );
    }
    let update = stream.finish(1);
    assert_eq!(
        drain(&mut stream, update, &mut mirror),
        StreamProgress::Finished
    );
    assert!(mirror.updates > 100);
    assert!(mirror.rewinds > 0);
    assert!(mirror.diagnostic_updates > 0);
    let snapshot = stream.materialize().unwrap();
    assert!(!snapshot.is_strictly_clean());
    let update = stream
        .apply_edits(
            &[TextEdit::replace(
                TextRange::new(TextSize(0), stream.source().byte_len()),
                "fixed := 2\n",
            )],
            1,
        )
        .unwrap();
    assert_eq!(update.syntax.from, 0);
    assert_eq!(update.diagnostics.from, 0);
    assert_eq!(
        drain(&mut stream, update, &mut mirror),
        StreamProgress::NeedInput
    );
    let update = stream.finish(1);
    assert_eq!(
        drain(&mut stream, update, &mut mirror),
        StreamProgress::Finished
    );
    assert!(stream.materialize().unwrap().is_strictly_clean());
    mirror.assert_matches(&stream.view());
}

#[test]
fn limited_materialization_requires_explicit_consumer_resynchronization() {
    let mut stream = DocumentStream::with_limits(
        DocumentId(827),
        ParseConfig::default(),
        StreamLimits {
            max_parser_work: 1,
            ..StreamLimits::default()
        },
    );
    let mut mirror = Mirror::default();
    let update = stream.append("x := [1, 2", 1).unwrap();
    assert_eq!(
        drain(&mut stream, update, &mut mirror),
        StreamProgress::Limited
    );
    let old = stream.view();
    stream.materialize().unwrap();
    let exported = stream.view();
    assert_ne!(old.identity, exported.identity);
    mirror.resynchronize(&exported);
    mirror.assert_matches(&exported);
    let update = stream
        .apply_edits(&[TextEdit::replace(TextRange::empty(TextSize(0)), "y")], 1)
        .unwrap();
    assert_eq!(
        drain(&mut stream, update, &mut mirror),
        StreamProgress::Limited
    );
}

#[test]
fn comment_probe_recovery_policy_survives_tiny_yields_and_restores_for_real_code() {
    for (text, limit, clean, recovery) in [
        ("--- heading\n---- heading\r\n---x\n", 0, true, false),
        ("--- heading\n---- heading\r\n", 1, true, false),
        ("--- heading\nx := [1,,2]\n", 65_536, false, true),
        ("--x +\n", 0, false, false),
    ] {
        let config = ParseConfig {
            limits: ParseLimits {
                max_recovery_bytes: limit,
                ..ParseLimits::default()
            },
        };
        let mut stream = DocumentStream::new(DocumentId(828), config);
        let mut mirror = Mirror::default();
        for ch in text.chars() {
            let update = stream.append(&ch.to_string(), 1).unwrap();
            assert_eq!(
                drain(&mut stream, update, &mut mirror),
                StreamProgress::NeedInput
            );
        }
        let update = stream.finish(1);
        assert_eq!(
            drain(&mut stream, update, &mut mirror),
            StreamProgress::Finished
        );
        let snapshot = stream.materialize().unwrap();
        assert_eq!(
            snapshot.is_strictly_clean(),
            clean,
            "{text:?}: {:?}",
            snapshot.diagnostics
        );
        assert_eq!(snapshot.stats.recovery_bytes > 0, recovery, "{text:?}");
        validate_lossless(&snapshot.root, &snapshot.source).unwrap();
        assert_eq!(
            reconstruct_source(&snapshot.root, &snapshot.source).unwrap(),
            text
        );
        // The reference consumer must also perform work on the yielded probe's deltas.
        assert!(mirror.work() >= mirror.updates);
        assert_eq!(mirror.records_read, 0);
    }
}
