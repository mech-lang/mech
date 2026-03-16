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
pub struct RangeIncrementInclusiveScalar<T, MatA> {
  pub from: Ref<T>,
  pub step: Ref<T>,
  pub to: Ref<T>,
  pub out: Ref<MatA>,
  phantom: PhantomData<T>,
}
impl<T, R1, C1, S1> MechFunctionFactory for RangeIncrementInclusiveScalar<T, naMatrix<T, R1, C1, S1>>
where
  T: Copy + Debug + Clone + Sync + Send + 
  CompileConst + ConstElem + AsValueKind +
  PartialOrd + 'static + One + Add<Output = T>,
  Ref<naMatrix<T, R1, C1, S1>>: ToValue,
  naMatrix<T, R1, C1, S1>: CompileConst + ConstElem + AsNaKind,
  R1: Dim + 'static, C1: Dim, S1: StorageMut<T, R1, C1> + Clone + Debug + 'static,
{
  fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
    match args {
      FunctionArgs::Ternary(out, from, step, to) => {
        let from: Ref<T> = unsafe { from.as_unchecked() }.clone();
        let step: Ref<T> = unsafe { step.as_unchecked() }.clone();
        let to: Ref<T> = unsafe { to.as_unchecked() }.clone();
        let out: Ref<naMatrix<T, R1, C1, S1>> = unsafe { out.as_unchecked() }.clone();
        Ok(Box::new(Self { from, step, to, out, phantom: PhantomData::default() }))
      },
      _ => Err(MechError::new(
          IncorrectNumberOfArguments { expected: 3, found: args.len() },
          None
        ).with_compiler_loc()
      ),
    }
  }
}
impl<T, R1, C1, S1> MechFunctionImpl for RangeIncrementInclusiveScalar<T, naMatrix<T, R1, C1, S1>>
where
  Ref<naMatrix<T, R1, C1, S1>>: ToValue,
  T: Copy + Scalar + Clone + Debug + Sync + Send + 'static + PartialOrd + One + Add<Output = T> + 'static,
  R1: Dim, C1: Dim, S1: StorageMut<T, R1, C1> + Clone + Debug,
{
  fn solve(&self) {
    unsafe {
      let out_ptr = self.out.as_ptr() as *mut naMatrix<T, R1, C1, S1>;
      let mut current = *self.from.as_ptr();
      let step = *self.step.as_ptr();
      for i in 0..(*out_ptr).len() {
        (&mut (*out_ptr))[i] = current;
        current = current + step;
      }
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}
#[cfg(feature = "compiler")]
impl<T, R1, C1, S1> MechFunctionCompiler for RangeIncrementInclusiveScalar<T, naMatrix<T, R1, C1, S1>> 
where
  T: CompileConst + ConstElem + AsValueKind,
  naMatrix<T, R1, C1, S1>: CompileConst + ConstElem + AsNaKind,
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    let name = format!("RangeIncrementInclusiveScalar<{}{}>", T::as_value_kind(), naMatrix::<T, R1, C1, S1>::as_na_kind());
    compile_ternop!(name, self.out, self.from, self.step, self.to, ctx, FeatureFlag::Builtin(FeatureKind::RangeInclusive) );
  }
}

#[macro_export]
macro_rules! impl_range_increment_inclusive_match_arms {
  ($fxn:ident, $arg1:expr, $arg2:expr, $arg3:expr, $($ty:tt, $feat:tt);+ $(;)?) => {
    paste! {
      match ($arg1, $arg2, $arg3) {
        $(
          #[cfg(feature = $feat)]
          (Value::[<$ty:camel>](from), Value::[<$ty:camel>](step), Value::[<$ty:camel>](to))  => {
            let from_val = *from.borrow();
            let step_val = *step.borrow();
            let to_val = *to.borrow();
            let diff = to_val - from_val;
            if diff < $ty::zero() {
              return Err(MechError::new(
                EmptyRangeError{},
                None
              ).with_compiler_loc());
            }
            let size = {
              let diff = to_val as f64 - from_val as f64;
              let step = step_val as f64;
              if step == 0.0 {
                return Err(MechError::new(EmptyRangeError {}, None).with_compiler_loc());
              }
              if (diff > 0.0 && step > 0.0) || (diff < 0.0 && step < 0.0) {
                (diff / step).floor() as usize + 1
              } else if diff == 0.0 {
                1
              } else {
                return Err(MechError::new(EmptyRangeError {}, None).with_compiler_loc());
              }
            };
            let mut vec = vec![from_val; size];
            match size {
              0 => Err(MechError::new(
                EmptyRangeError{},
                None
              ).with_compiler_loc()),
              #[cfg(feature = "matrix1")]
              1 => {
                register_range!($fxn, $ty, $feat, Matrix1);
                Ok(Box::new($fxn::<$ty,Matrix1<$ty>>{from: from.clone(), step: step.clone(), to: to.clone(), out: Ref::new(Matrix1::from_element(vec[0])), phantom: PhantomData::default()}))
              }
              #[cfg(all(not(feature = "matrix1"), feature = "matrixd")  )]
              1 => {
                register_range!($fxn, $ty, $feat, DMatrix);
                Ok(Box::new($fxn::<$ty,DMatrix<$ty>>{from: from.clone(), step: step.clone(), to: to.clone(), out: Ref::new(DMatrix::from_element(1,1,vec[0])), phantom: PhantomData::default()}))
              }
              #[cfg(feature = "row_vector2")]
              2 => {
                register_range!($fxn, $ty, $feat, RowVector2);
                Ok(Box::new($fxn::<$ty,RowVector2<$ty>>{from: from.clone(), step: step.clone(), to: to.clone(), out: Ref::new(RowVector2::from_vec(vec)), phantom: PhantomData::default()}))
              }
              #[cfg(feature = "row_vector3")]
              3 => {              
                register_range!($fxn, $ty, $feat, RowVector3);
                Ok(Box::new($fxn::<$ty,RowVector3<$ty>>{from: from.clone(), step: step.clone(), to: to.clone(), out: Ref::new(RowVector3::from_vec(vec)), phantom: PhantomData::default()}))
              }
              #[cfg(feature = "row_vector4")]
              4 => {
                register_range!($fxn, $ty, $feat, RowVector4);
                Ok(Box::new($fxn::<$ty,RowVector4<$ty>>{from: from.clone(), step: step.clone(), to: to.clone(), out: Ref::new(RowVector4::from_vec(vec)), phantom: PhantomData::default()}))
              }
              #[cfg(feature = "row_vectord")]
              n => {
                register_range!($fxn, $ty, $feat, RowDVector);
                Ok(Box::new($fxn::<$ty,RowDVector<$ty>>{from: from.clone(), step: step.clone(), to: to.clone(), out: Ref::new(RowDVector::from_vec(vec)), phantom: PhantomData::default()}))
              }
            }
          }
        )+
        (arg1,arg2,arg3) => Err(MechError::new(
          UnhandledFunctionArgumentKind3 {arg: (arg1.kind(),arg2.kind(), arg3.kind()), fxn_name: stringify!($fxn).to_string() },
          None
        ).with_compiler_loc()),
      }
    }
  }
}

fn impl_range_increment_inclusive_fxn(arg1_value: Value, arg2_value: Value, arg3_value: Value) -> MResult<Box<dyn MechFunction>> {
  impl_range_increment_inclusive_match_arms!(RangeIncrementInclusiveScalar, arg1_value, arg2_value, arg3_value,
    f32, "f32";
    f64, "f64";
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

pub struct RangeIncrementInclusive {}

impl NativeFunctionCompiler for RangeIncrementInclusive {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 3 {
      return Err(MechError::new(IncorrectNumberOfArguments { expected: 3, found: arguments.len() },None).with_compiler_loc());
    }
    let arg1 = arguments[0].clone();
    let arg2 = arguments[1].clone();
    let arg3 = arguments[2].clone();
    match impl_range_increment_inclusive_fxn(arg1.clone(), arg2.clone(), arg3.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (arg1, arg2, arg3) {
          (Value::MutableReference(arg1), Value::MutableReference(arg2), Value::MutableReference(arg3)) => {
            impl_range_increment_inclusive_fxn(arg1.borrow().clone(), arg2.borrow().clone(), arg3.borrow().clone())
          }
          (Value::MutableReference(arg1), Value::MutableReference(arg2), arg3) => {
            impl_range_increment_inclusive_fxn(arg1.borrow().clone(), arg2.borrow().clone(), arg3.clone())
          }
          (Value::MutableReference(arg1), arg2, Value::MutableReference(arg3)) => {
            impl_range_increment_inclusive_fxn(arg1.borrow().clone(), arg2.clone(), arg3.borrow().clone())
          }
          (Value::MutableReference(arg1), arg2, arg3) => {
            impl_range_increment_inclusive_fxn(arg1.borrow().clone(), arg2.clone(), arg3.clone())
          }
          (arg1, Value::MutableReference(arg2), Value::MutableReference(arg3)) => {
            impl_range_increment_inclusive_fxn(arg1.clone(), arg2.borrow().clone(), arg3.borrow().clone())
          }
          (arg1, Value::MutableReference(arg2), arg3) => {
            impl_range_increment_inclusive_fxn(arg1.clone(), arg2.borrow().clone(), arg3.clone())
          }
          (arg1, arg2, Value::MutableReference(arg3)) => {
            impl_range_increment_inclusive_fxn(arg1.clone(), arg2.clone(), arg3.borrow().clone())
          }
          (arg1, arg2, arg3) => Err(MechError::new(
            UnhandledFunctionArgumentKind3 { arg: (arg1.kind(), arg2.kind(), arg3.kind()), fxn_name: "range/inclusive-increment".to_string() },
            None
          ).with_compiler_loc()),
        }
      }
    }
  }
}

register_descriptor! {
  FunctionCompilerDescriptor {
    name: "range/inclusive-increment",
    ptr: &RangeIncrementInclusive{},
  }
}