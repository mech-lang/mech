use crate::document::TextSize;

use super::cursor::CursorCheckpoint;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ParserCheckpoint {
  pub(crate) cursor: CursorCheckpoint,
  pub(crate) events: usize,
  pub(crate) diagnostics: usize,
  pub(crate) open_markers: usize,
  pub(crate) covered_end: TextSize,
  pub(crate) rule_depth: usize,
  pub(crate) nesting: u32,
}
