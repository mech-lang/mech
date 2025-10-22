
use crate::*;

use indexmap::set::IndexSet;
use mech_core::set::MechSet;

// Complement ------------------------------------------------------------------------

#[derive(Debug)]
struct SetComplementFxn {
  input: Ref<MechSet>,
  out: Ref<MechSet>,
}
impl MechFunctionFactory for SetComplementFxn {
  fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
    match args {
      FunctionArgs::Binary(out, input) => {
        let input: Ref<MechSet> = unsafe { input.as_unchecked() }.clone();
        let out: Ref<MechSet> = unsafe { out.as_unchecked() }.clone();
        Ok(Box::new(SetComplementFxn {input, out }))
      },
      _ => Err(MechError{file: file!().to_string(), tokens: vec![], msg: format!("{} requires 1 argument, got {:?}", stringify!($struct_name), args), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments})
    }
  }    
}
impl MechFunctionImpl for SetComplementFxn {
  fn solve(&self) {
    unsafe {
      // Get mutable reference to the output set
      let out_ptr: &mut MechSet = &mut *(self.out.as_mut_ptr());

      // Get references to lhs and rhs sets
      let input_ptr: &MechSet = &*(self.input.as_ptr());

      // Clear the output set first (optional, depending on semantics)
      out_ptr.set.clear();

      // Complement input into output
      /*for item in input.set.iter().chain(rhs_ptr.set.iter()) {
        out_ptr.set.insert(item.clone());
      }*/

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
impl MechFunctionCompiler for SetComplementFxn {
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    let name = format!("SetComplementFxn");
    compile_unrop!(name, self.out, self.input, ctx, FeatureFlag::Builtin(FeatureKind::Complement) );
  }
}
register_descriptor! {
  FunctionDescriptor {
    name: "SetComplementFxn",
    ptr: SetComplementFxn::new,
  }
}

fn set_complement_fxn(lhs: Value, rhs: Value) -> MResult<Box<dyn MechFunction>> {
  match (lhs, rhs) {
    (Value::Set(lhs), Value::Set(rhs)) => {
      Ok(Box::new(SetComplementFxn { input: input.clone(), out: Ref::new(MechSet::new(input.borrow().kind.clone(), input.borrow().num_elements)) }))
    },
    x => Err(MechError{file: file!().to_string(), tokens: vec![], msg: format!("set_complement_fxn cannot handle arguments: {:?}", x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
  }
}

pub struct SetComplement {}
impl NativeFunctionCompiler for SetComplement {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() == 0 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let input = arguments[0].clone();
    match set_complement_fxn(input) {
      Ok(fxn) => Ok(fxn),
      Err(x) => {
        match (input) {
          (Value::MutableReference(input)) => { set_complement_fxn(input.borrow().clone()) },
          (input) => { set_complement_fxn(input.clone()) },
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

register_descriptor! {
  FunctionCompilerDescriptor {
    name: "set/complement",
    ptr: &SetComplement{},
  }
}