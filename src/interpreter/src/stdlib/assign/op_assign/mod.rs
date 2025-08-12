#[macro_use]
use crate::stdlib::*;

use std::fmt::Debug;
use std::marker::PhantomData;
use nalgebra::{
  base::{Matrix as naMatrix, Storage, StorageMut},
  Dim, Scalar,
};

pub mod add_assign;
pub mod sub_assign;
pub mod div_assign;
pub mod mul_assign;

pub use self::add_assign::*;
pub use self::sub_assign::*;
pub use self::div_assign::*;
pub use self::mul_assign::*;

#[macro_export]
macro_rules! impl_op_assign_range_fxn_s {
  ($struct_name:ident, $op:ident, $ix:ty) => {
    #[derive(Debug)]
    pub struct $struct_name<T, MatA, IxVec> {
      pub source: Ref<T>,
      pub ixes: Ref<IxVec>,
      pub sink: Ref<MatA>,
      pub _marker: PhantomData<T>,
    }
    impl<T, R1, C1, S1, IxVec> MechFunction for $struct_name<T, naMatrix<T, R1, C1, S1>, IxVec>
    where
      Ref<naMatrix<T, R1, C1, S1>>: ToValue,
      T: Copy + Debug + Clone + Sync + Send + 'static +
        Div<Output = T> + DivAssign +
        Add<Output = T> + AddAssign +
        Sub<Output = T> + SubAssign +
        Mul<Output = T> + MulAssign +
        Zero + One +
        PartialEq + PartialOrd,
      IxVec: AsRef<[$ix]> + Debug,
      R1: Dim, C1: Dim, S1: StorageMut<T, R1, C1> + Clone + Debug,
    {
      fn solve(&self) {
        unsafe {
          let sink_ptr = &mut *self.sink.as_ptr();
          let source_ptr = &*self.source.as_ptr();
          let ix_ptr = &(*self.ixes.as_ptr()).as_ref();
          $op!(source_ptr,ix_ptr,sink_ptr);
        }
      }
      fn out(&self) -> Value {self.sink.to_value()}
      fn to_string(&self) -> String {format!("{:#?}", self)}
    }};}

#[macro_export]
macro_rules! impl_op_assign_range_fxn_v {
  ($struct_name:ident, $op:ident, $ix:ty) => {
    #[derive(Debug)]
    pub struct $struct_name<T, MatA, MatB, IxVec> {
      pub source: Ref<MatB>,
      pub ixes: Ref<IxVec>,
      pub sink: Ref<MatA>,
      pub _marker: PhantomData<T>,
    }

    impl<T, R1, C1, S1, R2, C2, S2, IxVec>
      MechFunction for $struct_name<T, naMatrix<T, R1, C1, S1>, naMatrix<T, R2, C2, S2>, IxVec>
    where
      Ref<naMatrix<T, R1, C1, S1>>: ToValue,
      T: Copy + Debug + Clone + Sync + Send + 'static +
        Div<Output = T> + DivAssign +
        Add<Output = T> + AddAssign +
        Sub<Output = T> + SubAssign +
        Mul<Output = T> + MulAssign +
        Zero + One +
        PartialEq + PartialOrd,
      IxVec: AsRef<[$ix]> + Debug,
      R1: Dim, C1: Dim, S1: StorageMut<T, R1, C1> + Clone + Debug,
      R2: Dim, C2: Dim, S2: Storage<T, R2, C2> + Clone + Debug,
    {
      fn solve(&self) {
        unsafe {
          let sink_ptr = &mut *self.sink.as_ptr();
          let source_ptr = &*self.source.as_ptr();
          let ix_ptr = &(*self.ixes.as_ptr()).as_ref();
          $op!(source_ptr,ix_ptr,sink_ptr);
        }
      }
      fn out(&self) -> Value {self.sink.to_value()}
      fn to_string(&self) -> String {format!("{:#?}", self)}
    }};}

#[macro_export]
macro_rules! impl_op_assign_value_match_arms {
  ($lib:ident, $arg:expr,$($value_kind:ident, $feature:tt);+ $(;)?) => {
    paste::paste! {
      match $arg {
        $(
          #[cfg(feature = $feature)]
          (Value::$value_kind(sink), Value::$value_kind(source)) => Ok(Box::new([<$lib AssignSS>] { sink: sink.clone(), source: source.clone() })),
          #[cfg(all(feature = $feature, feature = "Matrix1"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix1(sink)), Value::[<Matrix $value_kind>](Matrix::Matrix1(source))) => Ok(Box::new([<$lib AssignVV>] { sink: sink.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $feature, feature = "Matrix2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2(sink)), Value::[<Matrix $value_kind>](Matrix::Matrix2(source))) => Ok(Box::new([<$lib AssignVV>] { sink: sink.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $feature, feature = "Matrix2x3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2x3(sink)), Value::[<Matrix $value_kind>](Matrix::Matrix2x3(source))) => Ok(Box::new([<$lib AssignVV>] { sink: sink.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $feature, feature = "Matrix3x2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3x2(sink)), Value::[<Matrix $value_kind>](Matrix::Matrix3x2(source))) => Ok(Box::new([<$lib AssignVV>] { sink: sink.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $feature, feature = "Matrix3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3(sink)), Value::[<Matrix $value_kind>](Matrix::Matrix3(source))) => Ok(Box::new([<$lib AssignVV>] { sink: sink.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $feature, feature = "Matrix4"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix4(sink)), Value::[<Matrix $value_kind>](Matrix::Matrix4(source))) => Ok(Box::new([<$lib AssignVV>] { sink: sink.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $feature, feature = "MatrixD"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)), Value::[<Matrix $value_kind>](Matrix::DMatrix(source))) => Ok(Box::new([<$lib AssignVV>] { sink: sink.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $feature, feature = "Vector2"))]
          (Value::[<Matrix $value_kind>](Matrix::Vector2(sink)), Value::[<Matrix $value_kind>](Matrix::Vector2(source))) => Ok(Box::new([<$lib AssignVV>] { sink: sink.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $feature, feature = "Vector3"))]
          (Value::[<Matrix $value_kind>](Matrix::Vector3(sink)), Value::[<Matrix $value_kind>](Matrix::Vector3(source))) => Ok(Box::new([<$lib AssignVV>] { sink: sink.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $feature, feature = "Vector4"))]
          (Value::[<Matrix $value_kind>](Matrix::Vector4(sink)), Value::[<Matrix $value_kind>](Matrix::Vector4(source))) => Ok(Box::new([<$lib AssignVV>] { sink: sink.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $feature, feature = "VectorD"))]
          (Value::[<Matrix $value_kind>](Matrix::DVector(sink)), Value::[<Matrix $value_kind>](Matrix::DVector(source))) => Ok(Box::new([<$lib AssignVV>] { sink: sink.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $feature, feature = "RowVector2"))]
          (Value::[<Matrix $value_kind>](Matrix::RowVector2(sink)), Value::[<Matrix $value_kind>](Matrix::RowVector2(source))) => Ok(Box::new([<$lib AssignVV>] { sink: sink.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $feature, feature = "RowVector3"))]
          (Value::[<Matrix $value_kind>](Matrix::RowVector3(sink)), Value::[<Matrix $value_kind>](Matrix::RowVector3(source))) => Ok(Box::new([<$lib AssignVV>] { sink: sink.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $feature, feature = "RowVector4"))]
          (Value::[<Matrix $value_kind>](Matrix::RowVector4(sink)), Value::[<Matrix $value_kind>](Matrix::RowVector4(source))) => Ok(Box::new([<$lib AssignVV>] { sink: sink.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $feature, feature = "RowVectorD"))]
          (Value::[<Matrix $value_kind>](Matrix::RowDVector(sink)), Value::[<Matrix $value_kind>](Matrix::RowDVector(source))) => Ok(Box::new([<$lib AssignVV>] { sink: sink.clone(), source: source.clone(), _marker: PhantomData::default() })),
        )+
        x => Err(MechError {file: file!().to_string(),tokens: vec![],msg: format!("Unhandled args {:?}", x),id: line!(),kind: MechErrorKind::UnhandledFunctionArgumentKind,}),
      }
    }
  };
}