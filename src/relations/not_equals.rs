use crate::*;

use indexmap::set::IndexSet;
use mech_core::set::MechSet;

// Not Equals --------------------------------------------------------------------
//
// Returns true if lhs and rhs do NOT contain exactly the same elements.
//

#[derive(Debug)]
struct SetNotEqualsFxn {
  lhs: Ref<MechSet>,
  rhs: Ref<MechSet>,
  out: Ref<bool>,
}

impl MechFunctionFactory for SetNotEqualsFxn {
  fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
    match args {
      FunctionArgs::Binary(out, arg1, arg2) => {
        let lhs: Ref<MechSet> = unsafe { arg1.as_unchecked() }.clone();
        let rhs: Ref<MechSet> = unsafe { arg2.as_unchecked() }.clone();
        let out: Ref<bool> = unsafe { out.as_unchecked() }.clone();
        Ok(Box::new(SetNotEqualsFxn { lhs, rhs, out }))
      },
      _ => Err(MechError{
        file: file!().to_string(),
        tokens: vec![],
        msg: format!("{} requires 2 arguments, got {:?}", stringify!($struct_name), args),
        id: line!(),
        kind: MechErrorKind::IncorrectNumberOfArguments
      })
    }
  }
}

impl MechFunctionImpl for SetNotEqualsFxn {
  fn solve(&self) {
    unsafe {
      let out_ptr: &mut bool = &mut *(self.out.as_mut_ptr());
      let lhs_ptr: &MechSet = &*(self.lhs.as_ptr());
      let rhs_ptr: &MechSet = &*(self.rhs.as_ptr());

      // Uses the implementation of PartialEq for IndexSet (!= operator)
      *out_ptr = lhs_ptr.set != rhs_ptr.set;
    }
  }
  fn out(&self) -> Value { Value::Bool(self.out.clone()) }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}

#[cfg(feature = "compiler")]
impl MechFunctionCompiler for SetNotEqualsFxn {
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    let name = "SetNotEqualsFxn".to_string();
    // Custom feature route: set/not_equals
    compile_binop!(name, self.out, self.lhs, self.rhs, ctx, FeatureFlag::Custom(hash_str("set/not_equals")));
  }
}

register_descriptor! {
  FunctionDescriptor {
    name: "SetNotEqualsFxn",
    ptr: SetNotEqualsFxn::new,
  }
}

fn set_not_equals_fxn(lhs: Value, rhs: Value) -> MResult<Box<dyn MechFunction>> {
  match (lhs, rhs) {
    (Value::Set(lhs), Value::Set(rhs)) => {
      Ok(Box::new(SetNotEqualsFxn { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(false) }))
    },
    x => Err(MechError{
      file: file!().to_string(),
      tokens: vec![],
      msg: format!("set_not_equals_fxn cannot handle arguments: {:?}", x),
      id: line!(),
      kind: MechErrorKind::UnhandledFunctionArgumentKind
    }),
  }
}

pub struct SetNotEquals {}
impl NativeFunctionCompiler for SetNotEquals {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError{
        file: file!().to_string(),
        tokens: vec![],
        msg: "".to_string(),
        id: line!(),
        kind: MechErrorKind::IncorrectNumberOfArguments
      });
    }
    let lhs = arguments[0].clone();
    let rhs = arguments[1].clone();
    match set_not_equals_fxn(lhs.clone(), rhs.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (lhs, rhs) {
          (Value::MutableReference(lhs), Value::MutableReference(rhs)) => set_not_equals_fxn(lhs.borrow().clone(), rhs.borrow().clone()),
          (lhs, Value::MutableReference(rhs)) => set_not_equals_fxn(lhs.clone(), rhs.borrow().clone()),
          (Value::MutableReference(lhs), rhs) => set_not_equals_fxn(lhs.borrow().clone(), rhs.clone()),
          x => Err(MechError{
            file: file!().to_string(),
            tokens: vec![],
            msg: format!("{:?}", x),
            id: line!(),
            kind: MechErrorKind::UnhandledFunctionArgumentKind
          }),
        }
      }
    }
  }
}

register_descriptor! {
  FunctionCompilerDescriptor {
    name: "set/not_equals",
    ptr: &SetNotEquals{},
  }
}