use alloc::string::String;
use core::str;

use unicode_segmentation::{
  GraphemeCursor, GraphemeIncomplete, UnicodeSegmentation,
};

use crate::document::source::SourceChunk;
use crate::document::{TextRange, TextSize, TextSnapshot};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CursorCheckpoint {
  pub offset: TextSize,
}

#[derive(Clone, Debug)]
pub struct Cursor<'a> {
  source: &'a TextSnapshot,
  offset: TextSize,
  consume_end: TextSize,
  context_end: TextSize,
}

#[derive(Clone, Copy, Debug)]
pub struct ContextView<'a> {
  source: &'a TextSnapshot,
  offset: TextSize,
  end: TextSize,
}

impl<'a> Cursor<'a> {
  pub fn new(source: &'a TextSnapshot) -> Self {
    Self {
      source,
      offset: TextSize::ZERO,
      consume_end: source.byte_len(),
      context_end: source.byte_len(),
    }
  }

  pub fn for_range(source: &'a TextSnapshot, range: TextRange) -> Self {
    Self::for_range_with_context(source, range, source.byte_len())
  }

  pub fn for_range_with_context(
    source: &'a TextSnapshot,
    range: TextRange,
    context_end: TextSize,
  ) -> Self {
    Self {
      source,
      offset: range.start,
      consume_end: range.end,
      context_end,
    }
  }

  pub fn offset(&self) -> TextSize {
    self.offset
  }

  pub fn end(&self) -> TextSize {
    self.consume_end
  }

  pub fn context_end(&self) -> TextSize {
    self.context_end
  }

  pub fn is_eof(&self) -> bool {
    self.offset.0 >= self.consume_end.0
  }

  pub fn checkpoint(&self) -> CursorCheckpoint {
    CursorCheckpoint {
      offset: self.offset,
    }
  }

  pub fn rewind(&mut self, checkpoint: CursorCheckpoint) {
    self.offset = checkpoint.offset;
  }

  pub fn byte(&self) -> Option<u8> {
    (self.offset.0 < self.consume_end.0)
      .then(|| self.source.byte_at(self.offset))
      .flatten()
  }

  pub fn byte_at(&self, relative: u32) -> Option<u8> {
    let offset = TextSize(self.offset.0.checked_add(relative)?);
    (offset.0 < self.consume_end.0)
      .then(|| self.source.byte_at(offset))
      .flatten()
  }

  pub fn context_byte_at(&self, relative: u32) -> Option<u8> {
    self.context_view().byte_at(relative)
  }

  pub fn starts_with(&self, expected: &str) -> bool {
    if expected.len() > self.remaining().to_usize() {
      return false;
    }
    expected
      .as_bytes()
      .iter()
      .enumerate()
      .all(|(index, byte)| self.byte_at(index as u32) == Some(*byte))
  }

  /// Match a literal as a sequence of complete extended graphemes.
  pub(crate) fn grapheme_literal_end(
    &self,
    literal: &str,
  ) -> Option<TextSize> {
    if literal.is_empty() {
      return None;
    }

    let mut scan = self.clone();
    for expected in UnicodeSegmentation::graphemes(literal, true) {
      let range = scan.peek_grapheme_range()?;
      if range.len().to_usize() != expected.len()
        || !scan.starts_with(expected)
      {
        return None;
      }
      scan.bump_bytes(range.len().0)?;
    }
    (scan.offset > self.offset).then_some(scan.offset)
  }

  /// Match a literal against complete graphemes after omitting selected
  /// scalar values, while retaining physical piece-backed offsets.
  pub(crate) fn filtered_grapheme_literal_end(
    &self,
    literal: &str,
    ignored: fn(char) -> bool,
  ) -> Option<TextSize> {
    if literal.is_empty() {
      return None;
    }

    let mut scan = self.clone();
    for expected in UnicodeSegmentation::graphemes(literal, true) {
      let (_, range) = scan.peek_filtered_grapheme_range(ignored)?;
      if !scan.filtered_range_matches(range, expected, ignored) {
        return None;
      }
      scan.bump_bytes(range.len().0)?;
    }
    (scan.offset > self.offset).then_some(scan.offset)
  }

  pub fn context_starts_with(&self, expected: &str) -> bool {
    self.context_view().starts_with(expected)
  }

  pub fn peek_char(&self) -> Option<char> {
    peek_char(self.source, self.offset, self.consume_end)
  }

  pub fn context_peek_char(&self) -> Option<char> {
    self.context_view().peek_char()
  }

  pub fn peek_grapheme_range(&self) -> Option<TextRange> {
    let range = self.context_peek_grapheme_range()?;
    (range.end.0 <= self.consume_end.0).then_some(range)
  }

  pub fn context_peek_grapheme_range(&self) -> Option<TextRange> {
    next_grapheme_range(self.source, self.offset, self.context_end)
  }

  /// Return one grapheme after omitting caller-selected scalar values.
  ///
  /// This is the isolated adapter needed by grammars whose legacy source
  /// initializer filtered bytes before Unicode segmentation. Only the current
  /// logical cluster and one lookahead scalar are buffered; physical offsets
  /// remain owned by the original piece-backed snapshot.
  pub(crate) fn peek_filtered_grapheme_range(
    &self,
    ignored: fn(char) -> bool,
  ) -> Option<(char, TextRange)> {
    if self.is_eof() {
      return None;
    }

    let mut scan = Self::for_range_with_context(
      self.source,
      TextRange::new(self.offset, self.context_end),
      self.context_end,
    );
    let mut logical = String::new();
    let mut first = None;
    let mut last_member_end = None;

    while let Some((character, physical)) = scan.bump_char() {
      if ignored(character) {
        continue;
      }
      if first.is_none() {
        if physical.end > self.consume_end {
          return None;
        }
        first = Some(character);
        last_member_end = Some(physical.end);
        logical.push(character);
        continue;
      }

      let boundary = logical.len();
      logical.push(character);
      let mut cursor = GraphemeCursor::new(boundary, logical.len(), true);
      match cursor.is_boundary(&logical, 0) {
        Ok(true) => {
          let end = TextSize(physical.start.0.min(self.consume_end.0));
          return Some((first?, TextRange::new(self.offset, end)));
        }
        Ok(false) => {
          if physical.end > self.consume_end {
            return None;
          }
          last_member_end = Some(physical.end);
        }
        Err(GraphemeIncomplete::PreContext(_)) => {
          // `logical` always contains the complete left context.
          return None;
        }
        Err(
          GraphemeIncomplete::NextChunk
          | GraphemeIncomplete::PrevChunk
          | GraphemeIncomplete::InvalidOffset,
        ) => return None,
      }
    }

    let last_member_end = last_member_end?;
    (last_member_end <= self.consume_end)
      .then_some((first?, TextRange::new(self.offset, self.consume_end)))
  }

  pub fn context_view(&self) -> ContextView<'a> {
    ContextView {
      source: self.source,
      offset: self.offset,
      end: self.context_end,
    }
  }

  pub fn bump_char(&mut self) -> Option<(char, TextRange)> {
    let start = self.offset;
    let character = self.peek_char()?;
    self.offset = TextSize(
      self
        .offset
        .0
        .saturating_add(character.len_utf8() as u32)
        .min(self.consume_end.0),
    );
    Some((character, TextRange::new(start, self.offset)))
  }

  pub fn bump_grapheme(&mut self) -> Option<TextRange> {
    let range = self.peek_grapheme_range()?;
    self.offset = range.end;
    Some(range)
  }

  pub fn bump_bytes(&mut self, count: u32) -> Option<TextRange> {
    let end = self.offset.0.checked_add(count)?;
    if end > self.consume_end.0 {
      return None;
    }
    let range = TextRange::new(self.offset, TextSize(end));
    self.offset = TextSize(end);
    Some(range)
  }

  pub fn remaining(&self) -> TextSize {
    self.consume_end - self.offset
  }

  pub fn is_line_start(&self) -> bool {
    if self.offset.0 == 0 {
      return true;
    }
    match self.source.byte_at(TextSize(self.offset.0 - 1)) {
      Some(b'\n') => true,
      Some(b'\r') => self.byte() != Some(b'\n'),
      _ => false,
    }
  }

  fn filtered_range_matches(
    &self,
    range: TextRange,
    expected: &str,
    ignored: fn(char) -> bool,
  ) -> bool {
    let mut scan =
      Self::for_range_with_context(self.source, range, range.end);
    let mut expected = expected.chars();

    while let Some((character, _)) = scan.bump_char() {
      if ignored(character) {
        continue;
      }
      if expected.next() != Some(character) {
        return false;
      }
    }
    expected.next().is_none()
  }
}

impl<'a> ContextView<'a> {
  pub fn offset(self) -> TextSize {
    self.offset
  }

  pub fn end(self) -> TextSize {
    self.end
  }

  pub fn byte_at(self, relative: u32) -> Option<u8> {
    let offset = TextSize(self.offset.0.checked_add(relative)?);
    (offset.0 < self.end.0)
      .then(|| self.source.byte_at(offset))
      .flatten()
  }

  pub fn starts_with(self, expected: &str) -> bool {
    if expected.len() > self.remaining().to_usize() {
      return false;
    }
    expected
      .as_bytes()
      .iter()
      .enumerate()
      .all(|(index, byte)| self.byte_at(index as u32) == Some(*byte))
  }

  pub fn peek_char(self) -> Option<char> {
    peek_char(self.source, self.offset, self.end)
  }

  pub fn at_relative(self, relative: u32) -> Option<Self> {
    let offset = TextSize(self.offset.0.checked_add(relative)?);
    (offset <= self.end).then_some(Self {
      source: self.source,
      offset,
      end: self.end,
    })
  }

  pub fn remaining(self) -> TextSize {
    self.end - self.offset
  }

  pub fn is_line_start(self) -> bool {
    if self.offset.0 == 0 {
      return true;
    }
    match self.source.byte_at(TextSize(self.offset.0 - 1)) {
      Some(b'\n') => true,
      Some(b'\r') => self.byte_at(0) != Some(b'\n'),
      _ => false,
    }
  }
}

fn peek_char(
  source: &TextSnapshot,
  offset: TextSize,
  end: TextSize,
) -> Option<char> {
  let first = (offset < end).then(|| source.byte_at(offset)).flatten()?;
  let width = utf8_width(first);
  if width == 0 || offset.0.saturating_add(width as u32) > end.0 {
    return None;
  }
  let mut bytes = [0_u8; 4];
  for (index, slot) in bytes.iter_mut().take(width).enumerate() {
    *slot = source.byte_at(TextSize(offset.0 + index as u32))?;
  }
  str::from_utf8(&bytes[..width]).ok()?.chars().next()
}

fn next_grapheme_range(
  source: &TextSnapshot,
  offset: TextSize,
  end: TextSize,
) -> Option<TextRange> {
  if offset.0 >= end.0
    || end.0 > source.byte_len().0
    || !source.is_char_boundary(offset)
    || !source.is_char_boundary(end)
  {
    return None;
  }

  let mut cursor = GraphemeCursor::new(
    offset.to_usize(),
    end.to_usize(),
    true,
  );
  let mut chunk = forward_chunk(source, offset, end)?;
  loop {
    match cursor.next_boundary(chunk.text, chunk.range.start.to_usize()) {
      Ok(Some(boundary)) => {
        let boundary = TextSize::checked_from_usize(boundary).ok()?;
        return Some(TextRange::new(offset, boundary));
      }
      Ok(None) => return None,
      Err(GraphemeIncomplete::NextChunk) => {
        let at = TextSize::checked_from_usize(cursor.cur_cursor()).ok()?;
        chunk = forward_chunk(source, at, end)?;
      }
      Err(GraphemeIncomplete::PreContext(before)) => {
        let before = TextSize::checked_from_usize(before).ok()?;
        let context = source.chunk_before(before)?;
        cursor.provide_context(
          context.text,
          context.range.start.to_usize(),
        );
      }
      Err(
        GraphemeIncomplete::PrevChunk
        | GraphemeIncomplete::InvalidOffset,
      ) => return None,
    }
  }
}

fn forward_chunk(
  source: &TextSnapshot,
  offset: TextSize,
  end: TextSize,
) -> Option<SourceChunk<'_>> {
  let chunk = source.chunk_at(offset)?;
  let chunk_end = TextSize(chunk.range.end.0.min(end.0));
  let len = (chunk_end - chunk.range.start).to_usize();
  Some(SourceChunk {
    text: &chunk.text[..len],
    range: TextRange::new(chunk.range.start, chunk_end),
  })
}

const fn utf8_width(first: u8) -> usize {
  match first {
    0x00..=0x7f => 1,
    0xc2..=0xdf => 2,
    0xe0..=0xef => 3,
    0xf0..=0xf4 => 4,
    _ => 0,
  }
}
