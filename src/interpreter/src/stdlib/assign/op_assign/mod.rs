#[macro_use]
use crate::stdlib::*;

use std::fmt::Debug;
use std::ops::DivAssign;
use std::marker::PhantomData;
use nalgebra::{
  base::{Matrix as naMatrix, Storage, StorageMut},
  Dim, Scalar,
};

pub mod add_assign;
//pub mod sub_assign;
pub mod div_assign;

pub use self::add_assign::*;
//pub use self::sub_assign::*;
pub use self::div_assign::*;

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