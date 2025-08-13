#![feature(step_trait)]
use crate::*;
use mech_core::*;
use std::iter::Step;

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
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

pub struct RangeInclusive {}

impl NativeFunctionCompiler for RangeInclusive {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 2 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    match (arguments[0].clone(), arguments[1].clone()) {
      #[cfg(feature = "i8")]
      (Value::I8(min),   Value::I8(max))   => Ok(Box::new(RangeInclusiveScalar{max,min, out: new_ref(RowDVector::from_element(1,0))})),
      #[cfg(feature = "i16")]
      (Value::I16(min),  Value::I16(max))  => Ok(Box::new(RangeInclusiveScalar{max,min, out: new_ref(RowDVector::from_element(1,0))})),
      #[cfg(feature = "i32")]
      (Value::I32(min),  Value::I32(max))  => Ok(Box::new(RangeInclusiveScalar{max,min, out: new_ref(RowDVector::from_element(1,0))})),
      #[cfg(feature = "i64")]
      (Value::I64(min),  Value::I64(max))  => Ok(Box::new(RangeInclusiveScalar{max,min, out: new_ref(RowDVector::from_element(1,0))})),
      #[cfg(feature = "i128")]
      (Value::I128(min), Value::I128(max)) => Ok(Box::new(RangeInclusiveScalar{max,min, out: new_ref(RowDVector::from_element(1,0))})),
      #[cfg(feature = "u8")]
      (Value::U8(min),   Value::U8(max))   => Ok(Box::new(RangeInclusiveScalar{max,min, out: new_ref(RowDVector::from_element(1,0))})),
      #[cfg(feature = "u16")]
      (Value::U16(min),  Value::U16(max))  => Ok(Box::new(RangeInclusiveScalar{max,min, out: new_ref(RowDVector::from_element(1,0))})),
      #[cfg(feature = "u32")]
      (Value::U32(min),  Value::U32(max))  => Ok(Box::new(RangeInclusiveScalar{max,min, out: new_ref(RowDVector::from_element(1,0))})),
      #[cfg(feature = "u64")]
      (Value::U64(min),  Value::U64(max))  => Ok(Box::new(RangeInclusiveScalar{max,min, out: new_ref(RowDVector::from_element(1,0))})),
      #[cfg(feature = "u128")]
      (Value::U128(min), Value::U128(max)) => Ok(Box::new(RangeInclusiveScalar{max,min, out: new_ref(RowDVector::from_element(1,0))})),
      #[cfg(feature = "f32")]
      (Value::F32(min),  Value::F32(max))  => Ok(Box::new(RangeInclusiveScalar{max,min, out: new_ref(RowDVector::from_element(1,F32::new(0.0)))})),
      #[cfg(feature = "f64")]
      (Value::F64(min),  Value::F64(max))  => Ok(Box::new(RangeInclusiveScalar{max,min, out: new_ref(RowDVector::from_element(1,F64::new(0.0)))})),
      x => Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind})
    }
    }
}