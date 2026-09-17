use mech_syntax::document::{
  DocumentId, ParseConfig, Revision, TextSnapshot, parse_document, render_plain,
};

fn render(text: &str) -> Vec<String> {
  let snapshot = parse_document(
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
fn renders_missing_expression_with_operator_context() {
  assert_eq!(
    render("x := 1 +\n"),
    vec![String::from(
      "Error[syntax/missing-expression] at 1:9: expected an expression\n  1:8: `+` requires a right operand\n"
    )]
  );
}

#[test]
fn renders_malformed_mech_before_heading_without_losing_heading() {
  let rendered = render("x := @\n1. Next\n--------\n");
  assert_eq!(rendered.len(), 1);
  assert!(rendered[0].contains("syntax/unexpected-token"));
  assert!(rendered[0].contains("1:6"));
}

#[test]
fn renders_multiple_independent_errors() {
  let rendered = render("x :=\n1. Next\n--------\ny := (1\n");
  assert_eq!(rendered.len(), 2);
  assert!(rendered[0].contains("syntax/missing-expression"));
  assert!(rendered[1].contains("syntax/unclosed-delimiter"));
  assert!(rendered[1].contains("opening `(` is here"));
}
