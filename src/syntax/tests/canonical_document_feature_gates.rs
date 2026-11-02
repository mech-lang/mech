#![cfg(any(not(feature = "mika"), not(feature = "invariant_define")))]

use mech_syntax::document::{
    DocumentId, ParseConfig, Revision, SyntaxKind, SyntaxNode, TextSnapshot,
    parse_canonical_document,
};

fn source(text: &str) -> TextSnapshot {
    TextSnapshot::new(DocumentId(0x2d9), Revision(1), text).unwrap()
}

fn contains(node: &SyntaxNode, kind: SyntaxKind) -> bool {
    node.kind() == kind || node.children().any(|child| contains(&child, kind))
}

#[cfg(not(feature = "mika"))]
#[test]
fn canonical_document_does_not_recognize_mika_when_disabled() {
    let parsed = parse_canonical_document(source("╭◉╮\n"), ParseConfig::default());
    assert!(!contains(&parsed.syntax(), SyntaxKind::Mika));
    assert!(!contains(&parsed.syntax(), SyntaxKind::MikaSection));
}

#[cfg(not(feature = "invariant_define"))]
#[test]
fn canonical_document_does_not_recognize_invariants_when_disabled() {
    let parsed = parse_canonical_document(source("x! := 1\n"), ParseConfig::default());
    assert!(!contains(&parsed.syntax(), SyntaxKind::InvariantDefine));
}
