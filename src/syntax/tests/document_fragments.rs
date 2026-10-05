//! Retained canonical range and restart contracts after prototype fragments retire.
use mech_syntax::document::{
    DocumentId, GrammarFragmentContext, GrammarFragmentKind, IdGenerator, ParseConfig, Revision,
    SyntaxKind, SyntaxNode, TextRange, TextSize, TextSnapshot, parse_canonical_document,
    parse_canonical_grammar_fragment,
};

fn nodes_of_kind(root: &SyntaxNode, kind: SyntaxKind) -> Vec<SyntaxNode> {
    let mut nodes = Vec::new();
    if root.kind() == kind {
        nodes.push(root.clone());
    }
    for child in root.children() {
        nodes.extend(nodes_of_kind(&child, kind));
    }
    nodes
}

#[test]
fn restart_entries_record_actual_enclosing_parenthetical_depth() {
    let source = TextSnapshot::new(DocumentId(8), Revision(3), "x := (((1)))\n").unwrap();
    let snapshot = parse_canonical_document(source, ParseConfig::default());
    let parentheticals = nodes_of_kind(&snapshot.syntax(), SyntaxKind::ParentheticalExpression);
    assert_eq!(parentheticals.len(), 3);
    let depths = parentheticals
        .iter()
        .map(|node| snapshot.restarts.get(node.id()).unwrap().delimiter_depth)
        .collect::<Vec<_>>();
    assert_eq!(depths, vec![0, 1, 2]);
}

#[test]
fn invalid_utf8_fragment_ranges_return_failed_snapshots_without_panicking() {
    let source = TextSnapshot::new(DocumentId(9), Revision(0), "💡").unwrap();
    for range in [
        TextRange::new(TextSize(1), TextSize(4)),
        TextRange::new(TextSize(0), TextSize(2)),
    ] {
        let mut ids = IdGenerator::new();
        let fragment = parse_canonical_grammar_fragment(
            &source,
            range,
            GrammarFragmentKind::GrammarTerminalToken,
            GrammarFragmentContext::default(),
            ParseConfig::default(),
            &mut ids,
        );
        assert!(!fragment.matched);
        assert!(!fragment.consumed_complete);
        assert!(
            fragment
                .root
                .flags
                .intersects(mech_syntax::document::NodeFlags::ERROR)
        );
    }
}
