use crate::*;
use mech_core::*;

#[derive(Debug)]
pub struct StrictNotEqValue {
  pub lhs: Value,
  pub rhs: Value,
  pub out: Ref<bool>,
}

impl MechFunctionImpl for StrictNotEqValue {
  fn solve(&self) {
    let lhs = match &self.lhs {
      Value::MutableReference(v) => v.borrow().clone(),
      v => v.clone(),
    };
    let rhs = match &self.rhs {
      Value::MutableReference(v) => v.borrow().clone(),
      v => v.clone(),
    };
    *self.out.borrow_mut() = lhs != rhs;
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}

#[cfg(feature = "compiler")]
impl MechFunctionCompiler for StrictNotEqValue {
  fn compile(&self, _ctx: &mut CompileCtx) -> MResult<Register> {
    Err(MechError::new(GenericError{message: "StrictNotEqValue compiler path is not supported".to_string()}, None).with_compiler_loc())
  }
}

fn impl_sneq_fxn(lhs_value: Value, rhs_value: Value) -> MResult<Box<dyn MechFunction>> {
  Ok(Box::new(StrictNotEqValue { lhs: lhs_value, rhs: rhs_value, out: Ref::new(false) }))
}

impl_mech_binop_fxn!(CompareStrictNotEqual,impl_sneq_fxn,"compare/sneq");
