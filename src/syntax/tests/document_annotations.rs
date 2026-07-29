use mech_syntax::document::{
  DocumentId, GreenBuilder, IdGenerator, NodeIndex, NodeMap, Revision, SyntaxKind, TextSnapshot,
  validate_lossless,
};

#[test]
fn typed_annotations_survive_reuse_and_deleted_nodes_are_dropped() {
  let mut ids = IdGenerator::new();
  let mut paragraph_builder = GreenBuilder::new(&mut ids);
  paragraph_builder.start_node(SyntaxKind::Paragraph);
  paragraph_builder.token(SyntaxKind::Text, "stable").unwrap();
  paragraph_builder.finish_node().unwrap();
  let paragraph = paragraph_builder.finish().unwrap();

  let mut annotations = NodeMap::new(Revision(0));
  annotations.insert(paragraph.id, String::from("dimension checked"));

  let mut reused_builder = GreenBuilder::new(&mut ids);
  reused_builder.start_node(SyntaxKind::Document);
  reused_builder.token(SyntaxKind::Text, "prefix ").unwrap();
  reused_builder.reuse_node(paragraph.clone()).unwrap();
  reused_builder.finish_node().unwrap();
  let reused_root = reused_builder.finish().unwrap();
  let reused_index = NodeIndex::build(&reused_root);
  let annotations = annotations.retain_reused(Revision(1), &reused_index);

  assert_eq!(
    annotations.get(paragraph.id).map(String::as_str),
    Some("dimension checked")
  );
  assert_eq!(annotations.revision, Revision(1));
  let source =
    TextSnapshot::new(DocumentId(1), Revision(1), "prefix stable").unwrap();
  validate_lossless(&reused_root, &source).unwrap();

  let mut replaced_builder = GreenBuilder::new(&mut ids);
  replaced_builder.start_node(SyntaxKind::Document);
  replaced_builder.token(SyntaxKind::Text, "replacement").unwrap();
  replaced_builder.finish_node().unwrap();
  let replaced_root = replaced_builder.finish().unwrap();
  let replaced_index = NodeIndex::build(&replaced_root);
  let annotations = annotations.retain_reused(Revision(2), &replaced_index);
  assert!(annotations.is_empty());
}
