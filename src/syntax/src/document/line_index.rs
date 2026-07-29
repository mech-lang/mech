use alloc::sync::Arc;

use super::edit::{TextEdit, TextSize};
use super::source::{Piece, TextSnapshot};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LineIndex {
  starts: Arc<[TextSize]>,
}

impl Default for LineIndex {
  fn default() -> Self {
    Self {
      starts: Arc::from([TextSize::ZERO]),
    }
  }
}

impl LineIndex {
  pub fn from_str(source: &str) -> Self {
    let mut starts = alloc::vec![TextSize::ZERO];
    scan_line_starts(
      TextSize::ZERO,
      TextSize(source.len() as u32),
      |offset| source.as_bytes().get(offset.to_usize()).copied(),
      &mut starts,
    );
    Self {
      starts: starts.into(),
    }
  }

  pub(crate) fn from_pieces(pieces: &[Piece], byte_len: TextSize) -> Self {
    let mut starts = alloc::vec![TextSize::ZERO];
    scan_line_starts(
      TextSize::ZERO,
      byte_len,
      |offset| byte_from_pieces(pieces, offset),
      &mut starts,
    );
    Self {
      starts: starts.into(),
    }
  }

  pub(crate) fn updated(
    &self,
    old: &TextSnapshot,
    new_pieces: &[Piece],
    new_len: TextSize,
    edits: &[TextEdit],
  ) -> Self {
    if edits.is_empty() {
      return self.clone();
    }

    let first = edits[0].delete.start;
    let last = edits[edits.len() - 1].delete.end;
    let first_line = self.line_of(first).saturating_sub(1);
    let last_line = self.line_of(last);
    let scan_start = self.starts[first_line];
    let old_scan_end = self
      .starts
      .get(last_line.saturating_add(2))
      .copied()
      .unwrap_or_else(|| old.byte_len());
    let new_scan_end = map_old_offset(old_scan_end, edits);

    let mut starts = self
      .starts
      .iter()
      .copied()
      .take_while(|start| start.0 < scan_start.0)
      .collect::<alloc::vec::Vec<_>>();
    starts.push(scan_start);
    scan_line_starts(
      scan_start,
      new_scan_end,
      |offset| byte_from_pieces(new_pieces, offset),
      &mut starts,
    );

    for old_start in self
      .starts
      .iter()
      .copied()
      .filter(|start| start.0 > old_scan_end.0)
    {
      let mapped = map_old_offset(old_start, edits);
      if starts.last().copied() != Some(mapped) {
        starts.push(mapped);
      }
    }

    if starts.is_empty() {
      starts.push(TextSize::ZERO);
    }
    debug_assert!(starts.windows(2).all(|pair| pair[0].0 < pair[1].0));
    debug_assert!(starts.iter().all(|start| start.0 <= new_len.0));
    Self {
      starts: starts.into(),
    }
  }

  pub fn line_count(&self) -> usize {
    self.starts.len()
  }

  pub fn line_starts(&self) -> &[TextSize] {
    &self.starts
  }

  pub fn line_start(&self, line: usize) -> Option<TextSize> {
    self.starts.get(line).copied()
  }

  pub fn line_of(&self, offset: TextSize) -> usize {
    match self
      .starts
      .binary_search_by_key(&offset.0, |start| start.0)
    {
      Ok(line) => line,
      Err(next) => next.saturating_sub(1),
    }
  }

  pub fn line_and_byte_column(&self, offset: TextSize) -> (usize, TextSize) {
    let line = self.line_of(offset);
    let start = self.starts[line];
    (line, offset - start)
  }
}

fn map_old_offset(offset: TextSize, edits: &[TextEdit]) -> TextSize {
  let mut delta = 0_i64;
  for edit in edits {
    if offset.0 < edit.delete.start.0 {
      break;
    }
    if offset.0 <= edit.delete.end.0 {
      let mapped = i64::from(edit.delete.start.0)
        + delta
        + i64::try_from(edit.insert.len()).unwrap_or(i64::MAX);
      return TextSize(mapped.clamp(0, i64::from(u32::MAX)) as u32);
    }
    delta += i64::try_from(edit.insert.len()).unwrap_or(i64::MAX)
      - i64::from(edit.delete.len().0);
  }
  let mapped = i64::from(offset.0) + delta;
  TextSize(mapped.clamp(0, i64::from(u32::MAX)) as u32)
}

fn byte_from_pieces(pieces: &[Piece], offset: TextSize) -> Option<u8> {
  let mut absolute = 0_u32;
  for piece in pieces {
    let len = piece.len().0;
    if offset.0 < absolute.saturating_add(len) {
      let local = piece.range_in_chunk.start.0 + (offset.0 - absolute);
      return piece.chunk.as_bytes().get(local as usize).copied();
    }
    absolute = absolute.saturating_add(len);
  }
  None
}

fn scan_line_starts(
  start: TextSize,
  end: TextSize,
  mut byte_at: impl FnMut(TextSize) -> Option<u8>,
  starts: &mut alloc::vec::Vec<TextSize>,
) {
  let mut offset = start.0;
  while offset < end.0 {
    match byte_at(TextSize(offset)) {
      Some(b'\r') => {
        if offset.saturating_add(1) < end.0
          && byte_at(TextSize(offset + 1)) == Some(b'\n')
        {
          offset = offset.saturating_add(2);
        } else {
          offset = offset.saturating_add(1);
        }
        let next = TextSize(offset);
        if starts.last().copied() != Some(next) {
          starts.push(next);
        }
      }
      Some(b'\n') => {
        offset = offset.saturating_add(1);
        let next = TextSize(offset);
        if starts.last().copied() != Some(next) {
          starts.push(next);
        }
      }
      Some(_) => offset = offset.saturating_add(1),
      None => break,
    }
  }
}
