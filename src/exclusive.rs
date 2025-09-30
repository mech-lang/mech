#![feature(step_trait)]
use crate::*;
use mech_core::*;
use std::iter::Step;
use nalgebra::{
  base::{Matrix as naMatrix, Storage, StorageMut},
  Dim, Scalar,
};
use mech_core::matrix::Matrix;
use std::marker::PhantomData;

// Exclusive ------------------------------------------------------------------

#[derive(Debug)]
pub struct RangeExclusiveScalar<T, MatA> {
  pub from: Ref<T>,
  pub to: Ref<T>,
  pub out: Ref<MatA>,
  phantom: PhantomData<T>,
}
impl<T, R1, C1, S1> MechFunctionFactory for RangeExclusiveScalar<T, naMatrix<T, R1, C1, S1>>
where
  T: Copy + Debug + Clone + Sync + Send + Step + 
  CompileConst + ConstElem + AsValueKind +
  PartialOrd + 'static + One + Add<Output = T>,
  Ref<naMatrix<T, R1, C1, S1>>: ToValue,
  naMatrix<T, R1, C1, S1>: CompileConst + ConstElem + AsNaKind,
  R1: Dim + 'static, C1: Dim, S1: StorageMut<T, R1, C1> + Clone + Debug + 'static,
{
  fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
    match args {
      FunctionArgs::Binary(out, from, to) => {
        let from: Ref<T> = unsafe { from.as_unchecked() }.clone();
        let to: Ref<T> = unsafe { to.as_unchecked() }.clone();
        let out: Ref<naMatrix<T, R1, C1, S1>> = unsafe { out.as_unchecked() }.clone();
        Ok(Box::new(Self { from, to, out, phantom: PhantomData::default() }))
      },
      _ => Err(MechError{file: file!().to_string(), tokens: vec![], msg: format!("RangeExclusiveScalar requires 3 arguments, got {:?}", args), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments})
    }
  }
}
impl<T, R1, C1, S1> MechFunctionImpl for RangeExclusiveScalar<T, naMatrix<T, R1, C1, S1>>
where
  Ref<naMatrix<T, R1, C1, S1>>: ToValue,
  T: Scalar + Clone + Debug + Sync + Send + 'static + PartialOrd + One + Add<Output = T> + 'static,
  R1: Dim, C1: Dim, S1: StorageMut<T, R1, C1> + Clone + Debug,
{
  fn solve(&self) {
    let from_ptr = self.from.as_ptr();
    let to_ptr = self.to.as_ptr();
    let out_ptr = self.out.as_mut_ptr();
    let mut current = from_ptr;
    unsafe {
      for val in (*out_ptr).iter_mut() {
        *val = (*current).clone();
        current = &(*current).clone().add(T::one());
      }
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}
#[cfg(feature = "compiler")]
impl<T, R1, C1, S1> MechFunctionCompiler for RangeExclusiveScalar<T, naMatrix<T, R1, C1, S1>> 
where
  T: CompileConst + ConstElem + AsValueKind,
  naMatrix<T, R1, C1, S1>: CompileConst + ConstElem + AsNaKind,
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    let name = format!("RangeExclusiveScalar<{}{}>", T::as_value_kind(), naMatrix::<T, R1, C1, S1>::as_na_kind());
    compile_binop!(name, self.out, self.from, self.to, ctx, FeatureFlag::Builtin(FeatureKind::RangeExclusive) );
  }
}

#[macro_export]
macro_rules! impl_range_exclusive_match_arms {
  ($fxn:ident, $arg1:expr, $arg2:expr, $($ty:tt, $feat:tt);+ $(;)?) => {
    paste! {
      match ($arg1, $arg2) {
        $(
          #[cfg(feature = $feat)]
          (Value::[<$ty:camel>](from), Value::[<$ty:camel>](to))  => {
            let from_val = *from.borrow();
            let to_val = *to.borrow();
            let diff = to_val - from_val;
            if diff < $ty::zero() {
              return Err(MechError {file: file!().to_string(),tokens: vec![],msg: "Range size must be > 0".to_string(),id: line!(),kind: MechErrorKind::UnhandledFunctionArgumentKind,});
            }
            let size = diff.try_into().map_err(|_| MechError {file: file!().to_string(),tokens: vec![],msg: "Range size overflow".to_string(),id: line!(),kind: MechErrorKind::UnhandledFunctionArgumentKind,})?;            
            let mut vec = vec![from_val; size];
            match size {
              0 => Err(MechError {file: file!().to_string(),tokens: vec![],msg: "Range size must be > 0".to_string(),id: line!(),kind: MechErrorKind::UnhandledFunctionArgumentKind,}),
              #[cfg(feature = "matrix1")]
              1 => {
                register_range!($fxn, $ty, $feat, Matrix1);
                Ok(Box::new($fxn::<$ty,Matrix1<$ty>>{from: from.clone(), to: to.clone(), out: Ref::new(Matrix1::from_element(vec[0])), phantom: PhantomData::default()}))
              }
              #[cfg(all(not(feature = "matrix1"), feature = "matrixd")  )]
              1 => {
                register_range!($fxn, $ty, $feat, DMatrix);
                Ok(Box::new($fxn::<$ty,DMatrix<$ty>>{from: from.clone(), to: to.clone(), out: Ref::new(DMatrix::from_element(1,1,vec[0])), phantom: PhantomData::default()}))
              }
              #[cfg(feature = "row_vector2")]
              2 => {
                register_range!($fxn, $ty, $feat, RowVector2);
                Ok(Box::new($fxn::<$ty,RowVector2<$ty>>{from: from.clone(), to: to.clone(), out: Ref::new(RowVector2::from_vec(vec)), phantom: PhantomData::default()}))
              }
              #[cfg(feature = "row_vector3")]
              3 => {              
                register_range!($fxn, $ty, $feat, RowVector3);
                Ok(Box::new($fxn::<$ty,RowVector3<$ty>>{from: from.clone(), to: to.clone(), out: Ref::new(RowVector3::from_vec(vec)), phantom: PhantomData::default()}))
              }
              #[cfg(feature = "row_vector4")]
              4 => {
                register_range!($fxn, $ty, $feat, RowVector4);
                Ok(Box::new($fxn::<$ty,RowVector4<$ty>>{from: from.clone(), to: to.clone(), out: Ref::new(RowVector4::from_vec(vec)), phantom: PhantomData::default()}))
              }
              #[cfg(feature = "row_vectord")]
              n => {
                register_range!($fxn, $ty, $feat, RowDVector);
                Ok(Box::new($fxn::<$ty,RowDVector<$ty>>{from: from.clone(), to: to.clone(), out: Ref::new(RowDVector::from_vec(vec)), phantom: PhantomData::default()}))
              }
            }
          }
        )+
        x => Err(MechError {file: file!().to_string(),tokens: vec![],msg: format!("{:?}", x),id: line!(),kind: MechErrorKind::UnhandledFunctionArgumentKind,})
      }
    }
  }
}

fn impl_range_exclusive_fxn(arg1_value: Value, arg2_value: Value) -> MResult<Box<dyn MechFunction>> {
  impl_range_exclusive_match_arms!(RangeExclusiveScalar, arg1_value, arg2_value,
    F32, "f32";
    F64, "f64";
    i8,  "i8";
    i16, "i16";
    i32, "i32";
    i64, "i64";
    i128,"i128";
    u8,  "u8";
    u16, "u16";
    u32, "u32";
    u64, "u64";
    u128,"u128";
  )
}

pub struct RangeExclusive {}

impl NativeFunctionCompiler for RangeExclusive {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 2 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let arg1 = arguments[0].clone();
    let arg2 = arguments[1].clone();
    match impl_range_exclusive_fxn(arg1.clone(), arg2.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (arg1,arg2) {
          (Value::MutableReference(arg1),Value::MutableReference(arg2)) => {impl_range_exclusive_fxn(arg1.borrow().clone(),arg2.borrow().clone())}
          (Value::MutableReference(arg1),arg2) => {impl_range_exclusive_fxn(arg1.borrow().clone(),arg2.clone())}
          (arg1,Value::MutableReference(arg2)) => {impl_range_exclusive_fxn(arg1.clone(),arg2.borrow().clone())}
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}