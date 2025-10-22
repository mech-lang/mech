
use crate::*;

use indexmap::set::IndexSet;
use mech_core::set::MechSet;

// CartesianProduct ------------------------------------------------------------------------

#[derive(Debug)]
struct SetCartesianProductFxn {
  lhs: Ref<MechSet>,
  rhs: Ref<MechSet>,
  out: Ref<MechSet>,
}
impl MechFunctionFactory for SetCartesianProductFxn {
  fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
    match args {
      FunctionArgs::Binary(out, arg1, arg2) => {
        let lhs: Ref<MechSet> = unsafe { arg1.as_unchecked() }.clone();
        let rhs: Ref<MechSet> = unsafe { arg2.as_unchecked() }.clone();
        let out: Ref<MechSet> = unsafe { out.as_unchecked() }.clone();
        Ok(Box::new(SetCartesianProductFxn {lhs, rhs, out }))
      },
      _ => Err(MechError{file: file!().to_string(), tokens: vec![], msg: format!("{} requires 2 arguments, got {:?}", stringify!($struct_name), args), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments})
    }
  }    
}
impl MechFunctionImpl for SetCartesianProductFxn {
  fn solve(&self) {
    unsafe {
      // Get mutable reference to the output set
      let out_ptr: &mut MechSet = &mut *(self.out.as_mut_ptr());

      // Get references to lhs and rhs sets
      let lhs_ptr: &MechSet = &*(self.lhs.as_ptr());
      let rhs_ptr: &MechSet = &*(self.rhs.as_ptr());

      // Clear the output set first (optional, depending on semantics)
      out_ptr.set.clear();

      // Cartesian product lhs and rhs sets into output
      for elem1 in lhs_ptr.set.clone()
      {
        for elem2 in rhs_ptr.set.clone()
        {
          out_ptr.set.insert(Value::Tuple(Ref::new(MechTuple{elements: vec![Box::new(elem1.clone()), Box::new(elem2)]})));
        }
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
impl MechFunctionCompiler for SetCartesianProductFxn {
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    let name = format!("SetCartesianProductFxn");
    compile_binop!(name, self.out, self.lhs, self.rhs, ctx, FeatureFlag::Custom(hash_str("set/cartesianproduct")) );
  }
}
register_descriptor! {
  FunctionDescriptor {
    name: "SetCartesianProductFxn",
    ptr: SetCartesianProductFxn::new,
  }
}

fn set_cartesianproduct_fxn(lhs: Value, rhs: Value) -> MResult<Box<dyn MechFunction>> {
  match (lhs, rhs) {
    (Value::Set(lhs), Value::Set(rhs)) => {
      Ok(Box::new(SetCartesianProductFxn { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(MechSet::new(ValueKind::Tuple(vec![lhs.borrow().kind.clone(), rhs.borrow().kind.clone()]), lhs.borrow().num_elements * rhs.borrow().num_elements)) }))
    },
    x => Err(MechError{file: file!().to_string(), tokens: vec![], msg: format!("set_cartesianproduct_fxn cannot handle arguments: {:?}", x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
  }
}

pub struct SetCartesianProduct {}
impl NativeFunctionCompiler for SetCartesianProduct {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let lhs = arguments[0].clone();
    let rhs = arguments[1].clone();
    match set_cartesianproduct_fxn(lhs.clone(),rhs.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(x) => {
        match (lhs,rhs) {
          (Value::MutableReference(lhs),Value::MutableReference(rhs)) => { set_cartesianproduct_fxn(lhs.borrow().clone(),rhs.borrow().clone()) },
          (lhs,Value::MutableReference(rhs)) => { set_cartesianproduct_fxn(lhs.clone(),rhs.borrow().clone()) },
          (Value::MutableReference(lhs),rhs) => { set_cartesianproduct_fxn(lhs.borrow().clone(),rhs.clone()) },
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

register_descriptor! {
  FunctionCompilerDescriptor {
    name: "set/cartesianproduct",
    ptr: &SetCartesianProduct{},
  }
}