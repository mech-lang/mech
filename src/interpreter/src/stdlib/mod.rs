use crate::*;
use mech_core::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

use paste::paste;
#[cfg(feature = "matrix")]
use na::{Vector3, DVector, Vector2, Vector4, RowDVector, Matrix1, Matrix3, Matrix4, RowVector3, RowVector4, RowVector2, DMatrix, Rotation3, Matrix2x3, Matrix3x2, Matrix6, Matrix2};
use std::ops::*;
use std::fmt::Debug;
use std::marker::PhantomData;
#[cfg(any(feature = "num-traits"))]
use num_traits::*;

#[cfg(feature = "access")]
pub mod access;
#[cfg(feature = "assign")]
pub mod assign;
#[cfg(feature = "convert")]
pub mod convert;
#[cfg(feature = "matrix_horzcat")]
pub mod horzcat;
#[cfg(feature = "matrix_vertcat")]
pub mod vertcat;

pub trait LosslessInto<T> {
  fn lossless_into(self) -> T;
}

pub trait LossyFrom<T> {
  fn lossy_from(value: T) -> Self;
}

#[macro_export]
macro_rules! impl_range_range_fxn_v {
  ($struct_name:ident, $op:ident, $ix1:ty, $ix2:ty) => {
    #[derive(Debug)]
    pub struct $struct_name<T, MatA, MatB, IxVec1, IxVec2> {
      pub source: Ref<MatB>,
      pub ixes: (Ref<IxVec1>,Ref<IxVec2>),
      pub sink: Ref<MatA>,
      pub _marker: PhantomData<T>,
    }
    impl<T, R1: 'static, C1: 'static, S1: 'static, R2: 'static, C2: 'static, S2: 'static, IxVec1: 'static, IxVec2: 'static> MechFunctionFactory for $struct_name<T, naMatrix<T, R1, C1, S1>, naMatrix<T, R2, C2, S2>, IxVec1, IxVec2>
    where
      Ref<naMatrix<T, R1, C1, S1>>: ToValue,
      Ref<naMatrix<T, R2, C2, S2>>: ToValue,
      T: Debug + Clone + Sync + Send + 'static +
        PartialEq + PartialOrd +
        CompileConst + ConstElem + AsValueKind,
      IxVec1: CompileConst + ConstElem + AsNaKind + Debug + AsRef<[$ix1]>,
      IxVec2: CompileConst + ConstElem + AsNaKind + Debug + AsRef<[$ix2]>,
      R1: Dim, C1: Dim, S1: StorageMut<T, R1, C1> + Clone + Debug,
      R2: Dim, C2: Dim, S2: Storage<T, R2, C2> + Clone + Debug,
      naMatrix<T, R1, C1, S1>: CompileConst + ConstElem + Debug + AsNaKind,
      naMatrix<T, R2, C2, S2>: CompileConst + ConstElem + Debug + AsNaKind,
    {
      fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        match args {
          FunctionArgs::Ternary(out, arg1, arg2, arg3) => {
            let source: Ref<naMatrix<T, R2, C2, S2>> = unsafe { arg1.as_unchecked() }.clone();
            let ix1: Ref<IxVec1> = unsafe { arg2.as_unchecked() }.clone();
            let ix2: Ref<IxVec2> = unsafe { arg3.as_unchecked() }.clone();
            let sink: Ref<naMatrix<T, R1, C1, S1>> = unsafe { out.as_unchecked() }.clone();
            Ok(Box::new(Self { sink, source, ixes: (ix1, ix2), _marker: PhantomData::default() }))
          },
          _ => Err(MechError{file: file!().to_string(), tokens: vec![], msg: format!("{} requires 3 arguments, got {:?}", stringify!($struct_name), args), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments})
        }
      }
    }
    impl<T, R1, C1, S1, R2, C2, S2, IxVec1, IxVec2>
      MechFunctionImpl for $struct_name<T, naMatrix<T, R1, C1, S1>, naMatrix<T, R2, C2, S2>, IxVec1, IxVec2>
    where
      Ref<naMatrix<T, R1, C1, S1>>: ToValue,
      T: Debug + Clone + Sync + Send + 'static +
         PartialEq + PartialOrd,
      IxVec1: AsRef<[$ix1]> + Debug,
      IxVec2: AsRef<[$ix2]> + Debug,
      R1: Dim, C1: Dim, S1: StorageMut<T, R1, C1> + Clone + Debug,
      R2: Dim, C2: Dim, S2: Storage<T, R2, C2> + Clone + Debug,
    {
      fn solve(&self) {
        unsafe {
          let sink = &mut *self.sink.as_mut_ptr();
          let source = &*self.source.as_ptr();
          let ix1 = (*self.ixes.0.as_ptr()).as_ref();
          let ix2 = (*self.ixes.1.as_ptr()).as_ref();
          $op!(sink, ix1, ix2, source);
        }
      }
      fn out(&self) -> Value {self.sink.to_value()}
      fn to_string(&self) -> String {format!("{:#?}", self)}
    }
    #[cfg(feature = "compiler")]
    impl<T, R1, C1, S1, R2, C2, S2, IxVec1, IxVec2> MechFunctionCompiler for $struct_name<T, naMatrix<T, R1, C1, S1>, naMatrix<T, R2, C2, S2>, IxVec1, IxVec2> 
    where
      T: CompileConst + ConstElem + AsValueKind,
      IxVec1: CompileConst + ConstElem + AsNaKind,
      IxVec2: CompileConst + ConstElem + AsNaKind,
      naMatrix<T, R1, C1, S1>: CompileConst + ConstElem + AsNaKind,
      naMatrix<T, R2, C2, S2>: CompileConst + ConstElem + AsNaKind,
    {
      fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
        let name = format!("{}<{}{}{}{}{}>", stringify!($struct_name), T::as_value_kind(), naMatrix::<T, R1, C1, S1>::as_na_kind(), naMatrix::<T, R2, C2, S2>::as_na_kind(), IxVec1::as_na_kind(), IxVec2::as_na_kind());
        compile_ternop!(name, self.sink, self.source, self.ixes.0, self.ixes.1, ctx, FeatureFlag::Builtin(FeatureKind::Assign) );  
      }
    }  
  };}

#[macro_export]
macro_rules! impl_all_fxn_v {
  ($struct_name:ident, $op:ident, $ix:ty) => {
    #[derive(Debug)]
    pub struct $struct_name<T, MatA, MatB, IxVec> {
      pub source: Ref<MatB>,
      pub ixes: Ref<IxVec>,
      pub sink: Ref<MatA>,
      pub _marker: PhantomData<T>,
    }
    impl<T, R1: 'static, C1: 'static, S1: 'static, R2: 'static, C2: 'static, S2: 'static, IxVec: 'static> MechFunctionFactory for $struct_name<T, naMatrix<T, R1, C1, S1>, naMatrix<T, R2, C2, S2>, IxVec>
    where
      Ref<naMatrix<T, R1, C1, S1>>: ToValue,
      Ref<naMatrix<T, R2, C2, S2>>: ToValue,
      T: Debug + Clone + Sync + Send + 'static +
        PartialEq + PartialOrd +
        CompileConst + ConstElem + AsValueKind,
      IxVec: CompileConst + ConstElem + AsNaKind + Debug + AsRef<[$ix]>,
      R1: Dim, C1: Dim, S1: StorageMut<T, R1, C1> + Clone + Debug,
      R2: Dim, C2: Dim, S2: Storage<T, R2, C2> + Clone + Debug,
      naMatrix<T, R1, C1, S1>: CompileConst + ConstElem + Debug + AsNaKind,
      naMatrix<T, R2, C2, S2>: CompileConst + ConstElem + Debug + AsNaKind,
    {
      fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        match args {
          FunctionArgs::Binary(out, arg1, arg2) => {
            let source: Ref<naMatrix<T, R2, C2, S2>> = unsafe { arg1.as_unchecked() }.clone();
            let ixes: Ref<IxVec> = unsafe { arg2.as_unchecked() }.clone();
            let sink: Ref<naMatrix<T, R1, C1, S1>> = unsafe { out.as_unchecked() }.clone();
            Ok(Box::new(Self { sink, source, ixes, _marker: PhantomData::default() }))
          },
          _ => Err(MechError{file: file!().to_string(), tokens: vec![], msg: format!("{} requires 3 arguments, got {:?}", stringify!($struct_name), args), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments})
        }
      }
    }
    impl<T, R1, C1, S1, R2, C2, S2, IxVec>
      MechFunctionImpl for $struct_name<T, naMatrix<T, R1, C1, S1>, naMatrix<T, R2, C2, S2>, IxVec>
    where
      Ref<naMatrix<T, R1, C1, S1>>: ToValue,
      T: Debug + Clone + Sync + Send + 'static +
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
      IxVec: CompileConst + ConstElem + AsNaKind,
      naMatrix<T, R1, C1, S1>: CompileConst + ConstElem + AsNaKind,
      naMatrix<T, R2, C2, S2>: CompileConst + ConstElem + AsNaKind,
    {
      fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
        let name = format!("{}<{}{}{}{}>", stringify!($struct_name), T::as_value_kind(), naMatrix::<T, R1, C1, S1>::as_na_kind(), naMatrix::<T, R2, C2, S2>::as_na_kind(), IxVec::as_na_kind());
        compile_binop!(name, self.sink, self.source, self.ixes, ctx, FeatureFlag::Builtin(FeatureKind::OpAssign));
      }
    }  
  };}