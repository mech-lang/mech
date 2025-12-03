
use crate::*;

use indexmap::set::IndexSet;
use mech_core::set::MechSet;

// Remove ------------------------------------------------------------------------

#[derive(Debug)]
struct SetRemoveFxn {
  arg1: Ref<MechSet>,
  arg2: Ref<Value>,
  out: Ref<MechSet>,
}
impl MechFunctionFactory for SetRemoveFxn {
  fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
    match args {
      FunctionArgs::Binary(out, arg1, arg2) => {
        let arg1: Ref<MechSet> = unsafe { arg1.as_unchecked() }.clone();
        let arg2: Ref<Value> = unsafe { arg2.as_unchecked() }.clone();
        let out: Ref<MechSet> = unsafe { out.as_unchecked() }.clone();
        Ok(Box::new(SetRemoveFxn {arg1, arg2, out }))
      },
      _ => Err(MechError2::new(IncorrectNumberOfArguments { expected: 2, found: args.len() }, None).with_compiler_loc()),
    }
  }    
}
impl MechFunctionImpl for SetRemoveFxn {
  fn solve(&self) {
    unsafe {
      // Get mutable reference to the output set
      let mut out_ptr: &mut MechSet = &mut *(self.out.as_mut_ptr());

      // Get references to arg1 and arg2 sets
      let set_ptr: &MechSet = &*(self.arg1.as_ptr());
      let elem_ptr: &Value = &*(self.arg2.as_ptr());

      // Clear the output set first (optional, depending on semantics)
      out_ptr.set.clear();

      // Remove arg2 into arg1
      if(set_ptr.kind == elem_ptr.kind())
      {
        out_ptr.set = set_ptr.set.clone();
        out_ptr.set.shift_remove(elem_ptr);
      }
      // Update metadata
      out_ptr.num_elements = out_ptr.set.len();
      out_ptr.kind = if out_ptr.set.len() > 0 {
      out_ptr.set.iter().next().unwrap().kind()
      } else {
        ValueKind::Empty
      };
    }
  }
  fn out(&self) -> Value { Value::Set(self.out.clone()) }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}
#[cfg(feature = "compiler")]
impl MechFunctionCompiler for SetRemoveFxn {
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    let name = format!("SetRemoveFxn");
    compile_binop!(name, self.out, self.arg1, self.arg2, ctx, FeatureFlag::Custom(hash_str("set/remove")) );
  }
}
register_descriptor! {
  FunctionDescriptor {
    name: "SetRemoveFxn",
    ptr: SetRemoveFxn::new,
  }
}

fn set_remove_fxn(arg1: Value, arg2: Value) -> MResult<Box<dyn MechFunction>> {
  match (arg1, arg2) {
    (Value::Set(arg1), arg2) => {
      Ok(Box::new(SetRemoveFxn { arg1: arg1.clone(), arg2: Ref::new(arg2.clone()), out: Ref::new(MechSet::new(arg1.borrow().kind.clone(), arg1.borrow().num_elements + 1)) }))
    },
    x => Err(MechError2::new(
      UnhandledFunctionArgumentKind2 { arg: (x.0.kind(), x.1.kind()), fxn_name: "set/remove".to_string() },
      None
    ).with_compiler_loc()),
  }
}

pub struct SetRemove {}
impl NativeFunctionCompiler for SetRemove {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 2 {
      return Err(MechError2::new(IncorrectNumberOfArguments { expected: 2, found: arguments.len() }, None).with_compiler_loc());
    }
    let arg1 = arguments[0].clone();
    let arg2 = arguments[1].clone();
    match set_remove_fxn(arg1.clone(),arg2.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(x) => {
        match (arg1,arg2) {
          (Value::MutableReference(arg1),Value::MutableReference(arg2)) => { set_remove_fxn(arg1.borrow().clone(),arg2.borrow().clone()) },
          (arg1,Value::MutableReference(arg2)) => { set_remove_fxn(arg1.clone(),arg2.borrow().clone()) },
          (Value::MutableReference(arg1),arg2) => { set_remove_fxn(arg1.borrow().clone(),arg2.clone()) },
          x => Err(MechError2::new(
            UnhandledFunctionArgumentKind2 { arg: (x.0.kind(), x.1.kind()), fxn_name: "set/remove".to_string() },
            None
          ).with_compiler_loc()),
        }
      }
    }
  }
}

register_descriptor! {
  FunctionCompilerDescriptor {
    name: "set/remove",
    ptr: &SetRemove{},
  }
}