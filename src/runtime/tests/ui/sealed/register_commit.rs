use mech_core::{
  CanonicalCellId, ReactiveRegisterCommit,
};

struct ExternalRegisterCommit;

impl ReactiveRegisterCommit for ExternalRegisterCommit {
  fn output_cells(&self) -> &[CanonicalCellId] {
    &[]
  }

  fn commit(self: Box<Self>) {}
}

fn main() {}
