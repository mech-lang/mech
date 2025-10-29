
use crate::*;

use indexmap::set::IndexSet;
use mech_core::set::MechSet;

// Disjoint ------------------------------------------------------------------------

#[derive(Debug)]
struct SetDisjointFxn {
  lhs: Ref<MechSet>,
  rhs: Ref<MechSet>,
  out: Ref<bool>,
}
impl MechFunctionFactory for SetDisjointFxn {
  fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
    match args {
      FunctionArgs::Binary(out, arg1, arg2) => {
        let lhs: Ref<MechSet> = unsafe { arg1.as_unchecked() }.clone();
        let rhs: Ref<MechSet> = unsafe { arg2.as_unchecked() }.clone();
        let out: Ref<bool> = unsafe { out.as_unchecked() }.clone();
        Ok(Box::new(SetDisjointFxn {lhs, rhs, out }))
      },
      _ => Err(MechError{file: file!().to_string(), tokens: vec![], msg: format!("{} requires 2 arguments, got {:?}", stringify!($struct_name), args), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments})
    }
  }    
}
impl MechFunctionImpl for SetDisjointFxn {
  fn solve(&self) {
    unsafe {
      // Get mutable reference to the output set
      let mut out_ptr: &mut bool = &mut *(self.out.as_mut_ptr());

      // Get references to lhs and rhs sets
      let lhs_ptr: &MechSet = &*(self.lhs.as_ptr());
      let rhs_ptr: &MechSet = &*(self.rhs.as_ptr());

      // Check if lhs is disjoint of rhs
      *out_ptr = lhs_ptr.set.is_disjoint(&(rhs_ptr.set));
    }
  }
  fn out(&self) -> Value { Value::Bool(self.out.clone()) }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}
#[cfg(feature = "compiler")]
impl MechFunctionCompiler for SetDisjointFxn {
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    let name = format!("SetDisjointFxn");
    compile_binop!(name, self.out, self.lhs, self.rhs, ctx, FeatureFlag::Custom(hash_str("set/disjoint") ));
  }
}
register_descriptor! {
  FunctionDescriptor {
    name: "SetDisjointFxn",
    ptr: SetDisjointFxn::new,
  }
}

fn set_disjoint_fxn(lhs: Value, rhs: Value) -> MResult<Box<dyn MechFunction>> {
  match (lhs, rhs) {
    (Value::Set(lhs), Value::Set(rhs)) => {
      Ok(Box::new(SetDisjointFxn { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(false) }))
    },
    x => Err(MechError{file: file!().to_string(), tokens: vec![], msg: format!("set_disjoint_fxn cannot handle arguments: {:?}", x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
  }
}

pub struct SetDisjoint {}
impl NativeFunctionCompiler for SetDisjoint {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let lhs = arguments[0].clone();
    let rhs = arguments[1].clone();
    match set_disjoint_fxn(lhs.clone(),rhs.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(x) => {
        match (lhs,rhs) {
          (Value::MutableReference(lhs),Value::MutableReference(rhs)) => { set_disjoint_fxn(lhs.borrow().clone(),rhs.borrow().clone()) },
          (lhs,Value::MutableReference(rhs)) => { set_disjoint_fxn(lhs.clone(),rhs.borrow().clone()) },
          (Value::MutableReference(lhs),rhs) => { set_disjoint_fxn(lhs.borrow().clone(),rhs.clone()) },
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

register_descriptor! {
  FunctionCompilerDescriptor {
    name: "set/disjoint",
    ptr: &SetDisjoint{},
  }
}