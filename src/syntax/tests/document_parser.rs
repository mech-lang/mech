use mech_syntax::document::{
    AstNode, DiagnosticAnchor, DocumentId, DocumentSyntax, ExpectedSyntax, FixApplicability,
    NodeFlags, ParseConfig, RecoveryAction, Revision, SyntaxKind, SyntaxNode, TextSnapshot,
    VariableDefineSyntax, compact_debug_tree, parse_canonical_document, reconstruct_source,
    validate_lossless,
};

fn parse(text: &str) -> mech_syntax::document::SyntaxSnapshot {
    let source = TextSnapshot::new(DocumentId(42), Revision(0), text).unwrap();
    parse_canonical_document(source, ParseConfig::default())
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
    assert_eq!(
        reconstruct_source(&snapshot.root, &snapshot.source).unwrap(),
        text
    );
}

fn diagnostic_codes(snapshot: &mech_syntax::document::SyntaxSnapshot) -> Vec<&str> {
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
        vec!["syntax/missing-variable-definition-value"]
    );
    assert_eq!(
        nodes_of_kind(&snapshot.syntax(), SyntaxKind::Missing).len(),
        1
    );
    assert!(snapshot.root.flags.contains(NodeFlags::CONTAINS_MISSING));

    let tree = compact_debug_tree(&snapshot.syntax());
    let expected = include_str!("fixtures/document/trees/missing-rhs.tree");
    assert_eq!(tree, expected);

    let document = DocumentSyntax::cast(snapshot.syntax()).unwrap();
    assert_eq!(document.sections().len(), 1);
}

#[test]
fn missing_right_operand_after_plus_uses_canonical_rule_attribution() {
    let snapshot = parse("x := 1 +\n");
    assert_lossless("x := 1 +\n", &snapshot);
    let diagnostic = snapshot.diagnostics.iter().next().unwrap();
    assert_eq!(diagnostic.code.as_str(), "syntax/missing-operator-operand");
    assert!(diagnostic.rule.is_some());
    assert_eq!(diagnostic.context, None);
    assert_eq!(
        diagnostic.expected,
        vec![ExpectedSyntax::Production(String::from("expression"))]
    );
    assert!(matches!(
        diagnostic.recovery,
        Some(RecoveryAction::Insert { .. })
    ));
    // Canonical production insertion does not invent an expression or offer an
    // unsafe machine-applicable edit for an unknown right operand.
    assert!(diagnostic.labels.is_empty());
    assert!(diagnostic.fixes.is_empty());
    let json = snapshot.diagnostics.to_json().unwrap();
    assert!(json.contains("\"syntax/missing-operator-operand\""));
    assert!(json.contains("\"recovery\""));
    assert!(json.contains("\"expected\""));
}

#[test]
fn missing_right_parenthesis_has_owned_token_and_safe_fix() {
    let snapshot = parse("x := (1 + 2\n");
    assert_lossless("x := (1 + 2\n", &snapshot);
    let diagnostic = snapshot
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code.as_str() == "syntax/missing-delimiter")
        .unwrap();
    assert_eq!(diagnostic.labels.len(), 1);
    let boundary = diagnostic.labels[0]
        .anchor
        .resolve(snapshot.revision, &snapshot.nodes)
        .unwrap();
    assert!(boundary.is_empty());
    assert_eq!(
        diagnostic.labels[0].message,
        "closing delimiter expected before this boundary"
    );
    let opening = diagnostic
        .primary
        .resolve(snapshot.revision, &snapshot.nodes)
        .unwrap();
    assert_eq!(
        &snapshot.source.to_contiguous_string()[opening.start.0 as usize..opening.end.0 as usize],
        "("
    );
    assert_eq!(
        diagnostic.expected,
        vec![ExpectedSyntax::Token(SyntaxKind::RightParen)]
    );
    assert_eq!(diagnostic.fixes.len(), 1);
    let fix = &diagnostic.fixes[0];
    assert_eq!(fix.applicability, FixApplicability::MachineApplicable);
    assert_eq!(fix.edits.len(), 1);
    assert_eq!(fix.edits[0].insert, ")");
    assert!(fix.edits[0].delete.is_empty());
    assert!(fix.edits[0].delete.end <= snapshot.source.byte_len());
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
        vec!["syntax/unexpected-document-source"]
    );
    let errors = nodes_of_kind(&snapshot.syntax(), SyntaxKind::Error);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].text().unwrap(), ":= 1 @@@\n");
    assert_eq!(errors[0].range().start.0, 2);
    assert_eq!(errors[0].range().end, snapshot.source.byte_len());
}

#[test]
fn recovery_preserves_later_paragraph_and_canonical_heading() {
    let text = "x := @@@\nordinary prose\n1. Recovered Section\n-------------------\nlater prose\n";
    let snapshot = parse(text);
    assert_lossless(text, &snapshot);
    assert_eq!(
        nodes_of_kind(&snapshot.syntax(), SyntaxKind::VariableDefine).len(),
        1
    );
    assert_eq!(
        nodes_of_kind(&snapshot.syntax(), SyntaxKind::UlSubtitle).len(),
        1
    );
    assert_eq!(
        nodes_of_kind(&snapshot.syntax(), SyntaxKind::Paragraph)
            .into_iter()
            .map(|paragraph| paragraph.text().unwrap())
            .collect::<Vec<_>>(),
        ["ordinary prose", "Recovered Section", "later prose"]
    );
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
    assert!(diagnostic_codes(&snapshot).contains(&"syntax/unclosed-inline-code"));
    assert_eq!(
        nodes_of_kind(&snapshot.syntax(), SyntaxKind::CodeBlock).len(),
        1
    );
}

#[test]
fn unclosed_generic_fence_keeps_opaque_content() {
    let text = "~~~text\nx := not parsed here\n1. Not a heading\n----------------\n";
    let snapshot = parse(text);
    assert_lossless(text, &snapshot);
    assert_eq!(
        diagnostic_codes(&snapshot),
        vec!["syntax/missing-codeblock-sigil"]
    );
    let fences = nodes_of_kind(&snapshot.syntax(), SyntaxKind::CodeBlock);
    assert_eq!(fences.len(), 1);
    assert_eq!(fences[0].text().unwrap(), text);
    assert!(nodes_of_kind(&fences[0], SyntaxKind::VariableDefine).is_empty());
    assert_eq!(
        nodes_of_kind(&snapshot.syntax(), SyntaxKind::Missing).len(),
        1
    );
    assert_eq!(
        nodes_of_kind(&snapshot.syntax(), SyntaxKind::UlSubtitle).len(),
        0
    );
}

#[test]
fn two_independent_errors_survive_across_heading_restart() {
    // An explicit canonical statement terminal keeps the incomplete definition
    // separate from the following heading; expressions may otherwise span lines.
    let text = "x :=;\n1. Next\n--------\ny := (1\n";
    let snapshot = parse(text);
    assert_lossless(text, &snapshot);
    assert_eq!(
        diagnostic_codes(&snapshot),
        vec![
            "syntax/missing-variable-definition-value",
            "syntax/missing-delimiter"
        ]
    );
    assert_eq!(
        nodes_of_kind(&snapshot.syntax(), SyntaxKind::UlSubtitle).len(),
        1
    );
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
        vec!["syntax/missing-operator-operand"]
    );
}

#[test]
fn committed_mech_prefix_never_becomes_paragraph() {
    let snapshot = parse("x := @\n");
    assert_lossless("x := @\n", &snapshot);
    assert_eq!(
        nodes_of_kind(&snapshot.syntax(), SyntaxKind::VariableDefine).len(),
        1
    );
    assert_eq!(
        nodes_of_kind(&snapshot.syntax(), SyntaxKind::Paragraph).len(),
        0
    );
    let recovery = snapshot
        .diagnostics
        .iter()
        .next()
        .unwrap()
        .recovery
        .as_ref();
    let Some(RecoveryAction::Abandon { rule, .. }) = recovery else {
        panic!("distinctive Mech prefix must abandon to an ancestor restart root");
    };
    assert_eq!(
        mech_syntax::document::parser::canonical_rule_name(*rule),
        Some("variable-define")
    );
}

#[test]
fn definitions_handle_mutability_kind_annotations_and_typed_digits() {
    let text = "~x := 1_024u16\nx<u8> := 1u8\n";
    let snapshot = parse(text);
    assert_lossless(text, &snapshot);
    assert!(snapshot.diagnostics.is_empty());
    assert_eq!(
        nodes_of_kind(&snapshot.syntax(), SyntaxKind::VariableDefine).len(),
        2
    );
    assert_eq!(
        nodes_of_kind(&snapshot.syntax(), SyntaxKind::KindAnnotation).len(),
        1
    );
    assert_eq!(
        nodes_of_kind(&snapshot.syntax(), SyntaxKind::Paragraph).len(),
        0
    );
}

#[test]
fn missing_expression_is_not_a_typed_expression() {
    let snapshot = parse("x :=\n");
    let definition = nodes_of_kind(&snapshot.syntax(), SyntaxKind::VariableDefine)
        .into_iter()
        .next()
        .and_then(VariableDefineSyntax::cast)
        .unwrap();
    assert!(definition.value().is_none());
    assert!(definition.missing_value().is_some());
}

#[test]
fn malformed_items_restore_canonical_rule_attribution() {
    let snapshot = parse("x := @\ny := #\n`bad\n");
    assert!(!snapshot.diagnostics.is_empty());
    for diagnostic in snapshot.diagnostics.iter() {
        assert_eq!(diagnostic.context, None);
        if let Some(rule) = diagnostic.rule {
            assert!(
                mech_syntax::document::parser::canonical_rule_name(rule).is_some(),
                "diagnostic used a noncanonical RuleId: {rule}"
            );
        }
    }
}

#[test]
fn separate_colon_and_equal_are_paragraph_text() {
    let text = "ordinary : = prose\n";
    let snapshot = parse(text);
    assert_lossless(text, &snapshot);
    assert!(snapshot.diagnostics.is_empty());
    assert_eq!(
        nodes_of_kind(&snapshot.syntax(), SyntaxKind::Paragraph).len(),
        1
    );
}

#[test]
fn raw_define_operator_is_excluded_from_paragraph_text() {
    let text = " := raw\n";
    let snapshot = parse(text);
    assert_lossless(text, &snapshot);
    assert_eq!(
        diagnostic_codes(&snapshot),
        vec!["syntax/unexpected-document-source"]
    );
    assert_eq!(
        nodes_of_kind(&snapshot.syntax(), SyntaxKind::MechItem).len(),
        0
    );
    assert_eq!(
        nodes_of_kind(&snapshot.syntax(), SyntaxKind::Error).len(),
        1
    );
}

#[test]
fn parenthesized_canonical_subtitle_is_not_markdown_heading() {
    let text = "(1.1) Canonical subtitle\n# ordinary paragraph\n";
    let snapshot = parse(text);
    assert_lossless(text, &snapshot);
    assert_eq!(
        nodes_of_kind(&snapshot.syntax(), SyntaxKind::Subtitle).len(),
        1
    );
    assert_eq!(
        nodes_of_kind(&snapshot.syntax(), SyntaxKind::Paragraph).len(),
        1
    );
}
