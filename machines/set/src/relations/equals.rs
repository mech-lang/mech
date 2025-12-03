use crate::*;

use indexmap::set::IndexSet;
use mech_core::set::MechSet;

// Equals ------------------------------------------------------------------------
//
// Returns true if lhs and rhs contain exactly the same elements.
//

#[derive(Debug)]
struct SetEqualsFxn {
  lhs: Ref<MechSet>,
  rhs: Ref<MechSet>,
  out: Ref<bool>,
}

impl MechFunctionFactory for SetEqualsFxn {
  fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
    match args {
      FunctionArgs::Binary(out, arg1, arg2) => {
        let lhs: Ref<MechSet> = unsafe { arg1.as_unchecked() }.clone();
        let rhs: Ref<MechSet> = unsafe { arg2.as_unchecked() }.clone();
        let out: Ref<bool> = unsafe { out.as_unchecked() }.clone();
        Ok(Box::new(SetEqualsFxn { lhs, rhs, out }))
      },
      _ => Err(MechError2::new(IncorrectNumberOfArguments { expected: 2, found: args.len() }, None).with_compiler_loc()),
    }
  }
}

impl MechFunctionImpl for SetEqualsFxn {
  fn solve(&self) {
    unsafe {
      let out_ptr: &mut bool = &mut *(self.out.as_mut_ptr());
      let lhs_ptr: &MechSet = &*(self.lhs.as_ptr());
      let rhs_ptr: &MechSet = &*(self.rhs.as_ptr());

      // Uses the implementation of PartialEq for IndexSet (== operator)
      *out_ptr = lhs_ptr.set == rhs_ptr.set;
    }
  }
  fn out(&self) -> Value { Value::Bool(self.out.clone()) }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}

#[cfg(feature = "compiler")]
impl MechFunctionCompiler for SetEqualsFxn {
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    let name = "SetEqualsFxn".to_string();
    // Custom feature route: set/equals
    compile_binop!(name, self.out, self.lhs, self.rhs, ctx, FeatureFlag::Custom(hash_str("set/equals")));
  }
}

register_descriptor! {
  FunctionDescriptor {
    name: "SetEqualsFxn",
    ptr: SetEqualsFxn::new,
  }
}

fn set_equals_fxn(lhs: Value, rhs: Value) -> MResult<Box<dyn MechFunction>> {
  match (lhs, rhs) {
    (Value::Set(lhs), Value::Set(rhs)) => {
      Ok(Box::new(SetEqualsFxn { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(false) }))
    },
    x => Err(MechError2::new(
      UnhandledFunctionArgumentKind2 {
        arg: (x.0.kind(), x.1.kind()),
        fxn_name: "set/equals".to_string(),
      }, None
    ).with_compiler_loc()),
  }
}

pub struct SetEquals {}
impl NativeFunctionCompiler for SetEquals {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 2 {
      return Err(MechError2::new(IncorrectNumberOfArguments { expected: 2, found: arguments.len() }, None).with_compiler_loc());
    }
    let lhs = arguments[0].clone();
    let rhs = arguments[1].clone();
    match set_equals_fxn(lhs.clone(), rhs.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (lhs, rhs) {
          (Value::MutableReference(lhs), Value::MutableReference(rhs)) => set_equals_fxn(lhs.borrow().clone(), rhs.borrow().clone()),
          (lhs, Value::MutableReference(rhs)) => set_equals_fxn(lhs.clone(), rhs.borrow().clone()),
          (Value::MutableReference(lhs), rhs) => set_equals_fxn(lhs.borrow().clone(), rhs.clone()),
          x => Err(MechError2::new(
            UnhandledFunctionArgumentKind2 { arg: (x.0.kind(), x.1.kind()), fxn_name: "set/equals".to_string() },
            None
          ).with_compiler_loc()),
        }
      }
    }
  }
}

register_descriptor! {
  FunctionCompilerDescriptor {
    name: "set/equals",
    ptr: &SetEquals{},
  }
}