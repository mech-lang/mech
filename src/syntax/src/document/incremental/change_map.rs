use alloc::vec::Vec;

use crate::document::{TextEdit, TextRange, TextSize};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Affinity {
  Before,
  After,
}

#[derive(Clone, Debug)]
pub struct ChangeMap {
  edits: Vec<TextEdit>,
  old_changed: TextRange,
  new_changed: TextRange,
}

impl ChangeMap {
  pub fn new(edits: &[TextEdit]) -> Self {
    if edits.is_empty() {
      return Self {
        edits: Vec::new(),
        old_changed: TextRange::empty(TextSize::ZERO),
        new_changed: TextRange::empty(TextSize::ZERO),
      };
    }
    let old_changed = TextRange::new(
      edits[0].delete.start,
      edits[edits.len() - 1].delete.end,
    );
    let mut map = Self {
      edits: edits.to_vec(),
      old_changed,
      new_changed: TextRange::empty(TextSize::ZERO),
    };
    map.new_changed = TextRange::new(
      map.map_offset(old_changed.start, Affinity::Before),
      map.map_offset(old_changed.end, Affinity::After),
    );
    map
  }

  pub fn edits(&self) -> &[TextEdit] {
    &self.edits
  }

  pub fn old_changed_range(&self) -> TextRange {
    self.old_changed
  }

  pub fn new_changed_range(&self) -> TextRange {
    self.new_changed
  }

  pub fn map_range(&self, range: TextRange) -> TextRange {
    TextRange::new(
      self.map_offset(range.start, Affinity::Before),
      self.map_offset(range.end, Affinity::After),
    )
  }

  pub fn map_offset(&self, offset: TextSize, affinity: Affinity) -> TextSize {
    let mut delta = 0_i64;
    for edit in &self.edits {
      if offset.0 < edit.delete.start.0
        || (offset == edit.delete.start && affinity == Affinity::Before)
      {
        break;
      }
      if offset.0 < edit.delete.end.0
        || (offset == edit.delete.end && affinity == Affinity::Before)
      {
        let insertion = if affinity == Affinity::After {
          edit.insert.len()
        } else {
          0
        };
        let mapped = i64::from(edit.delete.start.0)
          + delta
          + i64::try_from(insertion).unwrap_or(i64::MAX);
        return TextSize(mapped.clamp(0, i64::from(u32::MAX)) as u32);
      }
      delta += i64::try_from(edit.insert.len()).unwrap_or(i64::MAX)
        - i64::from(edit.delete.len().0);
    }
    let mapped = i64::from(offset.0) + delta;
    TextSize(mapped.clamp(0, i64::from(u32::MAX)) as u32)
  }

  pub fn touches_boundary(&self, range: TextRange) -> bool {
    self.edits.iter().any(|edit| {
      edit.delete.start == range.start
        || edit.delete.end == range.end
        || (edit.delete.is_empty()
          && (edit.delete.start == range.start || edit.delete.start == range.end))
    })
  }

  pub fn changes_parser_context(&self, old_text: impl Fn(TextRange) -> alloc::string::String) -> bool {
    self.edits.iter().any(|edit| {
      let deleted = old_text(edit.delete);
      edit
        .insert
        .chars()
        .chain(deleted.chars())
        .any(|character| {
          matches!(
            character,
            '\r' | '\n' | '`' | '~' | '(' | ')' | '[' | ']' | '{' | '}' | ':' | '='
          )
        })
    })
  }
}
