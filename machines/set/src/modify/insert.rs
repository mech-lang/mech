
use crate::*;

use indexmap::set::IndexSet;
use mech_core::set::MechSet;

// Insert ------------------------------------------------------------------------

#[derive(Debug)]
struct SetInsertFxn {
  arg1: Ref<MechSet>,
  arg2: Ref<Value>,
  out: Ref<MechSet>,
}
impl MechFunctionFactory for SetInsertFxn {
  fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
    match args {
      FunctionArgs::Binary(out, arg1, arg2) => {
        let arg1: Ref<MechSet> = unsafe { arg1.as_unchecked() }.clone();
        let arg2: Ref<Value> = unsafe { arg2.as_unchecked() }.clone();
        let out: Ref<MechSet> = unsafe { out.as_unchecked() }.clone();
        Ok(Box::new(SetInsertFxn {arg1, arg2, out }))
      },
      _ => Err(MechError2::new(IncorrectNumberOfArguments { expected: 2, found: args.len() }, None).with_compiler_loc()),
    }
  }    
}

fn match_types(type1: ValueKind, type2: ValueKind) -> (bool, bool)
{
  match (type1, type2)
  {
    (ValueKind::Set(k1, s1), ValueKind::Set(k2, s2)) => {
      let (types_match, _) = match_types(*k1,*k2);
      (types_match, s1==s2)
    },
    (ValueKind::Set(_, _), _) => (false, false),
    (_, ValueKind::Set(_, _)) => (false, false),
    (k1, k2) => (k1==k2, k1==k2),
  }
}

impl MechFunctionImpl for SetInsertFxn {
  fn solve(&self) {
    unsafe {
      // Get mutable reference to the output set
      let mut out_ptr: &mut MechSet = &mut *(self.out.as_mut_ptr());

      // Get references to arg1 and arg2 sets
      let set_ptr: &MechSet = &*(self.arg1.as_ptr());
      let elem_ptr: &Value = &*(self.arg2.as_ptr());

      // Clear the output set first (optional, depending on semantics)
      out_ptr.set.clear();

      let (types_match, sizes_match) = match_types(set_ptr.kind.clone(), elem_ptr.kind().clone());
      // Insert arg2 into arg1
      if(types_match)
      {
        out_ptr.set = set_ptr.set.clone();
        out_ptr.set.insert(elem_ptr.clone());
        if(!sizes_match)
        {
          out_ptr.kind = match out_ptr.kind.clone()
          {
            ValueKind::Set(k1, _) => ValueKind::Set(k1, None),
            _ => ValueKind::Empty
          }
        }
      }
      // Update metadata
      out_ptr.num_elements = out_ptr.set.len();
      if(types_match && sizes_match)
      {
        out_ptr.kind = set_ptr.kind.clone();
      }
    }
  }
  fn out(&self) -> Value { Value::Set(self.out.clone()) }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}
#[cfg(feature = "compiler")]
impl MechFunctionCompiler for SetInsertFxn {
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    let name = format!("SetInsertFxn");
    compile_binop!(name, self.out, self.arg1, self.arg2, ctx, FeatureFlag::Custom(hash_str("set/insert")) );
  }
}
register_descriptor! {
  FunctionDescriptor {
    name: "SetInsertFxn",
    ptr: SetInsertFxn::new,
  }
}

fn set_insert_fxn(arg1: Value, arg2: Value) -> MResult<Box<dyn MechFunction>> {
  match (arg1, arg2) {
    (Value::Set(arg1), arg2) => {
      Ok(Box::new(SetInsertFxn { arg1: arg1.clone(), arg2: Ref::new(arg2.clone()), out: Ref::new(MechSet::new(arg1.borrow().kind.clone(), arg1.borrow().num_elements + 1)) }))
    },
    x => Err(MechError2::new(
      UnhandledFunctionArgumentKind2 { arg: (x.0.kind(), x.1.kind()), fxn_name: "set/insert".to_string() },
      None
    ).with_compiler_loc()),
  }
}

pub struct SetInsert {}
impl NativeFunctionCompiler for SetInsert {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 2 {
      return Err(MechError2::new(IncorrectNumberOfArguments { expected: 2, found: arguments.len() }, None).with_compiler_loc());
    }
    let arg1 = arguments[0].clone();
    let arg2 = arguments[1].clone();
    match set_insert_fxn(arg1.clone(),arg2.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(x) => {
        match (arg1,arg2) {
          (Value::MutableReference(arg1),Value::MutableReference(arg2)) => { set_insert_fxn(arg1.borrow().clone(),arg2.borrow().clone()) },
          (arg1,Value::MutableReference(arg2)) => { set_insert_fxn(arg1.clone(),arg2.borrow().clone()) },
          (Value::MutableReference(arg1),arg2) => { set_insert_fxn(arg1.borrow().clone(),arg2.clone()) },
          x => Err(MechError2::new(
            UnhandledFunctionArgumentKind2 { arg: (x.0.kind(), x.1.kind()), fxn_name: "set/insert".to_string() },
            None
          ).with_compiler_loc()),
        }
      }
    }
  }
}

register_descriptor! {
  FunctionCompilerDescriptor {
    name: "set/insert",
    ptr: &SetInsert{},
  }
}