use mech_syntax::document::parser::Cursor;
use mech_syntax::document::{
  DocumentId, Revision, TextRange, TextSize, TextSnapshot,
};

fn piece_snapshot(parts: &[&str]) -> TextSnapshot {
  let mut source =
    TextSnapshot::new(DocumentId(201), Revision(0), "").unwrap();
  for part in parts {
    source = source.append((*part).to_owned()).unwrap();
  }
  assert_eq!(source.piece_count(), parts.len());
  source
}

fn assert_one_cross_piece_grapheme(parts: &[&str]) {
  let source = piece_snapshot(parts);
  let expected = source.full_range();
  let mut cursor = Cursor::new(&source);

  assert_eq!(cursor.context_peek_grapheme_range(), Some(expected));
  assert_eq!(cursor.peek_grapheme_range(), Some(expected));
  assert_eq!(cursor.bump_grapheme(), Some(expected));
  assert!(cursor.is_eof());
  assert_eq!(cursor.peek_grapheme_range(), None);
}

#[test]
fn combining_sequence_crosses_pieces() {
  assert_one_cross_piece_grapheme(&["e", "\u{301}"]);
}

#[test]
fn emoji_variation_selector_crosses_pieces() {
  assert_one_cross_piece_grapheme(&["\u{2764}", "\u{fe0f}"]);
}

#[test]
fn zwj_family_emoji_crosses_every_piece() {
  assert_one_cross_piece_grapheme(&[
    "\u{1f468}",
    "\u{200d}",
    "\u{1f469}",
    "\u{200d}",
    "\u{1f467}",
    "\u{200d}",
    "\u{1f466}",
  ]);
}

#[test]
fn regional_indicator_flag_crosses_pieces() {
  assert_one_cross_piece_grapheme(&["\u{1f1fa}", "\u{1f1f8}"]);
}

#[test]
fn crlf_crosses_pieces() {
  assert_one_cross_piece_grapheme(&["\r", "\n"]);
}

#[test]
fn ordinary_ascii_preserves_separate_graphemes_across_pieces() {
  let source = piece_snapshot(&["a", "b"]);
  let mut cursor = Cursor::new(&source);

  assert_eq!(
    cursor.bump_grapheme(),
    Some(TextRange::new(TextSize(0), TextSize(1)))
  );
  assert_eq!(
    cursor.bump_grapheme(),
    Some(TextRange::new(TextSize(1), TextSize(2)))
  );
  assert!(cursor.is_eof());
}

#[test]
fn consume_bound_does_not_split_a_context_grapheme() {
  let source = piece_snapshot(&["e", "\u{301}", "x"]);
  let mut cursor = Cursor::for_range(
    &source,
    TextRange::new(TextSize::ZERO, TextSize(1)),
  );
  let complete = TextRange::new(TextSize::ZERO, TextSize(3));

  assert_eq!(cursor.context_peek_grapheme_range(), Some(complete));
  assert_eq!(cursor.peek_grapheme_range(), None);
  assert_eq!(cursor.bump_grapheme(), None);
  assert_eq!(cursor.offset(), TextSize::ZERO);
}
