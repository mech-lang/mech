#[macro_use]
use crate::stdlib::*;

// ----------------------------------------------------------------------------
// Range Library
// ----------------------------------------------------------------------------

// Exclusive ------------------------------------------------------------------

#[derive(Debug)]
struct RangeExclusiveScalar {
  max: Ref<i64>,
  min: Ref<i64>,
  out: Ref<RowDVector<i64>>,
}

impl MechFunction for RangeExclusiveScalar {
  fn solve(&self) {
    let max_ptr = self.max.as_ptr();
    let min_ptr = self.min.as_ptr();
    let out_ptr = self.out.as_ptr();
    
    unsafe {
      let rng = (*min_ptr..*max_ptr).collect::<Vec<i64>>();
      *out_ptr = RowDVector::from_vec(rng);
    }
  }
  fn out(&self) -> Value {
    Value::MatrixI64(Matrix::<i64>::RowDVector(self.out.clone()))
  }
  fn to_string(&self) -> String { format!("{:?}", self)}
}

pub struct RangeExclusive {}

impl NativeFunctionCompiler for RangeExclusive {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 2 {
      return Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    match (arguments[0].clone(), arguments[1].clone()) {
      (Value::I64(min), Value::I64(max)) =>
        Ok(Box::new(RangeExclusiveScalar{max,min, out: new_ref(RowDVector::from_element(1,0))})),
      x => 
        Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind})
    }
  }
}

// Inclusive ------------------------------------------------------------------

#[derive(Debug)]
struct RangeInclusiveScalar {
  max: Ref<i64>,
  min: Ref<i64>,
  out: Ref<RowDVector<i64>>,
}

impl MechFunction for RangeInclusiveScalar {
  fn solve(&self) {
    let max_ptr = self.max.as_ptr();
    let min_ptr = self.min.as_ptr();
    let out_ptr = self.out.as_ptr();
    unsafe {
      let rng = (*min_ptr..=*max_ptr).collect::<Vec<i64>>();
      *out_ptr = RowDVector::from_vec(rng);
    }
  }
  fn out(&self) -> Value {
    Value::MatrixI64(Matrix::<i64>::RowDVector(self.out.clone()))
  }
  fn to_string(&self) -> String { format!("{:?}", self)}
}

pub struct RangeInclusive {}

impl NativeFunctionCompiler for RangeInclusive {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 2 {
      return Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    match (arguments[0].clone(), arguments[1].clone()) {
      (Value::I64(min), Value::I64(max)) =>
        Ok(Box::new(RangeInclusiveScalar{max,min, out: new_ref(RowDVector::from_element(1,0))})),
      x => 
        Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind})
    }
  }
}