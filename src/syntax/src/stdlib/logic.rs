#[macro_use]
use crate::stdlib::*;

// ----------------------------------------------------------------------------
// Logic Library
// ----------------------------------------------------------------------------

// And ------------------------------------------------------------------------

#[derive(Debug)]
struct AndScalar {
  lhs: Ref<bool>,
  rhs: Ref<bool>,
  out: Ref<bool>,
}

impl MechFunction for AndScalar {
  fn solve(&self) {
    let lhs_ptr = self.lhs.as_ptr();
    let rhs_ptr = self.rhs.as_ptr();
    let out_ptr = self.out.as_ptr();
    unsafe {*out_ptr = *lhs_ptr && *rhs_ptr;}
  }
  fn out(&self) -> Value {
    Value::Bool(self.out.clone())
  }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

pub struct LogicAnd {}

impl NativeFunctionCompiler for LogicAnd {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 2 {
      return Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    match (arguments[0].clone(), arguments[1].clone()) {
      (Value::Bool(lhs), Value::Bool(rhs)) =>
        Ok(Box::new(AndScalar{lhs, rhs, out: new_ref(false)})),
      (Value::MutableReference(lhs),Value::MutableReference(rhs)) => {
        match (&*lhs.borrow(), &*rhs.borrow()) {
          (Value::Bool(lhs), Value::Bool(rhs)) => Ok(Box::new(AndScalar{lhs: lhs.clone(), rhs: rhs.clone(), out: new_ref(false)})),
          _ => Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind})
        }
      }
      (Value::Bool(lhs),Value::MutableReference(rhs)) => {
        match (&*rhs.borrow()) {
          (Value::Bool(rhs)) => Ok(Box::new(AndScalar{lhs, rhs: rhs.clone(), out: new_ref(false)})),
          _ => Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind})
        }
      }
      (Value::MutableReference(lhs),Value::Bool(rhs)) => {
        match (&*lhs.borrow()) {
          (Value::Bool(lhs)) => Ok(Box::new(AndScalar{lhs: lhs.clone(), rhs, out: new_ref(false)})),
          _ => Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind})
        }
      }
      x => Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind})
    }
  }
}

// Or ------------------------------------------------------------------------

#[derive(Debug)]
struct OrScalar {
  lhs: Ref<bool>,
  rhs: Ref<bool>,
  out: Ref<bool>,
}

impl MechFunction for OrScalar {
  fn solve(&self) {
    let lhs_ptr = self.lhs.as_ptr();
    let rhs_ptr = self.rhs.as_ptr();
    let out_ptr = self.out.as_ptr();
    unsafe {*out_ptr = *lhs_ptr || *rhs_ptr;}
  }
  fn out(&self) -> Value {
    Value::Bool(self.out.clone())
  }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

pub struct LogicOr {}

impl NativeFunctionCompiler for LogicOr {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 2 {
      return Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    match (arguments[0].clone(), arguments[1].clone()) {
      (Value::Bool(lhs), Value::Bool(rhs)) =>
        Ok(Box::new(OrScalar{lhs, rhs, out: new_ref(false)})),
      (Value::MutableReference(lhs),Value::MutableReference(rhs)) => {
        match (&*lhs.borrow(), &*rhs.borrow()) {
          (Value::Bool(lhs), Value::Bool(rhs)) => Ok(Box::new(OrScalar{lhs: lhs.clone(), rhs: rhs.clone(), out: new_ref(false)})),
          _ => Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind})
        }
      }
      (Value::Bool(lhs),Value::MutableReference(rhs)) => {
        match (&*rhs.borrow()) {
          (Value::Bool(rhs)) => Ok(Box::new(OrScalar{lhs, rhs: rhs.clone(), out: new_ref(false)})),
          _ => Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind})
        }
      }
      (Value::MutableReference(lhs),Value::Bool(rhs)) => {
        match (&*lhs.borrow()) {
          (Value::Bool(lhs)) => Ok(Box::new(OrScalar{lhs: lhs.clone(), rhs, out: new_ref(false)})),
          _ => Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind})
        }
      }
      x => Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind})
    }
  }
}

// Not ------------------------------------------------------------------------

#[derive(Debug)]
struct NotScalar {
  lhs: Ref<bool>,
  out: Ref<bool>,
}

impl MechFunction for NotScalar {
  fn solve(&self) {
    let lhs_ptr = self.lhs.as_ptr();
    let out_ptr = self.out.as_ptr();
    unsafe {*out_ptr = !*lhs_ptr;}
  }
  fn out(&self) -> Value {
    Value::Bool(self.out.clone())
  }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

pub struct LogicNot {}

impl NativeFunctionCompiler for LogicNot {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    match (arguments[0].clone()) {
      (Value::Bool(lhs)) =>
        Ok(Box::new(NotScalar{lhs, out: new_ref(false)})),
      (Value::MutableReference(lhs)) => {
        match (&*lhs.borrow()) {
          (Value::Bool(lhs)) => Ok(Box::new(NotScalar{lhs: lhs.clone(), out: new_ref(false)})),
          _ => Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind})
        }
      }
      x => Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind})
    }
  }
}