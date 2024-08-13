

#[macro_use]
use crate::stdlib::*;
use std::iter::Step;

// ----------------------------------------------------------------------------
// Range Library
// ----------------------------------------------------------------------------

// Exclusive ------------------------------------------------------------------

#[derive(Debug)]
struct RangeExclusiveScalar<T> {
  max: Ref<T>,
  min: Ref<T>,
  out: Ref<RowDVector<T>>,
}

impl<T> MechFunction for RangeExclusiveScalar<T>
where
    T: Copy + Debug + Clone + Sync + Send + Step + PartialEq + 'static,
    Ref<RowDVector<T>>: ToValue
{
  fn solve(&self) {
    let max_ptr = self.max.as_ptr();
    let min_ptr = self.min.as_ptr();
    let out_ptr = self.out.as_ptr();
    
    unsafe {
      let rng = (*min_ptr..*max_ptr).collect::<Vec<T>>();
      *out_ptr = RowDVector::from_vec(rng);
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:?}", self)}
}

pub struct RangeExclusive {}

impl NativeFunctionCompiler for RangeExclusive {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 2 {
      return Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    match (arguments[0].clone(), arguments[1].clone()) {
      (Value::I8(min), Value::I8(max)) => Ok(Box::new(RangeExclusiveScalar{max,min, out: new_ref(RowDVector::from_element(1,0))})),
      (Value::I16(min), Value::I16(max)) => Ok(Box::new(RangeExclusiveScalar{max,min, out: new_ref(RowDVector::from_element(1,0))})),
      (Value::I32(min), Value::I32(max)) => Ok(Box::new(RangeExclusiveScalar{max,min, out: new_ref(RowDVector::from_element(1,0))})),
      (Value::I64(min), Value::I64(max)) => Ok(Box::new(RangeExclusiveScalar{max,min, out: new_ref(RowDVector::from_element(1,0))})),
      (Value::I128(min), Value::I128(max)) => Ok(Box::new(RangeExclusiveScalar{max,min, out: new_ref(RowDVector::from_element(1,0))})),
      (Value::U8(min), Value::U8(max)) => Ok(Box::new(RangeExclusiveScalar{max,min, out: new_ref(RowDVector::from_element(1,0))})),
      (Value::U16(min), Value::U16(max)) => Ok(Box::new(RangeExclusiveScalar{max,min, out: new_ref(RowDVector::from_element(1,0))})),
      (Value::U32(min), Value::U32(max)) => Ok(Box::new(RangeExclusiveScalar{max,min, out: new_ref(RowDVector::from_element(1,0))})),
      (Value::U64(min), Value::U64(max)) => Ok(Box::new(RangeExclusiveScalar{max,min, out: new_ref(RowDVector::from_element(1,0))})),
      (Value::U128(min), Value::U128(max)) => Ok(Box::new(RangeExclusiveScalar{max,min, out: new_ref(RowDVector::from_element(1,0))})),
      (Value::F32(min), Value::F32(max)) => Ok(Box::new(RangeExclusiveScalar{max,min, out: new_ref(RowDVector::from_element(1,F32::new(0.0)))})),
      (Value::F64(min), Value::F64(max)) => Ok(Box::new(RangeExclusiveScalar{max,min, out: new_ref(RowDVector::from_element(1,F64::new(0.0)))})),
      x => 
        Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind})
    }
  }
}

// Inclusive ------------------------------------------------------------------

#[derive(Debug)]
struct RangeInclusiveScalar<T> {
  max: Ref<T>,
  min: Ref<T>,
  out: Ref<RowDVector<T>>,
}

impl<T> MechFunction for RangeInclusiveScalar<T>
where
    T: Copy + Debug + Clone + Sync + Send + Step + PartialEq + 'static,
    Ref<RowDVector<T>>: ToValue
{
  fn solve(&self) {
    let max_ptr = self.max.as_ptr();
    let min_ptr = self.min.as_ptr();
    let out_ptr = self.out.as_ptr();
    
    unsafe {
      let rng = (*min_ptr..=*max_ptr).collect::<Vec<T>>();
      *out_ptr = RowDVector::from_vec(rng);
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:?}", self)}
}

pub struct RangeInclusive {}

impl NativeFunctionCompiler for RangeInclusive {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 2 {
      return Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    match (arguments[0].clone(), arguments[1].clone()) {
      (Value::I8(min), Value::I8(max)) => Ok(Box::new(RangeInclusiveScalar{max,min, out: new_ref(RowDVector::from_element(1,0))})),
      (Value::I16(min), Value::I16(max)) => Ok(Box::new(RangeInclusiveScalar{max,min, out: new_ref(RowDVector::from_element(1,0))})),
      (Value::I32(min), Value::I32(max)) => Ok(Box::new(RangeInclusiveScalar{max,min, out: new_ref(RowDVector::from_element(1,0))})),
      (Value::I64(min), Value::I64(max)) => Ok(Box::new(RangeInclusiveScalar{max,min, out: new_ref(RowDVector::from_element(1,0))})),
      (Value::I128(min), Value::I128(max)) => Ok(Box::new(RangeInclusiveScalar{max,min, out: new_ref(RowDVector::from_element(1,0))})),
      (Value::U8(min), Value::U8(max)) => Ok(Box::new(RangeInclusiveScalar{max,min, out: new_ref(RowDVector::from_element(1,0))})),
      (Value::U16(min), Value::U16(max)) => Ok(Box::new(RangeInclusiveScalar{max,min, out: new_ref(RowDVector::from_element(1,0))})),
      (Value::U32(min), Value::U32(max)) => Ok(Box::new(RangeInclusiveScalar{max,min, out: new_ref(RowDVector::from_element(1,0))})),
      (Value::U64(min), Value::U64(max)) => Ok(Box::new(RangeInclusiveScalar{max,min, out: new_ref(RowDVector::from_element(1,0))})),
      (Value::U128(min), Value::U128(max)) => Ok(Box::new(RangeInclusiveScalar{max,min, out: new_ref(RowDVector::from_element(1,0))})),
      (Value::F32(min), Value::F32(max)) => Ok(Box::new(RangeInclusiveScalar{max,min, out: new_ref(RowDVector::from_element(1,F32::new(0.0)))})),
      (Value::F64(min), Value::F64(max)) => Ok(Box::new(RangeInclusiveScalar{max,min, out: new_ref(RowDVector::from_element(1,F64::new(0.0)))})),
      x => 
        Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind})
    }
  }
}