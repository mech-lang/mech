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
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    todo!();
  }
}

#[derive(Debug)]
pub struct RecordAccessSwizzle {
  pub source: Value,
}

impl MechFunction for RecordAccessSwizzle {
  fn solve(&self) {
    ()
  }
  fn out(&self) -> Value { self.source.clone() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    todo!();
  }
}
