use mech_syntax::document::*;
use std::sync::Arc;

pub fn same_tree(left: &GreenNode, right: &GreenNode) {
    assert_eq!(
        (left.kind, left.flags, left.text_len, left.structural_hash),
        (
            right.kind,
            right.flags,
            right.text_len,
            right.structural_hash
        )
    );
    assert_eq!(left.children.len(), right.children.len());
    for (left, right) in left.children.iter().zip(right.children.iter()) {
        match (left, right) {
            (GreenElement::Node(left), GreenElement::Node(right)) => same_tree(left, right),
            (GreenElement::Token(left), GreenElement::Token(right)) => assert_eq!(
                (left.kind, left.flags, left.text_len, left.text_hash),
                (right.kind, right.flags, right.text_len, right.text_hash)
            ),
            _ => panic!("canonical child shape"),
        }
    }
}
pub fn drain(
    stream: &mut DocumentStream,
    mut update: StreamUpdate,
    allowance: u64,
) -> StreamUpdate {
    let mut polls = 0;
    loop {
        assert_ne!(
            update.progress,
            StreamProgress::Limited,
            "uncapped qualification workload exhausted a hard limit: {:?}",
            update.work
        );
        if update.progress != StreamProgress::NeedsProcessing {
            return update;
        }
        update = stream.advance(allowance);
        polls += 1;
        assert!(polls < 2_000_000, "qualification stopped making progress");
    }
}
pub fn append(stream: &mut DocumentStream, text: &str, allowance: u64) -> StreamUpdate {
    let before = stream.work().parser_work;
    let update = stream.append(text, allowance).unwrap();
    assert!(update.work.parser_work - before <= allowance);
    drain(stream, update, allowance)
}
pub fn finish(stream: &mut DocumentStream, allowance: u64) -> Arc<SyntaxSnapshot> {
    let update = stream.finish(allowance);
    assert_eq!(
        drain(stream, update, allowance).progress,
        StreamProgress::Finished
    );
    stream.materialize().unwrap()
}
pub fn equivalent(snapshot: &SyntaxSnapshot, text: &str) {
    let expected = parse_canonical_document(snapshot.source.clone(), ParseConfig::default());
    same_tree(&snapshot.root, &expected.root);
    assert_eq!(
        reconstruct_source(&snapshot.root, &snapshot.source).unwrap(),
        text
    );
    assert_eq!(
        normalize_diagnostics(&snapshot.diagnostics, snapshot.revision, &snapshot.nodes),
        normalize_diagnostics(&expected.diagnostics, expected.revision, &expected.nodes)
    );
    assert!(!snapshot.stats.diagnostics_truncated);
    assert!(snapshot.stats.parser_steps < ParseConfig::default().limits.fuel);
    assert!(snapshot.stats.events_emitted < u64::from(ParseConfig::default().limits.max_events));
    assert!(
        snapshot.stats.recovery_bytes < u64::from(ParseConfig::default().limits.max_recovery_bytes)
    );
}
