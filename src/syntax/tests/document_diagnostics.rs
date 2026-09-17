use mech_syntax::document::{
  Diagnostic, DiagnosticAnchor, DiagnosticCode, DiagnosticFix, DiagnosticId, DiagnosticLabel,
  DiagnosticPhase, DiagnosticStore, DiagnosticTags, DocumentId, ExpectedSyntax, FixApplicability,
  FoundSyntax, GreenBuilder, IdGenerator, NodeIndex, RecoveryAction, Revision, Severity,
  SyntaxElementId, SyntaxKind, TextEdit, TextRange, TextSize, TextSnapshot, normalize_diagnostics,
  render_plain,
};

fn paragraph_tree(text: &str) -> (std::sync::Arc<mech_syntax::document::GreenNode>, IdGenerator) {
  let mut ids = IdGenerator::new();
  let mut builder = GreenBuilder::new(&mut ids);
  builder.start_node(SyntaxKind::Paragraph);
  builder.token(SyntaxKind::Text, text).unwrap();
  builder.finish_node().unwrap();
  (builder.finish().unwrap(), ids)
}

#[test]
fn structural_anchor_moves_with_reused_node() {
  let (paragraph, mut ids) = paragraph_tree("later");
  let mut root_builder = GreenBuilder::new(&mut ids);
  root_builder.start_node(SyntaxKind::Document);
  root_builder.token(SyntaxKind::Text, "💡 ").unwrap();
  root_builder.reuse_node(paragraph.clone()).unwrap();
  root_builder.finish_node().unwrap();
  let root = root_builder.finish().unwrap();
  let index = NodeIndex::build(&root);
  let anchor = DiagnosticAnchor::Element {
    element: SyntaxElementId::Node(paragraph.id),
    relative: TextRange::new(TextSize(1), TextSize(3)),
  };

  assert_eq!(
    anchor.resolve(Revision(1), &index),
    Some(TextRange::new(TextSize(6), TextSize(8)))
  );
}

#[test]
fn structured_diagnostic_serializes_and_renders() {
  let text = "x := 1 +";
  let source =
    TextSnapshot::new(DocumentId(9), Revision(2), text).unwrap();
  let (root, _) = paragraph_tree(text);
  let index = NodeIndex::build(&root);
  let diagnostic = Diagnostic {
    id: DiagnosticId(1),
    code: DiagnosticCode::syntax("missing-expression"),
    phase: DiagnosticPhase::Syntax,
    severity: Severity::Error,
    rule: mech_syntax::document::parser::canonical_rule_id("expression"),
    context: Some(mech_syntax::document::parser::parser_context_id(
      "prototype-expression",
    )),
    primary: DiagnosticAnchor::Absolute {
      revision: Revision(2),
      range: TextRange::empty(TextSize(text.len() as u32)),
    },
    labels: vec![DiagnosticLabel {
      anchor: DiagnosticAnchor::Absolute {
        revision: Revision(2),
        range: TextRange::new(TextSize(7), TextSize(8)),
      },
      message: "`+` requires a right operand".into(),
    }],
    expected: vec![ExpectedSyntax::Production("expression".into())],
    found: Some(FoundSyntax {
      kind: Some(SyntaxKind::Eof),
      text: None,
    }),
    fixes: vec![DiagnosticFix {
      title: "Insert an expression".into(),
      applicability: FixApplicability::HasPlaceholders,
      edits: vec![TextEdit::insert(TextSize(text.len() as u32), " _")],
    }],
    related: vec![],
    recovery: Some(RecoveryAction::Insert {
      syntax: ExpectedSyntax::Production("expression".into()),
      at: TextSize(text.len() as u32),
    }),
    tags: DiagnosticTags::NONE,
    message: "expected an expression after `+`".into(),
  };
  let mut store = DiagnosticStore::new(Revision(2));
  store.push(diagnostic.clone());

  let json = store.to_json().unwrap();
  assert!(json.contains("\"syntax/missing-expression\""));
  assert!(json.contains("\"machine-applicable\"") || json.contains("\"has-placeholders\""));
  assert!(json.contains("\"recovery\""));

  let rendered = render_plain(&diagnostic, &source, &index);
  assert!(rendered.contains("syntax/missing-expression"));
  assert!(rendered.contains("1:9"));
  assert!(rendered.contains("right operand"));

  let normalized = normalize_diagnostics(&store, Revision(2), &index);
  assert_eq!(normalized.len(), 1);
  assert_eq!(normalized[0].code, diagnostic.code);
  assert_eq!(normalized[0].phase, diagnostic.phase);
  assert_eq!(normalized[0].severity, diagnostic.severity);
  assert_eq!(normalized[0].rule, diagnostic.rule);
  assert_eq!(normalized[0].context, diagnostic.context);
  assert_eq!(normalized[0].labels.len(), 1);
  assert_eq!(normalized[0].expected, diagnostic.expected);
  assert_eq!(normalized[0].found, diagnostic.found);
  assert_eq!(normalized[0].fixes.len(), 1);
  assert_eq!(normalized[0].recovery, diagnostic.recovery);
  assert_eq!(normalized[0].tags, diagnostic.tags);
}
