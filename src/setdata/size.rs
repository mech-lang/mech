use crate::*;

use indexmap::set::IndexSet;
use mech_core::set::MechSet;

// Size --------------------------------------------------------------------------
//
// Returns the cardinality |A| as a u64.
//

#[derive(Debug)]
struct SetSizeFxn {
  input: Ref<MechSet>,
  out: Ref<u64>,
}

impl MechFunctionFactory for SetSizeFxn {
  fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
    match args {
      FunctionArgs::Unary(out, arg1) => {
        let input: Ref<MechSet> = unsafe { arg1.as_unchecked() }.clone();
        let out: Ref<u64> = unsafe { out.as_unchecked() }.clone();
        Ok(Box::new(SetSizeFxn { input, out }))
      },
      _ => Err(MechError{
        file: file!().to_string(),
        tokens: vec![],
        msg: format!("{} requires 1 argument, got {:?}", stringify!($struct_name), args),
        id: line!(),
        kind: MechErrorKind::IncorrectNumberOfArguments
      })
    }
  }
}

impl MechFunctionImpl for SetSizeFxn {
  fn solve(&self) {
    unsafe {
      let out_ptr: &mut u64 = &mut *(self.out.as_mut_ptr());
      let input_ptr: &MechSet = &*(self.input.as_ptr());
      // Uses the internal IndexSet length
      *out_ptr = input_ptr.set.len() as u64;
    }
  }
  fn out(&self) -> Value { Value::U64(self.out.clone()) }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}

#[cfg(feature = "compiler")]
impl MechFunctionCompiler for SetSizeFxn {
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    let name = "SetSizeFxn".to_string();
    // Custom feature route: set/size
    compile_unop!(name, self.out, self.input, ctx, FeatureFlag::Custom(hash_str("set/size")));
  }
}

register_descriptor! {
  FunctionDescriptor {
    name: "SetSizeFxn",
    ptr: SetSizeFxn::new,
  }
}

fn set_size_fxn(input: Value) -> MResult<Box<dyn MechFunction>> {
  match input {
    Value::Set(s) => Ok(Box::new(SetSizeFxn { input: s.clone(), out: Ref::new(0u64) })),
    x => Err(MechError{
      file: file!().to_string(),
      tokens: vec![],
      msg: format!("set_size_fxn cannot handle argument: {:?}", x),
      id: line!(),
      kind: MechErrorKind::UnhandledFunctionArgumentKind
    }),
  }
}

pub struct SetSize {}
impl NativeFunctionCompiler for SetSize {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() == 0 {
      return Err(MechError{
        file: file!().to_string(),
        tokens: vec![],
        msg: "".to_string(),
        id: line!(),
        kind: MechErrorKind::IncorrectNumberOfArguments
      });
    }
    let input = arguments[0].clone();
    match set_size_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match input {
          Value::MutableReference(r) => set_size_fxn(r.borrow().clone()),
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
    name: "set/size",
    ptr: &SetSize{},
  }
}