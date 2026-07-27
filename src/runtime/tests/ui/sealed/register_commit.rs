use mech_core::{
  ReactiveCellId, ReactiveRegisterCommit,
};

struct ExternalRegisterCommit;

impl ReactiveRegisterCommit for ExternalRegisterCommit {
  fn output_cells(&self) -> &[ReactiveCellId] {
    &[]
  }

  fn commit(self: Box<Self>) {}
}

fn main() {}
