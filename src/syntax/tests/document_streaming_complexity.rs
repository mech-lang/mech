//! S7B B1 acceptance gates against the real S7A append implementation.
//!
//! These positive gates intentionally fail until the retained streaming path
//! replaces this test-only driver. Existing counters expose partial accounting, not a
//! claim that source, scanner, indexing, and export costs are fully instrumented.
use mech_syntax::document::{
    DocumentSession, GreenElement, GreenNode, ParseConfig, SyntaxSnapshot, TextEdit,
    normalize_diagnostics, parse_canonical_document, reconstruct_source, validate_lossless,
};

#[derive(Debug, Default)]
struct Work {
    accepted_bytes: u64,
    parser: u64,
    events: u64,
    reconciliation: u64,
    restarts: u64,
    views: u64,
}

impl Work {
    fn accounted_work(&self) -> u64 {
        self.parser + self.events + self.reconciliation
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

fn append(session: &mut DocumentSession, chunk: &str, views: bool, work: &mut Work) {
    let end = session.snapshot().source.byte_len();
    let update = session.apply_edits(&[TextEdit::insert(end, chunk)]);
    work.accepted_bytes += chunk.len() as u64;
    work.parser += update.stats.total_parser_steps;
    work.events += update.stats.total_events_emitted;
    work.reconciliation += update.stats.reconciliation_steps;
    work.restarts += update.stats.document_fallbacks;
    // S7A eagerly builds the entire published snapshot in apply_edits. This
    // access performs no export; that eager publication cost is already above.
    if views {
        let view = session.snapshot().syntax();
        assert_eq!(view.kind(), session.snapshot().root.kind);
        work.views += 1;
    }
    uncapped(session.snapshot(), ParseConfig::default());
}

fn final_equivalence(session: &DocumentSession, source: &str) {
    let actual = session.snapshot();
    assert_eq!(actual.source.byte_len().to_usize(), source.len());
    let expected = parse_canonical_document(actual.source.clone(), ParseConfig::default());
    uncapped(&expected, ParseConfig::default());
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
    // All three selected complete fixtures are valid; recovery cannot mask an
    // accidentally truncated workload. Malformed input gets separate B5 gates.
    assert!(actual.is_strictly_clean(), "{:#?}", actual.diagnostics);
}

fn workload(source: &str, views: bool) -> Work {
    assert!(source.is_ascii()); // UTF-8 split qualification is a separate target.
    let mut session = DocumentSession::new("", ParseConfig::default());
    let mut work = Work::default();
    for chunk in source.as_bytes().chunks(8) {
        append(
            &mut session,
            std::str::from_utf8(chunk).unwrap(),
            views,
            &mut work,
        );
    }
    // Explicit full export/differential validation happens once after ingestion;
    // it is not included in this baseline's ordinary-publication accounting.
    final_equivalence(&session, source);
    assert_eq!(work.accepted_bytes as usize, source.len());
    work
}

fn growth(family: &str, make_source: impl Fn(usize) -> String) {
    let mut failures = Vec::new();
    for views in [false, true] {
        let mut previous = None;
        for n in [64, 128, 256, 512] {
            let work = workload(&make_source(n), views);
            eprintln!("S7B_BASELINE family={family} n={n} normal_views={views} {work:?}");
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
        let mut session = DocumentSession::new(&prefix, ParseConfig::default());
        uncapped(session.snapshot(), ParseConfig::default());
        let mut work = Work::default();
        append(&mut session, "Next.\n", true, &mut work);
        final_equivalence(&session, &(prefix + "Next.\n"));
        eprintln!("S7B_BASELINE family=settled n={n} {work:?}");
        costs.push(work.accounted_work());
    }
    assert!(
        costs[3] <= costs[0] * 3,
        "S7B stable-prefix gate failed: {costs:?}"
    );
}
