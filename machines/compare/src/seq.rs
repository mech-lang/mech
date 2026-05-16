use crate::*;
use mech_core::*;

#[derive(Debug)]
pub struct StrictEqValue {
  pub lhs: Value,
  pub rhs: Value,
  pub out: Ref<bool>,
}

impl MechFunctionImpl for StrictEqValue {
  fn solve(&self) {
    let lhs = match &self.lhs {
      Value::MutableReference(v) => v.borrow().clone(),
      v => v.clone(),
    };
    let rhs = match &self.rhs {
      Value::MutableReference(v) => v.borrow().clone(),
      v => v.clone(),
    };
    *self.out.borrow_mut() = lhs == rhs;
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}

#[cfg(feature = "compiler")]
impl MechFunctionCompiler for StrictEqValue {
  fn compile(&self, _ctx: &mut CompileCtx) -> MResult<Register> {
    todo!()
  }
}

fn impl_seq_fxn(lhs_value: Value, rhs_value: Value) -> MResult<Box<dyn MechFunction>> {
  Ok(Box::new(StrictEqValue { lhs: lhs_value, rhs: rhs_value, out: Ref::new(false) }))
}

impl_mech_binop_fxn!(CompareStrictEqual,impl_seq_fxn,"compare/seq");
