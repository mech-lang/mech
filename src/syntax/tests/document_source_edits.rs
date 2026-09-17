use mech_syntax::document::{
  DocumentId, Revision, SourceError, TextEdit, TextRange, TextSize, TextSnapshot,
};

fn snapshot(text: &str) -> TextSnapshot {
  TextSnapshot::new(DocumentId(7), Revision(0), text).unwrap()
}

fn range(start: u32, end: u32) -> TextRange {
  TextRange::new(TextSize(start), TextSize(end))
}

#[test]
fn insert_delete_replace_and_append_preserve_exact_bytes() {
  let source = snapshot("alpha 💡 omega");
  let inserted = source
    .apply_edits(&[TextEdit::insert(TextSize(6), "bright ")])
    .unwrap();
  assert_eq!(inserted.to_contiguous_string(), "alpha bright 💡 omega");
  assert!(inserted.piece_count() > 1);

  let deleted = inserted
    .apply_edits(&[TextEdit::delete(range(6, 13))])
    .unwrap();
  assert_eq!(deleted.to_contiguous_string(), "alpha 💡 omega");

  let replaced = deleted
    .apply_edits(&[TextEdit::replace(range(6, 10), "🦀")])
    .unwrap();
  assert_eq!(replaced.to_contiguous_string(), "alpha 🦀 omega");

  let appended = replaced.append("\r\n終").unwrap();
  assert_eq!(appended.to_contiguous_string(), "alpha 🦀 omega\r\n終");
  assert_eq!(appended.revision(), Revision(4));
}

#[test]
fn multiple_edits_share_one_revision_and_cross_piece_boundaries() {
  let first = snapshot("abcdef")
    .apply_edits(&[
      TextEdit::insert(TextSize(2), "12"),
      TextEdit::insert(TextSize(4), "34"),
    ])
    .unwrap();
  assert_eq!(first.to_contiguous_string(), "ab12cd34ef");

  let second = first
    .apply_edits(&[
      TextEdit::replace(range(1, 5), "B"),
      TextEdit::replace(range(7, 9), "E"),
    ])
    .unwrap();
  assert_eq!(second.to_contiguous_string(), "aBd3Ef");
  assert_eq!(second.revision(), Revision(2));
}

#[test]
fn rejects_invalid_ranges_order_overlap_and_utf8_boundaries() {
  let source = snapshot("a💡e\u{301}");
  assert!(matches!(
    source.apply_edits(&[TextEdit::delete(range(2, 3))]),
    Err(SourceError::InvalidUtf8Boundary(TextSize(2)))
  ));
  assert!(matches!(
    source.apply_edits(&[TextEdit::delete(range(8, 7))]),
    Err(SourceError::InvalidRange(invalid)) if invalid == range(8, 7)
  ));
  let ascii = snapshot("abcdefgh");
  assert!(matches!(
    ascii.apply_edits(&[
      TextEdit::delete(range(5, 7)),
      TextEdit::delete(range(0, 1)),
    ]),
    Err(SourceError::UnsortedEdits)
  ));
  assert!(matches!(
    ascii.apply_edits(&[
      TextEdit::delete(range(0, 5)),
      TextEdit::delete(range(4, 7)),
    ]),
    Err(SourceError::OverlappingEdits)
  ));
}

#[test]
fn deterministic_random_edit_sequences_match_string_model() {
  let mut state = 0x6d65_6368_5f31_6101_u64;
  for case in 0..128 {
    let initial = match case % 4 {
      0 => "hello\r\nworld".to_string(),
      1 => "emoji 💡 and 🦀".to_string(),
      2 => "e\u{301}\rline\n終".to_string(),
      _ => String::new(),
    };
    let mut expected = initial.clone();
    let mut actual = snapshot(&initial);
    for _ in 0..32 {
      state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
      let boundaries = expected
        .char_indices()
        .map(|(offset, _)| offset)
        .chain(core::iter::once(expected.len()))
        .collect::<Vec<_>>();
      let a = boundaries[(state as usize) % boundaries.len()];
      state = state.rotate_left(17).wrapping_add(0x9e37_79b9);
      let b = boundaries[(state as usize) % boundaries.len()];
      let (start, end) = if a <= b { (a, b) } else { (b, a) };
      let insert = match (state >> 8) % 7 {
        0 => "💡",
        1 => "\r\n",
        2 => "e\u{301}",
        3 => "x",
        4 => "\n",
        _ => "",
      };
      expected.replace_range(start..end, insert);
      actual = actual
        .apply_edits(&[TextEdit::replace(
          range(start as u32, end as u32),
          insert,
        )])
        .unwrap();
      assert_eq!(actual.to_contiguous_string(), expected);
    }
  }
}
