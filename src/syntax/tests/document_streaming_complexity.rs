//! End-to-end retained streaming gates. Ordinary publication shares canonical
//! event/diagnostic journals; explicit finite preview and full export are metered
//! separately from ingestion and ordinary views.
use mech_syntax::document::{
    DocumentId, DocumentStream, GreenElement, GreenNode, ParseConfig, StreamProgress,
    SyntaxSnapshot, normalize_diagnostics, parse_canonical_document, reconstruct_source,
    validate_lossless,
};
#[derive(Debug, Default)]
struct Work {
    accepted_bytes: u64,
    accounted: u64,
    views: u64,
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
    let mut update = session.append(chunk, 65_536).unwrap();
    while update.progress == StreamProgress::NeedsProcessing {
        update = session.advance(65_536);
    }
    assert_eq!(update.progress, StreamProgress::NeedInput);
    work.accepted_bytes += chunk.len() as u64;
    if views {
        let view = session.view();
        assert_eq!(view.kind(), mech_syntax::document::SyntaxKind::Document);
        assert_eq!(view.source.byte_len(), session.source().byte_len());
        work.views += 1;
    }
    work.accounted += session.work().total() - before.total();
}

fn final_equivalence(session: &mut DocumentStream, source: &str) -> u64 {
    let before = session.work().total();
    let mut update = session.finish(65_536);
    while update.progress == StreamProgress::NeedsProcessing {
        update = session.advance(65_536);
    }
    assert_eq!(update.progress, StreamProgress::Finished);
    let finish_work = session.work().total() - before;
    let actual = session.materialize().unwrap();
    uncapped(&actual, ParseConfig::default());
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
    finish_work
}

fn workload(source: &str, views: bool) -> Work {
    assert!(source.is_ascii()); // UTF-8 split qualification is a separate target.
    let mut session = DocumentStream::new(DocumentId(826), ParseConfig::default());
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
    work.accounted += final_equivalence(&mut session, source);
    assert_eq!(work.accepted_bytes as usize, source.len());
    work
}

fn growth(family: &str, make_source: impl Fn(usize) -> String) {
    let mut failures = Vec::new();
    for views in [false, true] {
        let mut previous = None;
        for n in [64, 128, 256, 512] {
            let work = workload(&make_source(n), views);
            eprintln!("S7B_STREAM family={family} n={n} normal_views={views} {work:?}");
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
        append(&mut session, &prefix, false, &mut warmup);
        let mut work = Work::default();
        append(&mut session, "Next.\n", true, &mut work);
        final_equivalence(&mut session, &(prefix + "Next.\n"));
        eprintln!("S7B_STREAM family=settled n={n} {work:?}");
        costs.push(work.accounted_work());
    }
    assert!(
        costs[3] <= costs[0] * 3,
        "S7B stable-prefix gate failed: {costs:?}"
    );
}
