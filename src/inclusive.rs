#![feature(step_trait)]
use crate::*;
use mech_core::*;
use std::iter::Step;

// Inclusive ------------------------------------------------------------------

#[cfg(feature = "row_vectord")]
#[derive(Debug)]
struct RangeInclusiveScalar<T> {
  max: Ref<T>,
  min: Ref<T>,
  out: Ref<RowDVector<T>>,
}

#[cfg(feature = "row_vectord")]
impl<T> MechFunctionImpl for RangeInclusiveScalar<T>
where
    T: Copy + Debug + Clone + Sync + Send + Step + PartialEq + 'static,
    Ref<RowDVector<T>>: ToValue
{
  fn solve(&self) {
    let max_ptr = self.max.as_ptr();
    let min_ptr = self.min.as_ptr();
    let out_ptr = self.out.as_mut_ptr();
    
    unsafe {
      let rng = (*min_ptr..=*max_ptr).collect::<Vec<T>>();
      *out_ptr = RowDVector::from_vec(rng);
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}
#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for RangeInclusiveScalar<T> 
where
  T: CompileConst + ConstElem + AsValueKind,
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    let name = format!("RangeInclusive<{}>", T::as_value_kind());
    compile_binop!(name, self.out, self.min, self.max, ctx, FeatureFlag::Custom(hash_str("range/inclusive")));
  }
}

pub struct RangeInclusive {}

impl NativeFunctionCompiler for RangeInclusive {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 2 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    match (arguments[0].clone(), arguments[1].clone()) {
      #[cfg(all(feature = "i8", feature = "row_vectord"))]
      (Value::I8(min),   Value::I8(max))   => Ok(Box::new(RangeInclusiveScalar{max,min, out: Ref::new(RowDVector::from_element(1,0))})),
      #[cfg(all(feature = "i16", feature = "row_vectord"))]
      (Value::I16(min),  Value::I16(max))  => Ok(Box::new(RangeInclusiveScalar{max,min, out: Ref::new(RowDVector::from_element(1,0))})),
      #[cfg(all(feature = "i32", feature = "row_vectord"))]
      (Value::I32(min),  Value::I32(max))  => Ok(Box::new(RangeInclusiveScalar{max,min, out: Ref::new(RowDVector::from_element(1,0))})),
      #[cfg(all(feature = "i64", feature = "row_vectord"))]
      (Value::I64(min),  Value::I64(max))  => Ok(Box::new(RangeInclusiveScalar{max,min, out: Ref::new(RowDVector::from_element(1,0))})),
      #[cfg(all(feature = "i128", feature = "row_vectord"))]
      (Value::I128(min), Value::I128(max)) => Ok(Box::new(RangeInclusiveScalar{max,min, out: Ref::new(RowDVector::from_element(1,0))})),
      #[cfg(all(feature = "u8", feature = "row_vectord"))]
      (Value::U8(min),   Value::U8(max))   => Ok(Box::new(RangeInclusiveScalar{max,min, out: Ref::new(RowDVector::from_element(1,0))})),
      #[cfg(all(feature = "u16", feature = "row_vectord"))]
      (Value::U16(min),  Value::U16(max))  => Ok(Box::new(RangeInclusiveScalar{max,min, out: Ref::new(RowDVector::from_element(1,0))})),
      #[cfg(all(feature = "u32", feature = "row_vectord"))]
      (Value::U32(min),  Value::U32(max))  => Ok(Box::new(RangeInclusiveScalar{max,min, out: Ref::new(RowDVector::from_element(1,0))})),
      #[cfg(all(feature = "u64", feature = "row_vectord"))]
      (Value::U64(min),  Value::U64(max))  => Ok(Box::new(RangeInclusiveScalar{max,min, out: Ref::new(RowDVector::from_element(1,0))})),
      #[cfg(all(feature = "u128", feature = "row_vectord"))]
      (Value::U128(min), Value::U128(max)) => Ok(Box::new(RangeInclusiveScalar{max,min, out: Ref::new(RowDVector::from_element(1,0))})),
      #[cfg(all(feature = "f32", feature = "row_vectord"))]
      (Value::F32(min),  Value::F32(max))  => Ok(Box::new(RangeInclusiveScalar{max,min, out: Ref::new(RowDVector::from_element(1,F32::new(0.0)))})),
      #[cfg(all(feature = "f64", feature = "row_vectord"))]
      (Value::F64(min),  Value::F64(max))  => Ok(Box::new(RangeInclusiveScalar{max,min, out: Ref::new(RowDVector::from_element(1,F64::new(0.0)))})),
      x => Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind})
    }
    }
}