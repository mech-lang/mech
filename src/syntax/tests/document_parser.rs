use mech_syntax::document::{
  AstNode, DiagnosticAnchor, DocumentId, DocumentSyntax, NodeFlags, ParseConfig, Revision,
  SyntaxKind, SyntaxNode, TextSnapshot, compact_debug_tree, parse_document, reconstruct_source,
  validate_lossless,
};

fn parse(text: &str) -> mech_syntax::document::SyntaxSnapshot {
  let source = TextSnapshot::new(DocumentId(42), Revision(0), text).unwrap();
  parse_document(source, ParseConfig::default())
}

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

fn assert_lossless(text: &str, snapshot: &mech_syntax::document::SyntaxSnapshot) {
  validate_lossless(&snapshot.root, &snapshot.source).unwrap();
  assert_eq!(reconstruct_source(&snapshot.root, &snapshot.source).unwrap(), text);
}

fn diagnostic_codes(
  snapshot: &mech_syntax::document::SyntaxSnapshot,
) -> Vec<&str> {
  snapshot
    .diagnostics
    .iter()
    .map(|diagnostic| diagnostic.code.as_str())
    .collect()
}

#[test]
fn missing_variable_rhs_is_structural_and_lossless() {
  let snapshot = parse("x :=\n");
  assert_lossless("x :=\n", &snapshot);
  assert_eq!(
    diagnostic_codes(&snapshot),
    vec!["syntax/missing-expression"]
  );
  assert_eq!(nodes_of_kind(&snapshot.syntax(), SyntaxKind::Missing).len(), 1);
  assert!(snapshot.root.flags.contains(NodeFlags::CONTAINS_MISSING));

  let tree = compact_debug_tree(&snapshot.syntax());
  let expected = r#"Document
  Body
    Section
      SectionElement
        MechItem
          VariableDefine
            Identifier
              IdentifierToken "x"
            Whitespace " "
            DefineOperator
              Colon ":"
              Equal "="
            Newline "\n"
            Missing
"#;
  assert_eq!(tree, expected);

  let document = DocumentSyntax::cast(snapshot.syntax()).unwrap();
  assert_eq!(document.sections().len(), 1);
}

#[test]
fn missing_right_operand_after_plus_uses_additive_rule() {
  let snapshot = parse("x := 1 +\n");
  assert_lossless("x := 1 +\n", &snapshot);
  let diagnostic = snapshot.diagnostics.iter().next().unwrap();
  assert_eq!(diagnostic.code.as_str(), "syntax/missing-expression");
  assert_eq!(
    diagnostic.rule,
    Some(mech_syntax::document::parser::rule_id(
      "additive-expression"
    ))
  );
  assert_eq!(diagnostic.expected.len(), 1);
  assert!(diagnostic.recovery.is_some());
  assert_eq!(diagnostic.labels.len(), 1);
  assert_eq!(diagnostic.fixes.len(), 1);
  let json = snapshot.diagnostics.to_json().unwrap();
  assert!(json.contains("\"syntax/missing-expression\""));
  assert!(json.contains("\"recovery\""));
  assert!(json.contains("\"expected\""));
}

#[test]
fn missing_right_parenthesis_has_opening_label_and_safe_fix() {
  let snapshot = parse("x := (1 + 2\n");
  assert_lossless("x := (1 + 2\n", &snapshot);
  let diagnostic = snapshot
    .diagnostics
    .iter()
    .find(|diagnostic| diagnostic.code.as_str() == "syntax/unclosed-delimiter")
    .unwrap();
  assert_eq!(diagnostic.labels.len(), 1);
  assert_eq!(diagnostic.fixes.len(), 1);
  assert!(matches!(
    diagnostic.primary,
    DiagnosticAnchor::Element { .. }
  ));
  assert_eq!(
    nodes_of_kind(&snapshot.syntax(), SyntaxKind::ParentheticalExpression).len(),
    1
  );
}

#[test]
fn unexpected_mech_source_is_retained_under_error_node() {
  let snapshot = parse("x := 1 @@@\n");
  assert_lossless("x := 1 @@@\n", &snapshot);
  assert_eq!(
    diagnostic_codes(&snapshot),
    vec!["syntax/unexpected-token"]
  );
  let errors = nodes_of_kind(&snapshot.syntax(), SyntaxKind::Error);
  assert_eq!(errors.len(), 1);
  assert_eq!(errors[0].text().unwrap(), "@@@");
}

#[test]
fn recovery_preserves_later_paragraph_and_canonical_heading() {
  let text =
    "x := @@@\nordinary prose\n1. Recovered Section\n-------------------\nlater prose\n";
  let snapshot = parse(text);
  assert_lossless(text, &snapshot);
  assert_eq!(nodes_of_kind(&snapshot.syntax(), SyntaxKind::MechItem).len(), 1);
  assert_eq!(nodes_of_kind(&snapshot.syntax(), SyntaxKind::UlSubtitle).len(), 1);
  assert_eq!(nodes_of_kind(&snapshot.syntax(), SyntaxKind::Paragraph).len(), 2);
  assert_eq!(
    DocumentSyntax::cast(snapshot.syntax())
      .unwrap()
      .sections()
      .len(),
    2
  );
}

#[test]
fn malformed_paragraph_element_recovers_before_generic_fence() {
  let text = "`unterminated inline\n```text\nopaque := content\n```\n";
  let snapshot = parse(text);
  assert_lossless(text, &snapshot);
  assert!(diagnostic_codes(&snapshot)
    .contains(&"syntax/invalid-paragraph-element"));
  assert_eq!(nodes_of_kind(&snapshot.syntax(), SyntaxKind::GenericFence).len(), 1);
}

#[test]
fn unclosed_generic_fence_keeps_opaque_content() {
  let text = "~~~text\nx := not parsed here\n1. Not a heading\n----------------\n";
  let snapshot = parse(text);
  assert_lossless(text, &snapshot);
  assert_eq!(
    diagnostic_codes(&snapshot),
    vec!["syntax/unclosed-fence"]
  );
  assert_eq!(nodes_of_kind(&snapshot.syntax(), SyntaxKind::FenceContent).len(), 1);
  assert_eq!(nodes_of_kind(&snapshot.syntax(), SyntaxKind::Missing).len(), 1);
  assert_eq!(nodes_of_kind(&snapshot.syntax(), SyntaxKind::UlSubtitle).len(), 0);
}

#[test]
fn two_independent_errors_survive_across_heading_restart() {
  let text = "x :=\n1. Next\n--------\ny := (1\n";
  let snapshot = parse(text);
  assert_lossless(text, &snapshot);
  assert_eq!(
    diagnostic_codes(&snapshot),
    vec![
      "syntax/missing-expression",
      "syntax/unclosed-delimiter"
    ]
  );
  assert_eq!(nodes_of_kind(&snapshot.syntax(), SyntaxKind::UlSubtitle).len(), 1);
}

#[test]
fn unicode_and_emoji_do_not_corrupt_primary_error_span() {
  let text = "💡 := 1 +\n";
  let snapshot = parse(text);
  assert_lossless(text, &snapshot);
  let diagnostic = snapshot.diagnostics.iter().next().unwrap();
  let range = diagnostic
    .primary
    .resolve(snapshot.revision, &snapshot.nodes)
    .unwrap();
  assert_eq!(range.start.0, "💡 := 1 +".len() as u32);
  assert!(range.is_empty());
}

#[test]
fn eof_error_is_total_for_streamed_input() {
  let text = "x := 1 +";
  let snapshot = parse(text);
  assert_lossless(text, &snapshot);
  assert_eq!(
    diagnostic_codes(&snapshot),
    vec!["syntax/missing-expression"]
  );
}

#[test]
fn committed_mech_prefix_never_becomes_paragraph() {
  let snapshot = parse("x := @\n");
  assert_lossless("x := @\n", &snapshot);
  assert_eq!(nodes_of_kind(&snapshot.syntax(), SyntaxKind::MechItem).len(), 1);
  assert_eq!(nodes_of_kind(&snapshot.syntax(), SyntaxKind::Paragraph).len(), 0);
}

#[test]
fn separate_colon_and_equal_are_paragraph_text() {
  let text = "ordinary : = prose\n";
  let snapshot = parse(text);
  assert_lossless(text, &snapshot);
  assert!(snapshot.diagnostics.is_empty());
  assert_eq!(nodes_of_kind(&snapshot.syntax(), SyntaxKind::Paragraph).len(), 1);
}

#[test]
fn raw_define_operator_is_excluded_from_paragraph_text() {
  let text = " := raw\n";
  let snapshot = parse(text);
  assert_lossless(text, &snapshot);
  assert_eq!(
    diagnostic_codes(&snapshot),
    vec!["syntax/invalid-paragraph-element"]
  );
  assert_eq!(nodes_of_kind(&snapshot.syntax(), SyntaxKind::MechItem).len(), 0);
  assert_eq!(nodes_of_kind(&snapshot.syntax(), SyntaxKind::Error).len(), 1);
}

#[test]
fn parenthesized_canonical_subtitle_is_not_markdown_heading() {
  let text = "(1.1) Canonical subtitle\n# ordinary paragraph\n";
  let snapshot = parse(text);
  assert_lossless(text, &snapshot);
  assert_eq!(nodes_of_kind(&snapshot.syntax(), SyntaxKind::Subtitle).len(), 1);
  assert_eq!(nodes_of_kind(&snapshot.syntax(), SyntaxKind::Paragraph).len(), 1);
}
