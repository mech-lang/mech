use mech_syntax::document::{
  DiagnosticAnchor, DocumentSession, NodeMap, ParseConfig, Revision,
  SyntaxKind, SyntaxNode, TextEdit, TextRange, TextSize, TextSnapshot, compact_debug_tree,
  parse_document, reconstruct_source, validate_lossless,
};

fn find_node(root: &SyntaxNode, kind: SyntaxKind) -> Option<SyntaxNode> {
  if root.kind() == kind {
    return Some(root.clone());
  }
  root.children().find_map(|child| find_node(&child, kind))
}

fn find_nodes(root: &SyntaxNode, kind: SyntaxKind) -> Vec<SyntaxNode> {
  let mut nodes = Vec::new();
  if root.kind() == kind {
    nodes.push(root.clone());
  }
  for child in root.children() {
    nodes.extend(find_nodes(&child, kind));
  }
  nodes
}

fn full_parse(
  snapshot: &mech_syntax::document::SyntaxSnapshot,
) -> mech_syntax::document::SyntaxSnapshot {
  parse_document(
    TextSnapshot::new(
      snapshot.document,
      snapshot.revision,
      snapshot.source.to_contiguous_string().as_str(),
    )
    .unwrap(),
    ParseConfig::default(),
  )
}

fn normalized_diagnostics(
  snapshot: &mech_syntax::document::SyntaxSnapshot,
) -> Vec<String> {
  snapshot
    .diagnostics
    .iter()
    .map(|diagnostic| {
      let range = diagnostic
        .primary
        .resolve(snapshot.revision, &snapshot.nodes);
      format!(
        "{}|{:?}|{:?}|{:?}|{:?}|{:?}",
        diagnostic.code.as_str(),
        diagnostic.rule,
        range,
        diagnostic.expected,
        diagnostic.found,
        diagnostic.recovery
      )
    })
    .collect()
}

fn assert_incremental_equals_full(
  snapshot: &mech_syntax::document::SyntaxSnapshot,
) {
  let full = full_parse(snapshot);
  assert_eq!(
    compact_debug_tree(&snapshot.syntax()),
    compact_debug_tree(&full.syntax())
  );
  assert_eq!(
    normalized_diagnostics(snapshot),
    normalized_diagnostics(&full)
  );
  validate_lossless(&snapshot.root, &snapshot.source).unwrap();
  assert_eq!(
    reconstruct_source(&snapshot.root, &snapshot.source).unwrap(),
    snapshot.source.to_contiguous_string()
  );
}

#[test]
fn required_edit_sequence_reparses_only_mech_item_and_reuses_later_section() {
  let initial = "Intro paragraph\n1. Code\n-------\nx := 1\n1. Later\n--------\nLater paragraph\n";
  let mut session = DocumentSession::new(initial, ParseConfig::default());
  let initial_snapshot = session.snapshot();
  assert_incremental_equals_full(initial_snapshot);
  let sections = find_nodes(&initial_snapshot.syntax(), SyntaxKind::Section);
  assert_eq!(sections.len(), 3);
  let later_section_id = sections[2].id();
  let intro_paragraph_id =
    find_node(&initial_snapshot.syntax(), SyntaxKind::Paragraph)
      .unwrap()
      .id();
  let original_mech =
    find_node(&initial_snapshot.syntax(), SyntaxKind::MechItem).unwrap();
  let original_mech_id = original_mech.id();

  let insert_at = initial.find("1\n1. Later").unwrap() + 1;
  let update = session.apply_edits(&[TextEdit::insert(
    TextSize(insert_at as u32),
    " +",
  )]);
  let middle = session.snapshot();
  assert_incremental_equals_full(middle);
  assert!(!middle.is_strictly_clean());
  assert_eq!(
    middle
      .diagnostics
      .iter()
      .map(|diagnostic| diagnostic.code.as_str())
      .collect::<Vec<_>>(),
    vec!["syntax/missing-expression"]
  );
  assert_eq!(update.reparsed_roots, vec![original_mech_id]);
  assert_eq!(update.stats.reparse_root_count, 1);
  assert_eq!(update.stats.document_fallbacks, 0);
  assert!(update.stats.reused_node_count > 0);
  assert!(
    update.stats.parser_steps
      < full_parse(middle).stats.parser_steps
  );
  assert_eq!(
    find_nodes(&middle.syntax(), SyntaxKind::Section)[2].id(),
    later_section_id
  );
  assert_eq!(
    find_node(&middle.syntax(), SyntaxKind::Paragraph)
      .unwrap()
      .id(),
    intro_paragraph_id
  );

  let middle_text = middle.source.to_contiguous_string();
  let expression_start = middle_text.find("1 +").unwrap();
  let update = session.apply_edits(&[TextEdit::replace(
    TextRange::new(
      TextSize(expression_start as u32),
      TextSize((expression_start + 3) as u32),
    ),
    "2",
  )]);
  let final_snapshot = session.snapshot();
  assert_incremental_equals_full(final_snapshot);
  assert!(final_snapshot.is_strictly_clean());
  assert_eq!(
    final_snapshot.source.to_contiguous_string(),
    initial.replace("x := 1", "x := 2")
  );
  assert_eq!(
    find_nodes(&final_snapshot.syntax(), SyntaxKind::Section)[2].id(),
    later_section_id
  );
  assert!(update.stats.reused_node_count > 0);
  assert_eq!(update.stats.document_fallbacks, 0);
}

#[test]
fn annotations_on_reused_nodes_survive_and_replaced_nodes_are_invalidated() {
  let text = "first paragraph\n1. Later\n--------\nlater paragraph\n";
  let mut session = DocumentSession::new(text, ParseConfig::default());
  let paragraphs = find_nodes(&session.snapshot().syntax(), SyntaxKind::Paragraph);
  let first = paragraphs[0].id();
  let later = paragraphs[1].id();
  let mut annotations = NodeMap::new(Revision(0));
  annotations.insert(first, "first");
  annotations.insert(later, "later");

  let start = text.find("first").unwrap();
  session.apply_edits(&[TextEdit::replace(
    TextRange::new(TextSize(start as u32), TextSize((start + 5) as u32)),
    "opening",
  )]);
  let snapshot = session.snapshot();
  let annotations =
    annotations.retain_reused(snapshot.revision, &snapshot.nodes);
  assert!(!annotations.contains_key(first));
  assert_eq!(annotations.get(later), Some(&"later"));
  assert!(snapshot.nodes.node(later).is_some());
}

#[test]
fn diagnostics_on_reused_nodes_shift_with_earlier_edits() {
  let text = "prefix paragraph\n1. Code\n-------\ny := 1 +\n";
  let mut session = DocumentSession::new(text, ParseConfig::default());
  let diagnostic = session.snapshot().diagnostics.iter().next().unwrap();
  let DiagnosticAnchor::Element { element, .. } = diagnostic.primary else {
    panic!("parser diagnostic should use a structural anchor");
  };
  let old_range = diagnostic
    .primary
    .resolve(session.snapshot().revision, &session.snapshot().nodes)
    .unwrap();
  let old_id = diagnostic.id;

  session.apply_edits(&[TextEdit::insert(TextSize(0), "💡 ")]);
  let snapshot = session.snapshot();
  let diagnostic = snapshot
    .diagnostics
    .iter()
    .find(|diagnostic| diagnostic.id == old_id)
    .unwrap();
  let new_range = diagnostic
    .primary
    .resolve(snapshot.revision, &snapshot.nodes)
    .unwrap();
  assert_eq!(new_range.start.0, old_range.start.0 + 5);
  assert!(snapshot.nodes.range(element).is_some());
  assert_incremental_equals_full(snapshot);
}

#[test]
fn deleted_nodes_no_longer_resolve() {
  let text = "delete this paragraph\nkeep this paragraph\n";
  let mut session = DocumentSession::new(text, ParseConfig::default());
  let deleted =
    find_node(&session.snapshot().syntax(), SyntaxKind::Paragraph)
      .unwrap()
      .id();
  let end = text.find('\n').unwrap() + 1;
  session.apply_edits(&[TextEdit::delete(TextRange::new(
    TextSize(0),
    TextSize(end as u32),
  ))]);
  assert!(session.snapshot().nodes.node(deleted).is_none());
  assert_incremental_equals_full(session.snapshot());
}

#[test]
fn deterministic_edit_sequence_matches_full_parse_each_revision() {
  let mut session = DocumentSession::new(
    "1. Code\n-------\nx := 1\n1. Later\n--------\nstable\n",
    ParseConfig::default(),
  );
  for replacement in ["2", "3 +", "4", "(5", "(6)", "7"] {
    let text = session.snapshot().source.to_contiguous_string();
    let start = text.find("x := ").unwrap() + 5;
    let end = text[start..].find('\n').unwrap() + start;
    session.apply_edits(&[TextEdit::replace(
      TextRange::new(TextSize(start as u32), TextSize(end as u32)),
      replacement,
    )]);
    assert_incremental_equals_full(session.snapshot());
  }
}

#[test]
fn fence_content_edit_reuses_later_section_and_never_treats_inner_heading_as_restart() {
  let text = "~~~text\nopaque\n1. Inner\n--------\n~~~\n1. Later\n--------\nstable\n";
  let mut session = DocumentSession::new(text, ParseConfig::default());
  let later_section =
    find_nodes(&session.snapshot().syntax(), SyntaxKind::Section)[1].id();
  let opaque = text.find("opaque").unwrap();
  let update = session.apply_edits(&[TextEdit::replace(
    TextRange::new(
      TextSize(opaque as u32),
      TextSize((opaque + "opaque".len()) as u32),
    ),
    "changed",
  )]);
  assert_eq!(update.stats.document_fallbacks, 0);
  assert_eq!(
    find_nodes(&session.snapshot().syntax(), SyntaxKind::Section)[1].id(),
    later_section
  );
  assert_eq!(
    find_nodes(&session.snapshot().syntax(), SyntaxKind::UlSubtitle).len(),
    1
  );
  assert_incremental_equals_full(session.snapshot());
}

#[test]
fn repeated_eof_appends_behave_like_streamed_editing() {
  let mut session =
    DocumentSession::new("x := 1 +", ParseConfig::default());
  assert!(!session.snapshot().is_strictly_clean());
  for appended in [" ", "2", "\r", "\n", "streamed ", "💡"] {
    let at = session.snapshot().source.byte_len();
    session.apply_edits(&[TextEdit::insert(at, appended)]);
    assert_incremental_equals_full(session.snapshot());
  }
  assert_eq!(
    session.snapshot().source.to_contiguous_string(),
    "x := 1 + 2\r\nstreamed 💡"
  );
  assert!(session.snapshot().is_strictly_clean());
}
