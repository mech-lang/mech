use std::hint::black_box;

use mech_syntax::document::parser::canonical::{
    CanonicalRuleOutcome, parse_canonical_phase_2i_rule_for_test,
};
use mech_syntax::document::parser::rules;
use mech_syntax::document::{
    AstNode, DocumentId, MatrixSyntax, ParseConfig, RecordSyntax, Revision, SyntaxKind, SyntaxNode,
    TextSnapshot,
};

// Reuse the existing, audited test allocator; no new unsafe boundary is needed.
#[path = "../../core/tests/support/r6_allocation_probe.rs"]
mod allocation_probe;

#[global_allocator]
static ALLOCATOR: allocation_probe::ProbeAllocator = allocation_probe::ProbeAllocator;

fn source(text: &str, pieces: bool) -> TextSnapshot {
    let initial = TextSnapshot::new(DocumentId(73), Revision(1), "").unwrap();
    if pieces {
        text.chars().fold(initial, |snapshot, character| {
            snapshot.append(character.to_string()).unwrap()
        })
    } else {
        TextSnapshot::new(DocumentId(73), Revision(1), text).unwrap()
    }
}

fn find_kind(node: &SyntaxNode, kind: SyntaxKind) -> Option<SyntaxNode> {
    if node.kind() == kind {
        return Some(node.clone());
    }
    node.children().find_map(|node| find_kind(&node, kind))
}

#[test]
fn allocation_probe_detects_owned_source_text() {
    let snapshot = source("│", false);
    let (text, allocations) =
        allocation_probe::measured(|| black_box(snapshot.text(snapshot.full_range()).unwrap()));
    assert_eq!(text, "│");
    assert!(allocations > 0, "the probe must detect source-text copies");
}

#[test]
fn record_delimiter_access_does_not_allocate_copied_glyph_text() {
    let mut baseline = None;
    for pieces in [false, true] {
        for (opening, closing) in [
            ("{", "}"),
            ("|", "|"),
            ("│", "│"),
            ("┃", "┃"),
            ("╭", "╯"),
            ("┌", "┘"),
            ("┏", "┛"),
            ("│", "|"),
            ("|", "┃"),
        ] {
            let text = format!("{opening}a: 1 {closing}");
            let parsed = parse_canonical_phase_2i_rule_for_test(
                source(&text, pieces),
                rules::RECORD,
                ParseConfig::default(),
            )
            .unwrap();
            assert_eq!(parsed.outcome, CanonicalRuleOutcome::Matched, "{text}");
            let record =
                RecordSyntax::cast(find_kind(&parsed.syntax(), SyntaxKind::Record).unwrap())
                    .unwrap();
            let ((start, end), allocations, bytes) = allocation_probe::measured_with_bytes(|| {
                (
                    black_box(record.opening_delimiter()),
                    black_box(record.closing_delimiter()),
                )
            });
            assert_eq!(start.unwrap().text().unwrap(), opening);
            assert_eq!(end.unwrap().text().unwrap(), closing);
            // These trees have the same physical child structure. Typed handle collections
            // cost the same for ASCII and Unicode; glyph text must not add allocations.
            let expected = *baseline.get_or_insert((allocations, bytes));
            assert_eq!((allocations, bytes), expected, "{text}, pieces={pieces}");
        }
    }
}

#[test]
fn matrix_delimiter_access_does_not_allocate_copied_glyph_text() {
    let mut baseline = None;
    for pieces in [false, true] {
        for (opening, closing) in [("[", "]"), ("╭", "╯")] {
            let text = format!("{opening}1 {closing}");
            let parsed = parse_canonical_phase_2i_rule_for_test(
                source(&text, pieces),
                rules::MATRIX,
                ParseConfig::default(),
            )
            .unwrap();
            assert_eq!(parsed.outcome, CanonicalRuleOutcome::Matched, "{text}");
            let matrix =
                MatrixSyntax::cast(find_kind(&parsed.syntax(), SyntaxKind::Matrix).unwrap())
                    .unwrap();
            let ((start, end), allocations, bytes) = allocation_probe::measured_with_bytes(|| {
                (
                    black_box(matrix.opening_delimiter()),
                    black_box(matrix.closing_delimiter()),
                )
            });
            assert_eq!(start.unwrap().text().unwrap(), opening);
            assert_eq!(end.unwrap().text().unwrap(), closing);
            let expected = *baseline.get_or_insert((allocations, bytes));
            assert_eq!((allocations, bytes), expected, "{text}, pieces={pieces}");
        }
    }
}
