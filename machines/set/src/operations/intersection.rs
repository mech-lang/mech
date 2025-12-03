
use crate::*;

use indexmap::set::IndexSet;
use mech_core::set::MechSet;

// Intersection ------------------------------------------------------------------------

#[derive(Debug)]
struct SetIntersectionFxn {
  lhs: Ref<MechSet>,
  rhs: Ref<MechSet>,
  out: Ref<MechSet>,
}
impl MechFunctionFactory for SetIntersectionFxn {
  fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
    match args {
      FunctionArgs::Binary(out, arg1, arg2) => {
        let lhs: Ref<MechSet> = unsafe { arg1.as_unchecked() }.clone();
        let rhs: Ref<MechSet> = unsafe { arg2.as_unchecked() }.clone();
        let out: Ref<MechSet> = unsafe { out.as_unchecked() }.clone();
        Ok(Box::new(SetIntersectionFxn {lhs, rhs, out }))
      },
      _ => Err(MechError2::new(IncorrectNumberOfArguments { expected: 2, found: args.len() }, None).with_compiler_loc()),
    }
  }    
}
impl MechFunctionImpl for SetIntersectionFxn {
  fn solve(&self) {
    unsafe {
      // Get mutable reference to the output set
      let out_ptr: &mut MechSet = &mut *(self.out.as_mut_ptr());

      // Get references to lhs and rhs sets
      let lhs_ptr: &MechSet = &*(self.lhs.as_ptr());
      let rhs_ptr: &MechSet = &*(self.rhs.as_ptr());

      // Clear the output set first (optional, depending on semantics)
      out_ptr.set.clear();

      // Intersection lhs and rhs sets into output
      out_ptr.set = lhs_ptr.set.intersection(&(rhs_ptr.set)).cloned().collect();

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
impl MechFunctionCompiler for SetIntersectionFxn {
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    let name = format!("SetIntersectionFxn");
    compile_binop!(name, self.out, self.lhs, self.rhs, ctx, FeatureFlag::Builtin(FeatureKind::Intersection) );
  }
}
register_descriptor! {
  FunctionDescriptor {
    name: "SetIntersectionFxn",
    ptr: SetIntersectionFxn::new,
  }
}

fn set_intersection_fxn(lhs: Value, rhs: Value) -> MResult<Box<dyn MechFunction>> {
  match (lhs, rhs) {
    (Value::Set(lhs), Value::Set(rhs)) => {
      Ok(Box::new(SetIntersectionFxn { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(MechSet::new(lhs.borrow().kind.clone(), lhs.borrow().num_elements + rhs.borrow().num_elements)) }))
    },
    x => Err(MechError2::new(
      UnhandledFunctionArgumentKind2 { arg: (x.0.kind(), x.1.kind()), fxn_name: "set/intersection".to_string() },
      None
    ).with_compiler_loc()),
  }
}

pub struct SetIntersection {}
impl NativeFunctionCompiler for SetIntersection {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 2 {
      return Err(MechError2::new(IncorrectNumberOfArguments { expected: 2, found: arguments.len() }, None).with_compiler_loc());
    }
    let lhs = arguments[0].clone();
    let rhs = arguments[1].clone();
    match set_intersection_fxn(lhs.clone(),rhs.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(x) => {
        match (lhs,rhs) {
          (Value::MutableReference(lhs),Value::MutableReference(rhs)) => { set_intersection_fxn(lhs.borrow().clone(),rhs.borrow().clone()) },
          (lhs,Value::MutableReference(rhs)) => { set_intersection_fxn(lhs.clone(),rhs.borrow().clone()) },
          (Value::MutableReference(lhs),rhs) => { set_intersection_fxn(lhs.borrow().clone(),rhs.clone()) },
          x => Err(MechError2::new(
            UnhandledFunctionArgumentKind2 { arg: (x.0.kind(), x.1.kind()), fxn_name: "set/intersection".to_string() },
            None
          ).with_compiler_loc()),
        }
      }
    }
  }
}

register_descriptor! {
  FunctionCompilerDescriptor {
    name: "set/intersection",
    ptr: &SetIntersection{},
  }
}