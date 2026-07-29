use mech_syntax::document::{
  DocumentId, FragmentKind, IdGenerator, ParseConfig, ParseContext, ParseLimits, ParseMode,
  Revision, SyntaxKind, TextRange, TextSize, TextSnapshot, normalize_diagnostics_in_range,
  parse_document, parse_fragment, reconstruct_source_range, validate_lossless_range,
};

fn nodes_of_kind(
  root: &mech_syntax::document::SyntaxNode,
  kind: SyntaxKind,
) -> Vec<mech_syntax::document::SyntaxNode> {
  let mut nodes = Vec::new();
  if root.kind() == kind {
    nodes.push(root.clone());
  }
  for child in root.children() {
    nodes.extend(nodes_of_kind(&child, kind));
  }
  nodes
}

fn fragment_case(fragment: &str, kind: FragmentKind) {
  let prefix = "unrelated prefix\n";
  let complete = format!("{prefix}{fragment}unrelated suffix\n");
  let source =
    TextSnapshot::new(DocumentId(77), Revision(9), complete.as_str()).unwrap();
  let start = prefix.len() as u32;
  let range = TextRange::new(
    TextSize(start),
    TextSize(start + fragment.len() as u32),
  );
  let mut ids = IdGenerator::new();
  let snapshot = parse_fragment(
    &source,
    range,
    kind,
    ParseContext::for_kind(kind),
    ParseConfig::default(),
    &mut ids,
  );

  assert!(snapshot.matched, "{kind:?} did not match");
  assert!(snapshot.consumed_complete, "{kind:?} did not consume its range");
  assert_eq!(snapshot.source.document(), DocumentId(77));
  assert_eq!(snapshot.source.revision(), Revision(9));
  assert_eq!(snapshot.range, range);
  assert_eq!(snapshot.consumed, range);
  assert_eq!(snapshot.root.kind, kind.syntax_kind());
  assert_eq!(snapshot.syntax().range(), range);
  assert_eq!(snapshot.syntax().text().unwrap(), fragment);
  validate_lossless_range(&snapshot.root, &snapshot.source, range).unwrap();
  assert_eq!(
    reconstruct_source_range(&snapshot.root, &snapshot.source, range).unwrap(),
    fragment
  );
}

#[test]
fn every_bounded_fragment_entry_point_returns_its_requested_root() {
  fragment_case("1. Head\n--------\nx := 1\n", FragmentKind::Section);
  fragment_case("plain paragraph\n", FragmentKind::SectionElement);
  fragment_case("plain paragraph\n", FragmentKind::Paragraph);
  fragment_case("x := 1\n", FragmentKind::MechItem);
  fragment_case("x := 1", FragmentKind::VariableDefine);
  fragment_case("1 + 2", FragmentKind::Expression);
  fragment_case("(1 + 2)", FragmentKind::ParentheticalTerm);
  fragment_case("~~~text\nopaque\n~~~\n", FragmentKind::CodeBlock);

  let source = TextSnapshot::new(DocumentId(77), Revision(9), "plain\n").unwrap();
  let mut ids = IdGenerator::new();
  let document = parse_fragment(
    &source,
    source.full_range(),
    FragmentKind::Document,
    ParseContext::for_kind(FragmentKind::Document),
    ParseConfig::default(),
    &mut ids,
  );
  assert!(document.consumed_complete);
  assert_eq!(document.root.kind, SyntaxKind::Document);
}

#[test]
fn fragment_mode_is_part_of_the_parse_contract() {
  let source = TextSnapshot::new(DocumentId(8), Revision(3), "x := 1").unwrap();
  let mut ids = IdGenerator::new();
  let snapshot = parse_fragment(
    &source,
    source.full_range(),
    FragmentKind::Expression,
    ParseContext {
      mode: ParseMode::Paragraph,
      ..ParseContext::for_kind(FragmentKind::Expression)
    },
    ParseConfig::default(),
    &mut ids,
  );
  assert!(!snapshot.matched);
  assert!(!snapshot.consumed_complete);
}

#[test]
fn section_fragment_stops_before_a_second_section() {
  let text = "1. One\n---\nfirst\n2. Two\n---\nsecond\n";
  let source = TextSnapshot::new(DocumentId(8), Revision(3), text).unwrap();
  let mut ids = IdGenerator::new();
  let snapshot = parse_fragment(
    &source,
    source.full_range(),
    FragmentKind::Section,
    ParseContext::for_kind(FragmentKind::Section),
    ParseConfig::default(),
    &mut ids,
  );
  assert!(snapshot.matched);
  assert!(!snapshot.consumed_complete);
  assert_eq!(
    snapshot.consumed.end,
    TextSize(text.find("2. Two").unwrap() as u32)
  );
}

#[test]
fn fragment_diagnostic_uses_heading_right_context() {
  let text =
    "1. Code\n-------\n\nx :=\n2. Later\n--------\nstable\n";
  let source = TextSnapshot::new(DocumentId(8), Revision(3), text).unwrap();
  let start = TextSize(text.find("x :=").unwrap() as u32);
  let end = TextSize(text.find("2. Later").unwrap() as u32);
  let range = TextRange::new(start, end);
  let full = parse_document(source.clone(), ParseConfig::default());
  let mut ids = IdGenerator::new();
  let fragment = parse_fragment(
    &source,
    range,
    FragmentKind::VariableDefine,
    ParseContext::for_kind(FragmentKind::VariableDefine),
    ParseConfig::default(),
    &mut ids,
  );

  assert!(fragment.matched);
  assert!(fragment.consumed_complete);
  let fragment_diagnostics = normalize_diagnostics_in_range(
    &fragment.diagnostics,
    fragment.source.revision(),
    &fragment.nodes,
    range,
  );
  let full_diagnostics = normalize_diagnostics_in_range(
    &full.diagnostics,
    full.revision,
    &full.nodes,
    range,
  );
  assert_eq!(fragment_diagnostics, full_diagnostics);
  assert_eq!(fragment_diagnostics.len(), 1);
  assert_eq!(
    fragment_diagnostics[0]
      .found
      .as_ref()
      .and_then(|found| found.text.as_deref()),
    Some("2")
  );
  assert!(!fragment_diagnostics[0].fixes.is_empty());
  assert!(fragment_diagnostics[0].recovery.is_some());
}

#[test]
fn restart_entries_record_actual_enclosing_parenthetical_depth() {
  let source =
    TextSnapshot::new(DocumentId(8), Revision(3), "x := (((1)))\n").unwrap();
  let snapshot = parse_document(source, ParseConfig::default());
  let parentheticals =
    nodes_of_kind(&snapshot.syntax(), SyntaxKind::ParentheticalExpression);
  assert_eq!(parentheticals.len(), 3);
  let depths = parentheticals
    .iter()
    .map(|node| snapshot.restarts.get(node.id()).unwrap().delimiter_depth)
    .collect::<Vec<_>>();
  assert_eq!(depths, vec![0, 1, 2]);
}

#[test]
fn fragment_nesting_limit_uses_actual_enclosing_depth() {
  let config = ParseConfig {
    limits: ParseLimits {
      max_nesting: 1,
      ..ParseLimits::default()
    },
  };

  let top_text = "x := (1)\n";
  let top_source =
    TextSnapshot::new(DocumentId(8), Revision(3), top_text).unwrap();
  let top_range = TextRange::new(TextSize(5), TextSize(8));
  let top_full = parse_document(top_source.clone(), config);
  let mut ids = IdGenerator::new();
  let top_fragment = parse_fragment(
    &top_source,
    top_range,
    FragmentKind::ParentheticalTerm,
    ParseContext {
      delimiter_depth: 0,
      ..ParseContext::for_kind(FragmentKind::ParentheticalTerm)
    },
    config,
    &mut ids,
  );
  assert!(top_fragment.matched);
  assert!(top_fragment.consumed_complete);
  validate_lossless_range(
    &top_fragment.root,
    &top_fragment.source,
    top_range,
  )
  .unwrap();
  assert_eq!(
    normalize_diagnostics_in_range(
      &top_fragment.diagnostics,
      top_fragment.source.revision(),
      &top_fragment.nodes,
      top_range,
    ),
    normalize_diagnostics_in_range(
      &top_full.diagnostics,
      top_full.revision,
      &top_full.nodes,
      top_range,
    )
  );

  let nested_text = "x := ((1))\n";
  let nested_source =
    TextSnapshot::new(DocumentId(8), Revision(4), nested_text).unwrap();
  let inner_range = TextRange::new(TextSize(6), TextSize(9));
  let nested_full = parse_document(nested_source.clone(), config);
  let mut ids = IdGenerator::new();
  let inner_fragment = parse_fragment(
    &nested_source,
    inner_range,
    FragmentKind::ParentheticalTerm,
    ParseContext {
      delimiter_depth: 1,
      ..ParseContext::for_kind(FragmentKind::ParentheticalTerm)
    },
    config,
    &mut ids,
  );
  assert!(inner_fragment.matched);
  assert!(inner_fragment.consumed_complete);
  validate_lossless_range(
    &inner_fragment.root,
    &inner_fragment.source,
    inner_range,
  )
  .unwrap();
  let fragment_diagnostics = normalize_diagnostics_in_range(
    &inner_fragment.diagnostics,
    inner_fragment.source.revision(),
    &inner_fragment.nodes,
    inner_range,
  );
  let full_diagnostics = normalize_diagnostics_in_range(
    &nested_full.diagnostics,
    nested_full.revision,
    &nested_full.nodes,
    inner_range,
  );
  assert_eq!(fragment_diagnostics, full_diagnostics);
  assert_eq!(fragment_diagnostics.len(), 1);
  assert_eq!(
    fragment_diagnostics[0].code.as_str(),
    "syntax/nesting-limit"
  );
}
