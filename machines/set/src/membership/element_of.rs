use crate::*;

use indexmap::set::IndexSet;
use mech_core::set::MechSet;

// Element Of --------------------------------------------------------------------
//
// Returns true iff elem ∈ set
//

#[derive(Debug)]
struct SetElementOfFxn {
  elem: Ref<Value>,
  set: Ref<MechSet>,
  out: Ref<bool>,
}

impl MechFunctionFactory for SetElementOfFxn {
  fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
    match args {
      FunctionArgs::Binary(out, arg1, arg2) => {
        let elem: Ref<Value> = unsafe { arg1.as_unchecked() }.clone();
        let set: Ref<MechSet> = unsafe { arg2.as_unchecked() }.clone();
        let out: Ref<bool> = unsafe { out.as_unchecked() }.clone();
        Ok(Box::new(SetElementOfFxn { elem, set, out }))
      },
      _ => Err(MechError2::new(IncorrectNumberOfArguments { expected: 2, found: args.len() }, None).with_compiler_loc()),
    }
  }
}

impl MechFunctionImpl for SetElementOfFxn {
  fn solve(&self) {
    unsafe {
      let out_ptr: &mut bool = &mut *(self.out.as_mut_ptr());
      let elem_ptr: &Value = &*(self.elem.as_ptr());
      let set_ptr: &MechSet = &*(self.set.as_ptr());

      // Only true if kinds are compatible and the set contains elem.
      if set_ptr.kind == elem_ptr.kind() {
        *out_ptr = set_ptr.set.contains(elem_ptr);
      } else {
        *out_ptr = false;
      }
    }
  }
  fn out(&self) -> Value { Value::Bool(self.out.clone()) }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}

#[cfg(feature = "compiler")]
impl MechFunctionCompiler for SetElementOfFxn {
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    let name = "SetElementOfFxn".to_string();
    // Builtin operator ∈
    compile_binop!(name, self.out, self.elem, self.set, ctx, FeatureFlag::Builtin(FeatureKind::ElementOf));
  }
}

register_descriptor! {
  FunctionDescriptor {
    name: "SetElementOfFxn",
    ptr: SetElementOfFxn::new,
  }
}

fn set_element_of_fxn(elem: Value, set: Value) -> MResult<Box<dyn MechFunction>> {
  match (elem, set) {
    (elem, Value::Set(set)) => {
      Ok(Box::new(SetElementOfFxn { elem: Ref::new(elem.clone()), set: set.clone(), out: Ref::new(false) }))
    },
    x => Err(MechError2::new(
      UnhandledFunctionArgumentKind2 {
        arg: (x.0.kind(), x.1.kind()),
        fxn_name: "set/element-of".to_string(),
      }, None
    ).with_compiler_loc()),
  }
}

pub struct SetElementOf {}
impl NativeFunctionCompiler for SetElementOf {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 2 {
      return Err(MechError2::new(IncorrectNumberOfArguments { expected: 2, found: arguments.len() }, None).with_compiler_loc());
    }
    let elem = arguments[0].clone();
    let set = arguments[1].clone();
    match set_element_of_fxn(elem.clone(), set.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (elem, set) {
          (Value::MutableReference(elem), Value::MutableReference(set)) => set_element_of_fxn(elem.borrow().clone(), set.borrow().clone()),
          (elem, Value::MutableReference(set)) => set_element_of_fxn(elem.clone(), set.borrow().clone()),
          (Value::MutableReference(elem), set) => set_element_of_fxn(elem.borrow().clone(), set.clone()),
          x => Err(MechError2::new(
            UnhandledFunctionArgumentKind2 { arg: (x.0.kind(), x.1.kind()), fxn_name: "set/element-of".to_string() },
            None
          ).with_compiler_loc()),
        }
      }
    }
  }
}

register_descriptor! {
  FunctionCompilerDescriptor {
    name: "set/element-of",
    ptr: &SetElementOf{},
  }
}