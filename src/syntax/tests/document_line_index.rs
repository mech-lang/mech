use mech_syntax::document::{
  DocumentId, LineIndex, Revision, TextEdit, TextRange, TextSize, TextSnapshot,
};

fn snapshot(text: &str) -> TextSnapshot {
  TextSnapshot::new(DocumentId(1), Revision(0), text).unwrap()
}

fn expected_starts(text: &str) -> Vec<TextSize> {
  let mut starts = vec![TextSize::ZERO];
  let bytes = text.as_bytes();
  let mut offset = 0;
  while offset < bytes.len() {
    match bytes[offset] {
      b'\r' if bytes.get(offset + 1) == Some(&b'\n') => {
        offset += 2;
        starts.push(TextSize(offset as u32));
      }
      b'\r' | b'\n' => {
        offset += 1;
        starts.push(TextSize(offset as u32));
      }
      _ => offset += 1,
    }
  }
  starts
}

fn assert_index(text: &str, index: &LineIndex) {
  assert_eq!(index.line_starts(), expected_starts(text));
  for boundary in text
    .char_indices()
    .map(|(offset, _)| offset)
    .chain(core::iter::once(text.len()))
  {
    let (line, column) = index.line_and_byte_column(TextSize(boundary as u32));
    assert_eq!(
      index.line_start(line).unwrap().0 + column.0,
      boundary as u32
    );
  }
}

#[test]
fn indexes_lf_crlf_and_cr_without_normalizing() {
  let text = "a\nb\r\nc\rd\n";
  let source = snapshot(text);
  assert_eq!(
    source.line_index().line_starts(),
    &[TextSize(0), TextSize(2), TextSize(5), TextSize(7), TextSize(9)]
  );
  assert_eq!(source.to_contiguous_string(), text);
}

#[test]
fn edits_update_line_boundaries_around_piece_edges() {
  let mut source = snapshot("a\rb\nc\r\nd");
  let cases = [
    (TextRange::empty(TextSize(2)), "\n"),
    (TextRange::new(TextSize(1), TextSize(3)), "\r\n"),
    (TextRange::new(TextSize(5), TextSize(6)), ""),
    (TextRange::empty(TextSize(0)), "💡\n"),
  ];
  for (delete, insert) in cases {
    source = source
      .apply_edits(&[TextEdit::replace(delete, insert)])
      .unwrap();
    let text = source.to_contiguous_string();
    assert_index(&text, source.line_index());
  }
}

#[test]
fn line_index_handles_unicode_as_byte_columns() {
  let source = snapshot("💡e\u{301}\r\n終");
  assert_eq!(source.line_index().line_starts(), &[TextSize(0), TextSize(9)]);
  assert_eq!(
    source.line_index().line_and_byte_column(TextSize(7)),
    (0, TextSize(7))
  );
}
