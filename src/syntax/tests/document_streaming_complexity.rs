//! End-to-end retained streaming gates. Ordinary publication shares canonical
//! event/diagnostic journals; explicit finite preview and full export are metered
//! separately from ingestion and ordinary views.
use mech_syntax::document::{
    DocumentId, DocumentStream, GreenElement, GreenNode, ParseConfig, StreamProgress,
    SyntaxSnapshot, normalize_diagnostics, parse_canonical_document, reconstruct_source,
    validate_lossless,
};
#[path = "support/stream_mirror.rs"]
mod stream_mirror;
use stream_mirror::Mirror;
#[derive(Debug, Default)]
struct Work {
    accepted_bytes: u64,
    accounted: u64,
    views: u64,
    consumer: Mirror,
}
impl Work {
    fn accounted_work(&self) -> u64 {
        self.accounted
    }
}

fn same_tree(left: &GreenNode, right: &GreenNode) {
    assert_eq!(
        (left.kind, left.flags, left.text_len),
        (right.kind, right.flags, right.text_len)
    );
    assert_eq!(left.children.len(), right.children.len());
    for (left, right) in left.children.iter().zip(right.children.iter()) {
        match (left, right) {
            (GreenElement::Node(left), GreenElement::Node(right)) => same_tree(left, right),
            (GreenElement::Token(left), GreenElement::Token(right)) => assert_eq!(
                (left.kind, left.flags, left.text_len, left.text_hash),
                (right.kind, right.flags, right.text_len, right.text_hash),
            ),
            _ => panic!("canonical child roles differ"),
        }
    }
}

fn uncapped(snapshot: &SyntaxSnapshot, config: ParseConfig) {
    assert!(
        snapshot.stats.parser_steps < config.limits.fuel,
        "fuel cap hid work"
    );
    assert!(snapshot.stats.events_emitted < u64::from(config.limits.max_events));
    assert!(snapshot.stats.recovery_bytes < u64::from(config.limits.max_recovery_bytes));
    assert!(!snapshot.stats.diagnostics_truncated);
    assert!(snapshot.diagnostics.len() < config.limits.max_diagnostics as usize);
    for diagnostic in snapshot.diagnostics.iter() {
        assert!(
            !diagnostic.code.as_str().ends_with("-limit"),
            "hard limit: {diagnostic:?}"
        );
    }
}

fn append(session: &mut DocumentStream, chunk: &str, views: bool, work: &mut Work) {
    let before = session.work();
    let consumer_before = work.consumer.work();
    let mut update = session.append(chunk, 65_536).unwrap();
    loop {
        if views {
            work.consumer.apply(&update);
        }
        if update.progress != StreamProgress::NeedsProcessing {
            break;
        }
        update = session.advance(65_536);
    }
    work.accounted += work.consumer.work() - consumer_before;
    assert_eq!(update.progress, StreamProgress::NeedInput);
    work.accepted_bytes += chunk.len() as u64;
    if views {
        let view = session.view();
        assert_eq!(view.kind(), mech_syntax::document::SyntaxKind::Document);
        assert_eq!(view.source.byte_len(), session.source().byte_len());
        // Exercise bounded random access to completed canonical nodes; callers
        // need not enumerate the settled prefix to publish an ordinary view.
        for index in view.event_count().saturating_sub(8)..view.event_count() {
            let (node, lookup_work) = view.completed_node_with_work(index);
            work.accounted += lookup_work;
            if let Some(node) = node {
                assert!(node.range().end <= view.source.byte_len());
            }
        }
        work.views += 1;
    }
    work.accounted += session.work().total() - before.total();
}

fn final_equivalence(
    session: &mut DocumentStream,
    source: &str,
    clean: bool,
    config: ParseConfig,
    consumer: Option<&mut Mirror>,
) -> u64 {
    let mut consumer = consumer;
    let consumer_before = consumer.as_ref().map_or(0, |mirror| mirror.work());
    let before = session.work().total();
    let mut update = session.finish(65_536);
    loop {
        if let Some(consumer) = consumer.as_mut() {
            consumer.apply(&update);
        }
        if update.progress != StreamProgress::NeedsProcessing {
            break;
        }
        update = session.advance(65_536);
    }
    assert_eq!(update.progress, StreamProgress::Finished);
    if let Some(consumer) = consumer.as_ref() {
        consumer.assert_matches(&update.view);
    }
    let finish_work = session.work().total() - before
        + consumer.as_ref().map_or(0, |mirror| mirror.work())
        - consumer_before;
    let actual = session.materialize().unwrap();
    uncapped(&actual, config);
    assert_eq!(actual.source.byte_len().to_usize(), source.len());
    let expected = parse_canonical_document(actual.source.clone(), config);
    uncapped(&expected, config);
    same_tree(&actual.root, &expected.root);
    validate_lossless(&actual.root, &actual.source).unwrap();
    assert_eq!(
        reconstruct_source(&actual.root, &actual.source).unwrap(),
        source
    );
    assert_eq!(
        normalize_diagnostics(&actual.diagnostics, actual.revision, &actual.nodes),
        normalize_diagnostics(&expected.diagnostics, expected.revision, &expected.nodes),
    );
    assert_eq!(actual.is_strictly_clean(), expected.is_strictly_clean());
    assert_eq!(
        actual.is_strictly_clean(),
        clean,
        "{:#?}",
        actual.diagnostics
    );
    finish_work
}

fn workload(source: &str, views: bool, clean: bool) -> Work {
    workload_with_config(
        source,
        views,
        clean,
        ParseConfig::default(),
        Default::default(),
        8,
    )
}

fn workload_with_config(
    source: &str,
    views: bool,
    clean: bool,
    config: ParseConfig,
    limits: mech_syntax::document::StreamLimits,
    chunk_bytes: usize,
) -> Work {
    let mut session = DocumentStream::with_limits(DocumentId(826), config, limits);
    let mut work = Work::default();
    let mut history = Vec::new();
    let mut start = 0;
    while start < source.len() {
        let mut end = (start + chunk_bytes).min(source.len());
        while !source.is_char_boundary(end) {
            end -= 1;
        }
        append(&mut session, &source[start..end], views, &mut work);
        if views {
            history.push(session.view());
            work.accounted += 1;
        }
        start = end;
    }
    for view in &history {
        assert!(view.source.byte_len() <= session.source().byte_len());
    }
    // Explicit full export/differential validation happens once after ingestion;
    // it is not included in this baseline's ordinary-publication accounting.
    work.accounted += final_equivalence(
        &mut session,
        source,
        clean,
        config,
        views.then_some(&mut work.consumer),
    );
    assert!(session.work().parser_work < limits.max_parser_work);
    assert_eq!(session.work().source_bytes_copied, source.len() as u64);
    assert_eq!(work.accepted_bytes as usize, source.len());
    work
}

fn growth(family: &str, make_source: impl Fn(usize) -> String) {
    let mut failures = Vec::new();
    for views in [false, true] {
        let mut previous = None;
        for n in [64, 128, 256, 512] {
            let work = workload(&make_source(n), views, family != "malformed-tail");
            eprintln!(
                "S7B_STREAM family={family} n={n} normal_views={views} bytes={} accounted={} consumer_read={} logical_removed={} updates={} consumer_work={} ranges_adopted={} ranges_removed={} range_visits={}",
                work.accepted_bytes,
                work.accounted,
                work.consumer.records_read,
                work.consumer.records_removed,
                work.consumer.updates,
                work.consumer.work(),
                work.consumer.ranges_adopted,
                work.consumer.ranges_removed,
                work.consumer.range_visits
            );
            if let Some(previous) = previous {
                if work.accounted_work() > previous * 3 {
                    failures.push((views, n, previous, work.accounted_work()));
                }
            }
            previous = Some(work.accounted_work());
        }
    }
    assert!(
        failures.is_empty(),
        "S7B <=3x doubling gate failed for {family}: {failures:?}"
    );
}

#[test]
fn long_unfinished_string_has_bounded_cumulative_work() {
    growth("string", |n| format!("x := \"{}\"\n", "a".repeat(n)));
}

#[test]
fn growing_nested_expression_has_bounded_cumulative_work() {
    growth("nested", |n| {
        format!("x := (((({}1))))\n", "1 + ".repeat(n / 4))
    });
}

#[test]
fn unfinished_fenced_body_has_bounded_cumulative_work() {
    growth("fence", |n| {
        format!("```text\n{}\n```\n", "abcdefgh\n".repeat(n / 8))
    });
}

#[test]
fn small_append_cost_is_independent_of_settled_prefix() {
    let mut costs = Vec::new();
    for n in [64, 128, 256, 512] {
        let prefix = "A settled paragraph.\n\n".repeat(n);
        let mut session = DocumentStream::new(DocumentId(826), ParseConfig::default());
        let mut warmup = Work::default();
        append(&mut session, &prefix, true, &mut warmup);
        let mut work = Work {
            consumer: warmup.consumer,
            ..Work::default()
        };
        append(&mut session, "Next.\n", true, &mut work);
        final_equivalence(
            &mut session,
            &(prefix + "Next.\n"),
            true,
            ParseConfig::default(),
            Some(&mut work.consumer),
        );
        eprintln!(
            "S7B_STREAM family=settled n={n} accounted={}",
            work.accounted
        );
        costs.push(work.accounted_work());
    }
    assert!(
        costs[3] <= costs[0] * 3,
        "S7B stable-prefix gate failed: {costs:?}"
    );
}

#[test]
fn combining_clusters_and_regional_indicators_have_bounded_cumulative_work() {
    growth("combining", |n| {
        format!("x := \"a{}\"\n", "\u{301}".repeat(n))
    });
    growth("regional", |n| format!("x := \"{}\"\n", "🇺🇸".repeat(n / 4)));
}

#[test]
fn executable_fence_lines_have_bounded_cumulative_work() {
    growth("executable-fence", |n| {
        format!("```mech\n{}\n```\n", "x := 1\n".repeat(n / 8))
    });
}

#[test]
fn malformed_tail_and_late_brace_selection_have_bounded_cumulative_work() {
    growth("malformed-tail", |n| format!("x := \"{}", "a".repeat(n)));
    growth("late-brace", |n| {
        format!("x := {{\"{}\": 1}}\n", "a".repeat(n))
    });
}

// Explicitly scale well beyond the historical ~1 KiB token/fence cases. These
// limits are identical for every size and cover both streaming and its oracle.
fn large_config() -> (ParseConfig, mech_syntax::document::StreamLimits) {
    use mech_syntax::document::{ParseLimits, StreamLimits};
    (
        ParseConfig {
            limits: ParseLimits {
                max_nesting: 256,
                max_diagnostics: 65_536,
                max_events: 16_000_000,
                max_recovery_bytes: 8_000_000,
                fuel: 1_000_000_000,
            },
        },
        StreamLimits {
            max_source_bytes: 1_000_000,
            max_parser_work: 2_000_000_000,
        },
    )
}
fn large_growth(family: &str, make_source: impl Fn(usize) -> String, clean: bool) {
    let (config, limits) = large_config();
    eprintln!(
        "S7B_LARGE_CONFIG family={family} config={config:?} session={limits:?} chunk_bytes=64 allowance=65536"
    );
    let mut previous: Option<(u64, u64)> = None;
    for n in [8_192, 16_384, 32_768, 65_536] {
        let source = make_source(n);
        let work = workload_with_config(&source, true, clean, config, limits, 64);
        eprintln!(
            "S7B_LARGE family={family} n={n} bytes={} accounted={} consumer_read={} logical_removed={} updates={} consumer_work={} ranges_adopted={} ranges_removed={} range_visits={}",
            work.accepted_bytes,
            work.accounted,
            work.consumer.records_read,
            work.consumer.records_removed,
            work.consumer.updates,
            work.consumer.work(),
            work.consumer.ranges_adopted,
            work.consumer.ranges_removed,
            work.consumer.range_visits
        );
        if let Some((total, consumer)) = previous {
            assert!(
                work.accounted <= total * 3,
                "{family}: total growth {total} -> {}",
                work.accounted
            );
            assert!(
                work.consumer.work() <= consumer * 3,
                "{family}: consumer growth {consumer} -> {}",
                work.consumer.work()
            );
        }
        previous = Some((work.accounted, work.consumer.work()));
    }
}
#[test]
fn large_generated_mech_consumes_every_delta() {
    large_growth("mech", |n| "x := 1\n".repeat(n / 7), true);
}
#[test]
fn large_mixed_documents_consume_every_delta() {
    let unit = "A settled paragraph.\n\n```mech\nx := 1\n```\n\n";
    large_growth("mixed", |n| unit.repeat(n / unit.len()), true);
}
#[test]
fn large_unfinished_tokens_and_late_selection_consume_every_delta() {
    large_growth("string", |n| format!("x := \"{}\"\n", "a".repeat(n)), true);
    large_growth(
        "late-brace",
        |n| format!("x := {{\"{}\": 1}}\n", "a".repeat(n)),
        true,
    );
    large_growth("prose", |n| format!("A {}.\n", "word ".repeat(n / 5)), true);
}
#[test]
fn large_executable_fences_consume_every_delta() {
    large_growth(
        "executable-fence",
        |n| format!("```mech\n{}\n```\n", "x := 1\n".repeat(n / 7)),
        true,
    );
}
#[test]
fn large_malformed_tails_consume_every_delta() {
    large_growth(
        "malformed-tail",
        |n| format!("x := \"{}", "a".repeat(n)),
        false,
    );
}
#[test]
fn large_settled_prefix_does_not_increase_small_append_work() {
    let (config, limits) = large_config();
    let mut costs = Vec::new();
    for n in [8_192, 16_384, 32_768, 65_536] {
        let prefix = "A settled paragraph.\n\n".repeat(n / 22);
        let mut session = DocumentStream::with_limits(DocumentId(826), config, limits);
        let mut warmup = Work::default();
        append(&mut session, &prefix, true, &mut warmup);
        let mut work = Work {
            consumer: warmup.consumer,
            ..Work::default()
        };
        let before = work.consumer.work();
        append(&mut session, "Next.\n", true, &mut work);
        work.consumer.assert_matches(&session.view());
        eprintln!(
            "S7B_LARGE family=settled n={n} bytes={} accounted={} consumer={}",
            prefix.len(),
            work.accounted,
            work.consumer.work() - before
        );
        costs.push(work.accounted);
        final_equivalence(
            &mut session,
            &(prefix + "Next.\n"),
            true,
            config,
            Some(&mut work.consumer),
        );
    }
    assert!(costs[3] <= costs[0] * 3, "settled prefix costs: {costs:?}");
}
