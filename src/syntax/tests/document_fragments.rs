use mech_syntax::document::{
  DocumentId, FragmentKind, IdGenerator, ParseConfig, ParseContext, ParseMode, Revision,
  SyntaxKind, TextRange, TextSize, TextSnapshot, parse_fragment, reconstruct_source_range,
  validate_lossless_range,
};

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
