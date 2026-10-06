use mech_syntax::document::{
    DocumentId, ParseConfig, Revision, TextSnapshot, parse_canonical_document, render_plain,
};

fn render(text: &str) -> Vec<String> {
    let snapshot = parse_canonical_document(
        TextSnapshot::new(DocumentId(5), Revision(0), text).unwrap(),
        ParseConfig::default(),
    );
    snapshot
        .diagnostics
        .iter()
        .map(|diagnostic| render_plain(diagnostic, &snapshot.source, &snapshot.nodes))
        .collect()
}

#[test]
fn renders_missing_operand_at_its_canonical_position() {
    assert_eq!(
        render("x := 1 +\n"),
        vec![String::from(
            "Error[syntax/missing-operator-operand] at 1:9: missing expression after operator\n"
        )]
    );
}

#[test]
fn renders_malformed_mech_before_heading_without_losing_heading() {
    let rendered = render("x := @\n1. Next\n--------\n");
    assert_eq!(rendered.len(), 1);
    assert!(rendered[0].contains("syntax/unexpected-production-source"));
    assert!(rendered[0].contains("1:6"));
}

#[test]
fn renders_multiple_independent_errors() {
    let rendered = render("x :=;\n1. Next\n--------\ny := (1\n");
    assert_eq!(rendered.len(), 2);
    assert!(rendered[0].contains("syntax/missing-variable-definition-value"));
    assert!(rendered[1].contains("syntax/missing-delimiter"));
    assert!(rendered[1].contains("missing closing delimiter"));
}
