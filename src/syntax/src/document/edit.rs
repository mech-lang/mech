use alloc::string::String;
use core::fmt;
use core::ops::{Add, AddAssign, Sub, SubAssign};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct TextSize(pub u32);

impl TextSize {
  pub const ZERO: Self = Self(0);

  pub const fn from_u32(value: u32) -> Self {
    Self(value)
  }

  pub const fn to_u32(self) -> u32 {
    self.0
  }

  pub const fn to_usize(self) -> usize {
    self.0 as usize
  }

  pub fn checked_from_usize(value: usize) -> Result<Self, SourceError> {
    u32::try_from(value)
      .map(Self)
      .map_err(|_| SourceError::SourceTooLarge)
  }
}

impl Add for TextSize {
  type Output = Self;

  fn add(self, rhs: Self) -> Self::Output {
    Self(self.0.saturating_add(rhs.0))
  }
}

impl AddAssign for TextSize {
  fn add_assign(&mut self, rhs: Self) {
    *self = *self + rhs;
  }
}

impl Sub for TextSize {
  type Output = Self;

  fn sub(self, rhs: Self) -> Self::Output {
    Self(self.0.saturating_sub(rhs.0))
  }
}

impl SubAssign for TextSize {
  fn sub_assign(&mut self, rhs: Self) {
    *self = *self - rhs;
  }
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct TextRange {
  pub start: TextSize,
  pub end: TextSize,
}

impl TextRange {
  pub const fn new(start: TextSize, end: TextSize) -> Self {
    Self { start, end }
  }

  pub const fn empty(offset: TextSize) -> Self {
    Self {
      start: offset,
      end: offset,
    }
  }

  pub const fn at(start: TextSize, len: TextSize) -> Self {
    Self {
      start,
      end: TextSize(start.0.saturating_add(len.0)),
    }
  }

  pub const fn len(self) -> TextSize {
    TextSize(self.end.0.saturating_sub(self.start.0))
  }

  pub const fn is_empty(self) -> bool {
    self.start.0 == self.end.0
  }

  pub const fn contains(self, offset: TextSize) -> bool {
    self.start.0 <= offset.0 && offset.0 < self.end.0
  }

  pub const fn contains_inclusive(self, offset: TextSize) -> bool {
    self.start.0 <= offset.0 && offset.0 <= self.end.0
  }

  pub const fn contains_range(self, other: Self) -> bool {
    self.start.0 <= other.start.0 && other.end.0 <= self.end.0
  }

  pub const fn intersects(self, other: Self) -> bool {
    self.start.0 < other.end.0 && other.start.0 < self.end.0
  }
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct TextEdit {
  pub delete: TextRange,
  pub insert: String,
}

impl TextEdit {
  pub fn insert(offset: TextSize, text: impl Into<String>) -> Self {
    Self {
      delete: TextRange::empty(offset),
      insert: text.into(),
    }
  }

  pub fn delete(range: TextRange) -> Self {
    Self {
      delete: range,
      insert: String::new(),
    }
  }

  pub fn replace(range: TextRange, text: impl Into<String>) -> Self {
    Self {
      delete: range,
      insert: text.into(),
    }
  }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SourceError {
  InvalidRange(TextRange),
  InvalidUtf8Boundary(TextSize),
  UnsortedEdits,
  OverlappingEdits,
  SourceTooLarge,
  WrongDocumentRevision,
}

impl fmt::Display for SourceError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::InvalidRange(range) => write!(
        f,
        "invalid source range {}..{}",
        range.start.0, range.end.0
      ),
      Self::InvalidUtf8Boundary(offset) => {
        write!(f, "offset {} is not a UTF-8 boundary", offset.0)
      }
      Self::UnsortedEdits => f.write_str("edits must be sorted by source range"),
      Self::OverlappingEdits => f.write_str("edit ranges must not overlap"),
      Self::SourceTooLarge => f.write_str("source exceeds the 32-bit text range"),
      Self::WrongDocumentRevision => {
        f.write_str("source snapshot belongs to another document revision")
      }
    }
  }
}
