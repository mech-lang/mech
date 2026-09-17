use mech_syntax::document::*;

fn exercise(config: ParseConfig, limits: StreamLimits, text: &str, step: u64) {
    let mut stream = DocumentStream::with_limits(DocumentId(826), config, limits);
    let mut update = stream.append(text, step).unwrap();
    let mut calls = 0;
    while update.progress == StreamProgress::NeedsProcessing {
        update = stream.advance(step);
        calls += 1;
        assert!(calls < 2_000_000);
    }
    if update.progress == StreamProgress::NeedInput {
        update = stream.finish(step);
        while update.progress == StreamProgress::NeedsProcessing {
            update = stream.advance(step);
            calls += 1;
            assert!(calls < 2_000_000);
        }
    }
    assert!(matches!(
        update.progress,
        StreamProgress::Finished | StreamProgress::Limited
    ));
    assert_eq!(update.view.source.to_contiguous_string(), text);
    assert!(stream.work().parser_work <= limits.max_parser_work);
    let final_snapshot = stream.materialize().unwrap();
    assert_eq!(
        reconstruct_source(&final_snapshot.root, &final_snapshot.source).unwrap(),
        text
    );
    assert!(final_snapshot.stats.parser_steps <= config.limits.fuel);
    assert!(final_snapshot.stats.events_emitted <= u64::from(config.limits.max_events));
    assert!(final_snapshot.stats.recovery_bytes <= u64::from(config.limits.max_recovery_bytes));
    assert!(final_snapshot.diagnostics.len() <= config.limits.max_diagnostics as usize);
    let work = stream.work();
    stream.materialize().unwrap();
    assert_eq!(stream.work(), work);
    assert!(matches!(
        stream.append("more", 10),
        Err(StreamError::Closed(_))
    ));
}
#[test]
fn hard_limits_and_resource_exports_are_lossless_under_small_allowances() {
    let source = "```mech\nx := [1, +, {a: \"é👩‍💻\", b: [2,3]}]\ny := 4\n```\n";
    let mut configs = vec![ParseConfig::default()];
    for fuel in [0, 1, 8, 64] {
        let mut config = ParseConfig::default();
        config.limits.fuel = fuel;
        configs.push(config);
    }
    for events in [0, 1, 6, 7, 16, 64] {
        let mut config = ParseConfig::default();
        config.limits.max_events = events;
        configs.push(config);
    }
    for nesting in [0, 1, 2, 8] {
        let mut config = ParseConfig::default();
        config.limits.max_nesting = nesting;
        configs.push(config);
    }
    for recovery in [0, 1, 8] {
        let mut config = ParseConfig::default();
        config.limits.max_recovery_bytes = recovery;
        configs.push(config);
    }
    for diagnostics in [0, 1] {
        let mut config = ParseConfig::default();
        config.limits.max_diagnostics = diagnostics;
        configs.push(config);
    }
    for config in configs {
        for step in [1, 127] {
            exercise(config, StreamLimits::default(), source, step);
        }
    }
}
#[test]
fn session_work_limit_is_cumulative_and_separate_from_call_allowance() {
    for work in [0, 1, 7, 64, 1024] {
        exercise(
            ParseConfig::default(),
            StreamLimits {
                max_parser_work: work,
                ..StreamLimits::default()
            },
            "x := \"unterminated é👩‍💻",
            1,
        );
    }
}

#[test]
fn rejected_edits_are_atomic_and_do_not_scan_or_allocate_the_insertion() {
    let mut stream = DocumentStream::with_limits(
        DocumentId(826),
        ParseConfig::default(),
        StreamLimits {
            max_source_bytes: 16,
            ..StreamLimits::default()
        },
    );
    stream.append("x := 1\n", 100_000).unwrap();
    let identity = stream.identity();
    let work = stream.work();
    let insertion = "\n".repeat(1_000_000);
    assert_eq!(
        stream
            .apply_edits(
                &[TextEdit::replace(TextRange::empty(TextSize(0)), insertion)],
                100_000
            )
            .unwrap_err(),
        StreamError::SourceLimit
    );
    assert_eq!(stream.identity(), identity);
    assert_eq!(stream.work(), work);
    assert_eq!(stream.source().to_contiguous_string(), "x := 1\n");
}

#[test]
fn preview_and_live_parsing_share_the_cumulative_session_budget() {
    let limit = 128;
    let mut stream = DocumentStream::with_limits(
        DocumentId(826),
        ParseConfig::default(),
        StreamLimits {
            max_parser_work: limit,
            ..StreamLimits::default()
        },
    );
    for _ in 0..8 {
        if stream.state() == StreamState::Limited {
            break;
        }
        stream.append("a", 1).unwrap();
        let preview = stream.preview();
        assert_eq!(
            preview.snapshot.source.byte_len(),
            stream.source().byte_len()
        );
        let work = stream.work();
        assert!(work.parser_work + work.preview_parser_work <= limit);
        stream.preview();
        assert_eq!(stream.work(), work, "cached preview is free");
    }
    assert_eq!(stream.advance(1000).progress, StreamProgress::Limited);
    assert_eq!(
        stream.work().parser_work + stream.work().preview_parser_work,
        limit
    );
}

#[test]
fn limited_export_changes_identity_and_updates_the_delta_baseline() {
    let mut stream = DocumentStream::with_limits(
        DocumentId(826),
        ParseConfig::default(),
        StreamLimits {
            max_parser_work: 1,
            ..StreamLimits::default()
        },
    );
    stream.append("x := [1, 2", 1).unwrap();
    let before = stream.advance(1).view;
    let before_events = format!("{:?}", before.events().collect::<Vec<_>>());
    stream.materialize().unwrap();
    let after = stream.view();
    assert_ne!(before.identity, after.identity);
    assert_eq!(
        before_events,
        format!("{:?}", before.events().collect::<Vec<_>>())
    );
    let update = stream
        .apply_edits(&[TextEdit::replace(TextRange::empty(TextSize(0)), "y")], 1)
        .unwrap();
    assert_eq!(update.syntax.old_len, after.event_count());
    assert_eq!(update.diagnostics.old_len, after.diagnostic_count());
}
