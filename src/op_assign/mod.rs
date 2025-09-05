#[macro_use]
use crate::*;

use std::fmt::Debug;
use std::marker::PhantomData;

#[cfg(feature = "matrix")]
use nalgebra::{
  base::{Matrix as naMatrix, Storage, StorageMut},
  Dim, Scalar,
};

#[cfg(feature = "add_assign")]
pub mod add_assign;
#[cfg(feature = "sub_assign")]
pub mod sub_assign;
#[cfg(feature = "div_assign")]
pub mod div_assign;
#[cfg(feature = "mul_assign")]
pub mod mul_assign;

#[cfg(feature = "add_assign")]
pub use self::add_assign::*;
#[cfg(feature = "sub_assign")]
pub use self::sub_assign::*;
#[cfg(feature = "div_assign")]
pub use self::div_assign::*;
#[cfg(feature = "mul_assign")]
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
    impl<T, R1, C1, S1, IxVec> MechFunctionImpl for $struct_name<T, naMatrix<T, R1, C1, S1>, IxVec>
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
          let sink_ptr = &mut *self.sink.as_mut_ptr();
          let source_ptr = &*self.source.as_ptr();
          let ix_ptr = &(*self.ixes.as_ptr()).as_ref();
          $op!(source_ptr,ix_ptr,sink_ptr);
        }
      }
      fn out(&self) -> Value {self.sink.to_value()}
      fn to_string(&self) -> String {format!("{:#?}", self)}
    }
    #[cfg(feature = "compiler")]
    impl<T, R1, C1, S1, IxVec> MechFunctionCompiler for $struct_name<T, naMatrix<T, R1, C1, S1>, IxVec> 
    where
      T: CompileConst + ConstElem + AsValueKind,
      IxVec: CompileConst + ConstElem,
      naMatrix<T, R1, C1, S1>: CompileConst + ConstElem,
    {
      fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
        compile_binop!(self.sink, self.source, self.ixes, ctx, FeatureFlag::Builtin(FeatureKind::OpAssign), T);
      }
    }};}

#[macro_export]
macro_rules! impl_op_assign_range_fxn_v {
  ($struct_name:ident, $op:ident, $ix:ty) => {
    #[cfg(feature = "matrix")]
    #[derive(Debug)]
    pub struct $struct_name<T, MatA, MatB, IxVec> {
      pub source: Ref<MatB>,
      pub ixes: Ref<IxVec>,
      pub sink: Ref<MatA>,
      pub _marker: PhantomData<T>,
    }

    impl<T, R1, C1, S1, R2, C2, S2, IxVec>
      MechFunctionImpl for $struct_name<T, naMatrix<T, R1, C1, S1>, naMatrix<T, R2, C2, S2>, IxVec>
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
          let sink_ptr = &mut *self.sink.as_mut_ptr();
          let source_ptr = &*self.source.as_ptr();
          let ix_ptr = &(*self.ixes.as_ptr()).as_ref();
          $op!(source_ptr,ix_ptr,sink_ptr);
        }
      }
      fn out(&self) -> Value {self.sink.to_value()}
      fn to_string(&self) -> String {format!("{:#?}", self)}
    }
    #[cfg(feature = "compiler")]
    impl<T, R1, C1, S1, R2, C2, S2, IxVec> MechFunctionCompiler for $struct_name<T, naMatrix<T, R1, C1, S1>, naMatrix<T, R2, C2, S2>, IxVec> 
    where
      T: CompileConst + ConstElem + AsValueKind,
      IxVec: CompileConst + ConstElem,
      naMatrix<T, R1, C1, S1>: CompileConst + ConstElem,
      naMatrix<T, R2, C2, S2>: CompileConst + ConstElem,
    {
      fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
        compile_binop!(self.sink, self.source, self.ixes, ctx, FeatureFlag::Builtin(FeatureKind::OpAssign), T);
      }
    }};}

#[macro_export]
macro_rules! impl_op_assign_value_match_arms {
  ($lib:ident, $arg:expr,$($value_kind:ident, $feature:tt);+ $(;)?) => {
    paste::paste! {
      match $arg {
        $(
          #[cfg(feature = $feature)]
          (Value::$value_kind(sink), Value::$value_kind(source)) => Ok(Box::new([<$lib AssignSS>] { sink: sink.clone(), source: source.clone() })),
          #[cfg(all(feature = $feature, feature = "matrix1"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix1(sink)), Value::[<Matrix $value_kind>](Matrix::Matrix1(source))) => Ok(Box::new([<$lib AssignVV>] { sink: sink.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $feature, feature = "matrix2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2(sink)), Value::[<Matrix $value_kind>](Matrix::Matrix2(source))) => Ok(Box::new([<$lib AssignVV>] { sink: sink.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $feature, feature = "matrix2x3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2x3(sink)), Value::[<Matrix $value_kind>](Matrix::Matrix2x3(source))) => Ok(Box::new([<$lib AssignVV>] { sink: sink.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $feature, feature = "matrix3x2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3x2(sink)), Value::[<Matrix $value_kind>](Matrix::Matrix3x2(source))) => Ok(Box::new([<$lib AssignVV>] { sink: sink.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $feature, feature = "matrix3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3(sink)), Value::[<Matrix $value_kind>](Matrix::Matrix3(source))) => Ok(Box::new([<$lib AssignVV>] { sink: sink.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $feature, feature = "matrix4"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix4(sink)), Value::[<Matrix $value_kind>](Matrix::Matrix4(source))) => Ok(Box::new([<$lib AssignVV>] { sink: sink.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $feature, feature = "matrixd"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)), Value::[<Matrix $value_kind>](Matrix::DMatrix(source))) => Ok(Box::new([<$lib AssignVV>] { sink: sink.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $feature, feature = "vector2"))]
          (Value::[<Matrix $value_kind>](Matrix::Vector2(sink)), Value::[<Matrix $value_kind>](Matrix::Vector2(source))) => Ok(Box::new([<$lib AssignVV>] { sink: sink.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $feature, feature = "vector3"))]
          (Value::[<Matrix $value_kind>](Matrix::Vector3(sink)), Value::[<Matrix $value_kind>](Matrix::Vector3(source))) => Ok(Box::new([<$lib AssignVV>] { sink: sink.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $feature, feature = "vector4"))]
          (Value::[<Matrix $value_kind>](Matrix::Vector4(sink)), Value::[<Matrix $value_kind>](Matrix::Vector4(source))) => Ok(Box::new([<$lib AssignVV>] { sink: sink.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $feature, feature = "vectord"))]
          (Value::[<Matrix $value_kind>](Matrix::DVector(sink)), Value::[<Matrix $value_kind>](Matrix::DVector(source))) => Ok(Box::new([<$lib AssignVV>] { sink: sink.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $feature, feature = "row_vector2"))]
          (Value::[<Matrix $value_kind>](Matrix::RowVector2(sink)), Value::[<Matrix $value_kind>](Matrix::RowVector2(source))) => Ok(Box::new([<$lib AssignVV>] { sink: sink.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $feature, feature = "row_vector3"))]
          (Value::[<Matrix $value_kind>](Matrix::RowVector3(sink)), Value::[<Matrix $value_kind>](Matrix::RowVector3(source))) => Ok(Box::new([<$lib AssignVV>] { sink: sink.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $feature, feature = "row_vector4"))]
          (Value::[<Matrix $value_kind>](Matrix::RowVector4(sink)), Value::[<Matrix $value_kind>](Matrix::RowVector4(source))) => Ok(Box::new([<$lib AssignVV>] { sink: sink.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $feature, feature = "row_vectord"))]
          (Value::[<Matrix $value_kind>](Matrix::RowDVector(sink)), Value::[<Matrix $value_kind>](Matrix::RowDVector(source))) => Ok(Box::new([<$lib AssignVV>] { sink: sink.clone(), source: source.clone(), _marker: PhantomData::default() })),
        )+
        x => Err(MechError {file: file!().to_string(),tokens: vec![],msg: format!("Unhandled args {:?}", x),id: line!(),kind: MechErrorKind::UnhandledFunctionArgumentKind,}),
      }
    }
  };
}

#[macro_export]
macro_rules! impl_assign_scalar_scalar {
  ($op_name:tt, $op_fn:tt) => {
    paste::paste! {
      #[derive(Debug)]
      struct [<$op_name AssignSS>]<T> {
        sink: Ref<T>,
        source: Ref<T>,
      }

      impl<T> MechFunctionImpl for [<$op_name AssignSS>]<T>
      where
        T: Debug + Clone + Sync + Send + 'static +
           $op_name<Output = T> + [<$op_name Assign>] +
           PartialEq + PartialOrd,
        Ref<T>: ToValue
      {
        fn solve(&self) {
          let sink_ptr = self.sink.as_mut_ptr();
          let source_ptr = self.source.as_ptr();
          unsafe {
            *sink_ptr $op_fn (*source_ptr).clone();
          }
        }
        fn out(&self) -> Value { self.sink.to_value() }
        fn to_string(&self) -> String { format!("{:#?}", self) }
      }
      #[cfg(feature = "compiler")]
      impl<T> MechFunctionCompiler for [<$op_name AssignSS>]<T> 
      where
        T: CompileConst + ConstElem + AsValueKind,
      {
        fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
          compile_unop!(self.sink, self.source, ctx, FeatureFlag::Builtin(FeatureKind::Assign) );
        }
      }
    }
  };
}

#[macro_export]
macro_rules! impl_assign_vector_vector {
  ($op_name:tt, $op_fn:tt) => {
    paste::paste! {
      #[derive(Debug)]
      pub struct [<$op_name AssignVV>]<T, MatA, MatB> {
        pub sink: Ref<MatA>,
        pub source: Ref<MatB>,
        _marker: PhantomData<T>,
      }

      impl<T, MatA, MatB> MechFunctionImpl for [<$op_name AssignVV>]<T, MatA, MatB>
      where
        Ref<MatA>: ToValue,
        T: Debug + Clone + Sync + Send + 'static + [<$op_name Assign>],
        for<'a> &'a MatA: IntoIterator<Item = &'a T>,
        for<'a> &'a mut MatA: IntoIterator<Item = &'a mut T>,
        for<'a> &'a MatB: IntoIterator<Item = &'a T>,
        MatA: Debug,
        MatB: Debug,
      {
        fn solve(&self) {
          unsafe {
            let sink_ptr = self.sink.as_mut_ptr();
            let source_ptr = self.source.as_ptr();
            let sink_ref: &mut MatA = &mut *sink_ptr;
            let source_ref: &MatB = &*source_ptr;
            for (dst, src) in (&mut *sink_ref).into_iter().zip((&*source_ref).into_iter()) {
              *dst $op_fn src.clone();
            }
          }
        }
        fn out(&self) -> Value {self.sink.to_value()}
        fn to_string(&self) -> String {format!("{:#?}", self)}
      }
      #[cfg(feature = "compiler")]
      impl<T, MatA, MatB> MechFunctionCompiler for [<$op_name AssignVV>]<T, MatA, MatB> 
      where
        T: CompileConst + ConstElem,
        MatA: CompileConst + ConstElem,
        MatB: CompileConst + ConstElem,
      {
        fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
          compile_unop!(self.sink, self.source, ctx, FeatureFlag::Builtin(FeatureKind::OpAssign) );
        }
      }
    }
  };
}