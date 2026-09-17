use core::str;

use crate::document::{TextRange, TextSize, TextSnapshot};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CursorCheckpoint {
  pub offset: TextSize,
}

#[derive(Clone, Debug)]
pub struct Cursor<'a> {
  source: &'a TextSnapshot,
  offset: TextSize,
  end: TextSize,
}

impl<'a> Cursor<'a> {
  pub fn new(source: &'a TextSnapshot) -> Self {
    Self {
      source,
      offset: TextSize::ZERO,
      end: source.byte_len(),
    }
  }

  pub fn for_range(source: &'a TextSnapshot, range: TextRange) -> Self {
    Self {
      source,
      offset: range.start,
      end: range.end,
    }
  }

  pub fn offset(&self) -> TextSize {
    self.offset
  }

  pub fn end(&self) -> TextSize {
    self.end
  }

  pub fn is_eof(&self) -> bool {
    self.offset.0 >= self.end.0
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
    (self.offset.0 < self.end.0)
      .then(|| self.source.byte_at(self.offset))
      .flatten()
  }

  pub fn byte_at(&self, relative: u32) -> Option<u8> {
    let offset = TextSize(self.offset.0.checked_add(relative)?);
    (offset.0 < self.end.0)
      .then(|| self.source.byte_at(offset))
      .flatten()
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

  pub fn peek_char(&self) -> Option<char> {
    let first = self.byte()?;
    let width = utf8_width(first);
    if width == 0 || self.offset.0.saturating_add(width as u32) > self.end.0 {
      return None;
    }
    let mut bytes = [0_u8; 4];
    for (index, slot) in bytes.iter_mut().take(width).enumerate() {
      *slot = self.byte_at(index as u32)?;
    }
    str::from_utf8(&bytes[..width]).ok()?.chars().next()
  }

  pub fn bump_char(&mut self) -> Option<(char, TextRange)> {
    let start = self.offset;
    let character = self.peek_char()?;
    self.offset = TextSize(
      self
        .offset
        .0
        .saturating_add(character.len_utf8() as u32)
        .min(self.end.0),
    );
    Some((character, TextRange::new(start, self.offset)))
  }

  pub fn bump_bytes(&mut self, count: u32) -> Option<TextRange> {
    let end = self.offset.0.checked_add(count)?;
    if end > self.end.0 {
      return None;
    }
    let range = TextRange::new(self.offset, TextSize(end));
    self.offset = TextSize(end);
    Some(range)
  }

  pub fn remaining(&self) -> TextSize {
    self.end - self.offset
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
