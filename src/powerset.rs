
use std::cell::RefCell;

use crate::*;

use indexmap::set::IndexSet;
use mech_core::set::MechSet;

// Powerset ------------------------------------------------------------------------

#[derive(Debug)]
struct SetPowersetFxn {
  input: Ref<MechSet>,
  out: Ref<MechSet>,
}
impl MechFunctionFactory for SetPowersetFxn {
  fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
    match args {
      FunctionArgs::Unary(out, input) => {
        let input: Ref<MechSet> = unsafe { input.as_unchecked() }.clone();
        let out: Ref<MechSet> = unsafe { out.as_unchecked() }.clone();
        Ok(Box::new(SetPowersetFxn {input, out }))
      },
      _ => Err(MechError{file: file!().to_string(), tokens: vec![], msg: format!("{} requires 1 argument, got {:?}", stringify!($struct_name), args), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments})
    }
  }    
}

fn powerset_recursive<T>(set: &Vec<T>) -> Vec<Vec<T>>
    where T: std::fmt::Debug + Clone,
{
    if set.len() == 0
    {
        return vec![vec![]];
    }
    let mut with_set = powerset_recursive_aux(set, vec![vec![set[0].clone()]], 1);
    let mut without_set = powerset_recursive_aux(set, vec![vec![]], 1);
    with_set.append(&mut without_set);
    with_set.sort_by(|a, b| a.len().cmp(&b.len()));
    with_set
}

fn powerset_recursive_aux<T>(set: &Vec<T>, mut unfinished_set: Vec<Vec<T>>, index: usize) -> Vec<Vec<T>>
    where T: std::fmt::Debug + Clone,
{
    if index == set.len()
    {
        return unfinished_set;
    }
    let mut with_set = powerset_recursive_aux(set, unfinished_set.iter_mut().
        map(|x| {let mut y = x.clone(); y.push(set[index].clone()); y}).collect(), index + 1);
    let mut without_set = powerset_recursive_aux(set, unfinished_set, index + 1);
    with_set.append(&mut without_set);
    with_set
}

impl MechFunctionImpl for SetPowersetFxn {
  fn solve(&self) {
    unsafe {
      // Get mutable reference to the output set
      let out_ptr: &mut MechSet = &mut *(self.out.as_mut_ptr());

      // Get references to lhs and rhs sets
      let input_ptr: &MechSet = &*(self.input.as_ptr());

      // Clear the output set first (optional, depending on semantics)
      out_ptr.set.clear();

      // Powerset input into output
      let vec_set = powerset_recursive(&(input_ptr.set.clone().into_iter().collect()));
      for set in vec_set
      {
        out_ptr.set.insert(Value::Set(Ref::new(MechSet::from_vec(set))));
      }

      // Update metadata
      out_ptr.num_elements = out_ptr.set.len();
      out_ptr.kind = ValueKind::Set(Box::new(input_ptr.kind.clone()), None);
    }
  }
  fn out(&self) -> Value { Value::Set(self.out.clone()) }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}
#[cfg(feature = "compiler")]
impl MechFunctionCompiler for SetPowersetFxn {
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    let name = format!("SetPowersetFxn");
    compile_unop!(name, self.out, self.input, ctx, FeatureFlag::Custom(hash_str("set/powerset")));
  }
}
register_descriptor! {
  FunctionDescriptor {
    name: "SetPowersetFxn",
    ptr: SetPowersetFxn::new,
  }
}

fn set_powerset_fxn(input: Value) -> MResult<Box<dyn MechFunction>> {
  match (input) {
    (Value::Set(input)) => {
      Ok(Box::new(SetPowersetFxn { input: input.clone(), out: Ref::new(MechSet::new(input.borrow().kind.clone(), 2.pow(input.borrow().num_elements))) }))
    },
    x => Err(MechError{file: file!().to_string(), tokens: vec![], msg: format!("set_powerset_fxn cannot handle arguments: {:?}", x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
  }
}

pub struct SetPowerset {}
impl NativeFunctionCompiler for SetPowerset {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() == 0 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let input = arguments[0].clone();
    match set_powerset_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(x) => {
        match input {
          Value::MutableReference(input) => { set_powerset_fxn(input.borrow().clone()) },
          input => { set_powerset_fxn(input.clone()) },
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

register_descriptor! {
  FunctionCompilerDescriptor {
    name: "set/powerset",
    ptr: &SetPowerset{},
  }
}