#[macro_use]
use crate::stdlib::*;

// Record Access --------------------------------------------------------------

#[derive(Debug)]
pub struct RecordAccess {
  pub source: Value,
}
impl MechFunction for RecordAccess {
  fn solve(&self) {
    ()
  }
  fn out(&self) -> Value { self.source.clone() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}