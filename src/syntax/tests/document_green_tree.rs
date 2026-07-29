use mech_syntax::document::{
  DocumentId, GreenBuilder, IdGenerator, NodeFlags, NodeIndex, Revision, SyntaxElementId,
  SyntaxKind, TextSnapshot, TokenFlags, reconstruct_source, validate_lossless,
};

fn source(text: &str) -> TextSnapshot {
  TextSnapshot::new(DocumentId(3), Revision(0), text).unwrap()
}

#[test]
fn reconstructs_unicode_whitespace_comments_and_delimiters() {
  let text = "x := 💡 -- note\r\n";
  let source = source(text);
  let mut ids = IdGenerator::new();
  let mut builder = GreenBuilder::new(&mut ids);
  builder.start_node(SyntaxKind::Document);
  builder.start_node(SyntaxKind::MechItem);
  builder.start_node(SyntaxKind::VariableDefine);
  builder.start_node(SyntaxKind::Identifier);
  builder.token(SyntaxKind::IdentifierToken, "x").unwrap();
  builder.finish_node().unwrap();
  builder.token(SyntaxKind::Whitespace, " ").unwrap();
  builder.start_node(SyntaxKind::DefineOperator);
  builder.token(SyntaxKind::Colon, ":").unwrap();
  builder.token(SyntaxKind::Equal, "=").unwrap();
  builder.finish_node().unwrap();
  builder.token(SyntaxKind::Whitespace, " ").unwrap();
  builder.start_node(SyntaxKind::Expression);
  builder.token(SyntaxKind::Text, "💡").unwrap();
  builder.finish_node().unwrap();
  builder.token(SyntaxKind::Whitespace, " ").unwrap();
  builder.start_node(SyntaxKind::Comment);
  builder.token(SyntaxKind::CommentToken, "-- note").unwrap();
  builder.finish_node().unwrap();
  builder.token(SyntaxKind::Newline, "\r\n").unwrap();
  builder.finish_node().unwrap();
  builder.finish_node().unwrap();
  builder.finish_node().unwrap();
  let root = builder.finish().unwrap();

  validate_lossless(&root, &source).unwrap();
  assert_eq!(reconstruct_source(&root, &source).unwrap(), text);
  assert_eq!(root.text_len, source.byte_len());
}

#[test]
fn missing_tokens_are_zero_width_and_error_nodes_retain_source() {
  let source = source("x := @");
  let mut ids = IdGenerator::new();
  let mut builder = GreenBuilder::new(&mut ids);
  builder.start_node(SyntaxKind::Document);
  builder.token(SyntaxKind::IdentifierToken, "x").unwrap();
  builder.token(SyntaxKind::Whitespace, " ").unwrap();
  builder.token(SyntaxKind::Colon, ":").unwrap();
  builder.token(SyntaxKind::Equal, "=").unwrap();
  builder.token(SyntaxKind::Whitespace, " ").unwrap();
  builder.start_node_with_flags(SyntaxKind::Error, NodeFlags::ERROR);
  builder
    .token_with_flags(SyntaxKind::Unknown, "@", TokenFlags::ERROR)
    .unwrap();
  builder.finish_node().unwrap();
  builder.start_node_with_flags(SyntaxKind::Missing, NodeFlags::MISSING);
  builder.missing_token(SyntaxKind::IdentifierToken).unwrap();
  let missing = builder.finish_node().unwrap();
  builder.finish_node().unwrap();
  let root = builder.finish().unwrap();

  assert_eq!(missing.text_len.0, 0);
  assert!(missing.flags.contains(NodeFlags::MISSING));
  assert!(root.flags.contains(NodeFlags::CONTAINS_ERROR));
  assert!(root.flags.contains(NodeFlags::CONTAINS_MISSING));
  validate_lossless(&root, &source).unwrap();
  assert_eq!(reconstruct_source(&root, &source).unwrap(), "x := @");
}

#[test]
fn reused_green_nodes_keep_ids_and_shift_in_the_red_index() {
  let mut ids = IdGenerator::new();
  let mut paragraph_builder = GreenBuilder::new(&mut ids);
  paragraph_builder.start_node(SyntaxKind::Paragraph);
  paragraph_builder.token(SyntaxKind::Text, "later").unwrap();
  paragraph_builder.finish_node().unwrap();
  let paragraph = paragraph_builder.finish().unwrap();
  let paragraph_id = paragraph.id;
  let token_id = match &paragraph.children[0] {
    mech_syntax::document::GreenElement::Token(token) => token.id,
    _ => unreachable!(),
  };

  let mut root_builder = GreenBuilder::new(&mut ids);
  root_builder.start_node(SyntaxKind::Document);
  root_builder.token(SyntaxKind::Text, "x ").unwrap();
  root_builder.reuse_node(paragraph.clone()).unwrap();
  root_builder.finish_node().unwrap();
  let root = root_builder.finish().unwrap();
  let index = NodeIndex::build(&root);

  assert_eq!(paragraph.id, paragraph_id);
  assert_eq!(
    index.range(SyntaxElementId::Node(paragraph_id)).unwrap().start.0,
    2
  );
  assert_eq!(
    index.range(SyntaxElementId::Token(token_id)).unwrap().start.0,
    2
  );
  validate_lossless(&root, &source("x later")).unwrap();
}
