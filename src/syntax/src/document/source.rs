use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

use super::edit::{SourceError, TextEdit, TextRange, TextSize};
use super::ids::{DocumentId, Revision};
use super::line_index::LineIndex;

#[derive(Clone, Debug)]
pub(crate) struct Piece {
  pub(crate) chunk: Arc<str>,
  pub(crate) range_in_chunk: TextRange,
}

impl Piece {
  pub(crate) fn len(&self) -> TextSize {
    self.range_in_chunk.len()
  }

  fn text(&self) -> &str {
    &self.chunk
      [self.range_in_chunk.start.to_usize()..self.range_in_chunk.end.to_usize()]
  }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct SourceChunk<'a> {
  pub text: &'a str,
  pub range: TextRange,
}

#[derive(Clone, Debug)]
pub struct TextSnapshot {
  document: DocumentId,
  revision: Revision,
  pub(crate) pieces: Arc<[Piece]>,
  byte_len: TextSize,
  line_index: LineIndex,
}

impl TextSnapshot {
  pub fn new(
    document: DocumentId,
    revision: Revision,
    source: impl Into<Arc<str>>,
  ) -> Result<Self, SourceError> {
    let source = source.into();
    let byte_len = TextSize::checked_from_usize(source.len())?;
    let line_index = LineIndex::from_str(&source);
    let pieces: Arc<[Piece]> = if source.is_empty() {
      Arc::from([])
    } else {
      Arc::from([Piece {
        chunk: source,
        range_in_chunk: TextRange::new(TextSize::ZERO, byte_len),
      }])
    };
    Ok(Self {
      document,
      revision,
      pieces,
      byte_len,
      line_index,
    })
  }

  pub fn document(&self) -> DocumentId {
    self.document
  }

  pub fn revision(&self) -> Revision {
    self.revision
  }

  pub fn byte_len(&self) -> TextSize {
    self.byte_len
  }

  pub fn is_empty(&self) -> bool {
    self.byte_len.0 == 0
  }

  pub fn full_range(&self) -> TextRange {
    TextRange::new(TextSize::ZERO, self.byte_len)
  }

  pub fn line_index(&self) -> &LineIndex {
    &self.line_index
  }

  pub fn piece_count(&self) -> usize {
    self.pieces.len()
  }

  pub fn chunks(&self) -> impl Iterator<Item = &str> {
    self.pieces.iter().map(Piece::text)
  }

  pub fn text(&self, range: TextRange) -> Result<String, SourceError> {
    self.validate_range(range)?;
    let mut text = String::with_capacity(range.len().to_usize());
    self.for_each_slice(range, |slice| text.push_str(slice));
    Ok(text)
  }

  pub fn to_contiguous_string(&self) -> String {
    let mut text = String::with_capacity(self.byte_len.to_usize());
    for piece in self.pieces.iter() {
      text.push_str(piece.text());
    }
    text
  }

  pub fn byte_at(&self, offset: TextSize) -> Option<u8> {
    if offset.0 >= self.byte_len.0 {
      return None;
    }
    let mut absolute = 0_u32;
    for piece in self.pieces.iter() {
      let end = absolute + piece.len().0;
      if offset.0 < end {
        let local = piece.range_in_chunk.start.0 + (offset.0 - absolute);
        return piece.chunk.as_bytes().get(local as usize).copied();
      }
      absolute = end;
    }
    None
  }

  pub(crate) fn chunk_at(&self, offset: TextSize) -> Option<SourceChunk<'_>> {
    if offset.0 >= self.byte_len.0 {
      return None;
    }
    let mut absolute = TextSize::ZERO;
    for piece in self.pieces.iter() {
      let end = absolute + piece.len();
      if offset.0 < end.0 {
        return Some(SourceChunk {
          text: piece.text(),
          range: TextRange::new(absolute, end),
        });
      }
      absolute = end;
    }
    None
  }

  pub(crate) fn chunk_before(
    &self,
    offset: TextSize,
  ) -> Option<SourceChunk<'_>> {
    if offset.0 == 0
      || offset.0 > self.byte_len.0
      || !self.is_char_boundary(offset)
    {
      return None;
    }
    let mut absolute = TextSize::ZERO;
    for piece in self.pieces.iter() {
      let end = absolute + piece.len();
      if offset.0 <= end.0 {
        let prefix_len = (offset - absolute).to_usize();
        if prefix_len == 0 {
          return None;
        }
        return Some(SourceChunk {
          text: &piece.text()[..prefix_len],
          range: TextRange::new(absolute, offset),
        });
      }
      absolute = end;
    }
    None
  }

  pub fn is_char_boundary(&self, offset: TextSize) -> bool {
    if offset.0 == 0 || offset.0 == self.byte_len.0 {
      return true;
    }
    self
      .byte_at(offset)
      .map(|byte| byte & 0b1100_0000 != 0b1000_0000)
      .unwrap_or(false)
  }

  pub fn validate_range(&self, range: TextRange) -> Result<(), SourceError> {
    if range.start.0 > range.end.0 || range.end.0 > self.byte_len.0 {
      return Err(SourceError::InvalidRange(range));
    }
    if !self.is_char_boundary(range.start) {
      return Err(SourceError::InvalidUtf8Boundary(range.start));
    }
    if !self.is_char_boundary(range.end) {
      return Err(SourceError::InvalidUtf8Boundary(range.end));
    }
    Ok(())
  }

  pub fn append(&self, text: impl Into<String>) -> Result<Self, SourceError> {
    self.apply_edits(&[TextEdit::insert(self.byte_len, text)])
  }

  pub fn apply_edits(&self, edits: &[TextEdit]) -> Result<Self, SourceError> {
    self.validate_edits(edits)?;
    if edits.is_empty() {
      return Ok(self.clone());
    }

    let mut pieces = Vec::with_capacity(self.pieces.len() + edits.len() * 2);
    let mut copied_until = TextSize::ZERO;
    let mut new_len = i64::from(self.byte_len.0);
    for edit in edits {
      self.copy_range(
        TextRange::new(copied_until, edit.delete.start),
        &mut pieces,
      );
      push_insert(&mut pieces, &edit.insert)?;
      copied_until = edit.delete.end;
      new_len += i64::try_from(edit.insert.len()).map_err(|_| SourceError::SourceTooLarge)?
        - i64::from(edit.delete.len().0);
    }
    self.copy_range(
      TextRange::new(copied_until, self.byte_len),
      &mut pieces,
    );
    let byte_len = TextSize::checked_from_usize(
      usize::try_from(new_len).map_err(|_| SourceError::SourceTooLarge)?,
    )?;
    let line_index = self
      .line_index
      .updated(self, &pieces, byte_len, edits);
    Ok(Self {
      document: self.document,
      revision: Revision(self.revision.0.saturating_add(1)),
      pieces: pieces.into(),
      byte_len,
      line_index,
    })
  }

  pub(crate) fn with_revision(mut self, revision: Revision) -> Self {
    self.revision = revision;
    self
  }

  pub(crate) fn for_each_slice(&self, range: TextRange, mut f: impl FnMut(&str)) {
    if range.is_empty() {
      return;
    }
    let mut absolute = 0_u32;
    for piece in self.pieces.iter() {
      let piece_start = absolute;
      let piece_end = absolute + piece.len().0;
      absolute = piece_end;
      let start = range.start.0.max(piece_start);
      let end = range.end.0.min(piece_end);
      if start >= end {
        continue;
      }
      let local_start = piece.range_in_chunk.start.0 + start - piece_start;
      let local_end = piece.range_in_chunk.start.0 + end - piece_start;
      f(&piece.chunk[local_start as usize..local_end as usize]);
    }
  }

  fn validate_edits(&self, edits: &[TextEdit]) -> Result<(), SourceError> {
    let mut previous: Option<TextRange> = None;
    for edit in edits {
      self.validate_range(edit.delete)?;
      TextSize::checked_from_usize(edit.insert.len())?;
      if let Some(prior) = previous {
        if edit.delete.start.0 < prior.start.0 {
          return Err(SourceError::UnsortedEdits);
        }
        if edit.delete.start.0 < prior.end.0 {
          return Err(SourceError::OverlappingEdits);
        }
      }
      previous = Some(edit.delete);
    }
    Ok(())
  }

  fn copy_range(&self, range: TextRange, output: &mut Vec<Piece>) {
    if range.is_empty() {
      return;
    }
    let mut absolute = 0_u32;
    for piece in self.pieces.iter() {
      let piece_start = absolute;
      let piece_end = absolute + piece.len().0;
      absolute = piece_end;
      let start = range.start.0.max(piece_start);
      let end = range.end.0.min(piece_end);
      if start >= end {
        continue;
      }
      let local_start = piece.range_in_chunk.start.0 + start - piece_start;
      let local_end = piece.range_in_chunk.start.0 + end - piece_start;
      push_piece(
        output,
        Piece {
          chunk: piece.chunk.clone(),
          range_in_chunk: TextRange::new(TextSize(local_start), TextSize(local_end)),
        },
      );
    }
  }
}

fn push_insert(output: &mut Vec<Piece>, insert: &str) -> Result<(), SourceError> {
  if insert.is_empty() {
    return Ok(());
  }
  let len = TextSize::checked_from_usize(insert.len())?;
  push_piece(
    output,
    Piece {
      chunk: Arc::<str>::from(insert),
      range_in_chunk: TextRange::new(TextSize::ZERO, len),
    },
  );
  Ok(())
}

fn push_piece(output: &mut Vec<Piece>, piece: Piece) {
  if piece.len().0 == 0 {
    return;
  }
  if let Some(previous) = output.last_mut()
    && Arc::ptr_eq(&previous.chunk, &piece.chunk)
    && previous.range_in_chunk.end == piece.range_in_chunk.start
  {
    previous.range_in_chunk.end = piece.range_in_chunk.end;
    return;
  }
  output.push(piece);
}
