use crate::*;
use mech_core::*;
use na::{Vector3, DVector, Vector2, Vector4, RowDVector, Matrix1, Matrix3, Matrix4, RowVector3, RowVector4, RowVector2, DMatrix, Rotation3, Matrix2x3, Matrix3x2, Matrix6, Matrix2};
use paste::paste;
use mech_core::matrix::Matrix;
use std::ops::Neg;
use std::fmt::Debug;
use std::marker::PhantomData;

// Not ------------------------------------------------------------------------

use std::ops::Not;

#[derive(Debug)]
pub struct NotV<T, MatA> {
  pub arg: Ref<MatA>,
  pub out: Ref<MatA>,
  pub _marker: PhantomData<T>,
}

impl<T, MatA> MechFunction for NotV<T, MatA>
where
  Ref<MatA>: ToValue,
  T: Debug + Clone + Sync + Send + 'static + std::ops::Not<Output = T>,
  for<'a> &'a MatA: IntoIterator<Item = &'a T>,
  for<'a> &'a mut MatA: IntoIterator<Item = &'a mut T>,
  MatA: Debug,
{
  fn solve(&self) {
    unsafe {
      let sink_ptr = self.out.as_ptr();
      let source_ptr = self.arg.as_ptr();
      let sink_ref: &mut MatA = &mut *sink_ptr;
      let source_ref: &MatA = &*source_ptr;
      for (dst, src) in sink_ref.into_iter().zip(source_ref.into_iter()) {
        *dst = !src.clone();
      }
    }
  }
  fn out(&self) -> Value {self.out.to_value()}
  fn to_string(&self) -> String {format!("{:#?}", self)}
}

#[derive(Debug)]
struct NotS<O> {
  pub arg: Ref<O>,
  pub out: Ref<O>,
  pub _marker: PhantomData<O>,
}

impl<O> MechFunction for NotS<O>
where
  O: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<O>: ToValue,
{
  fn solve(&self) {
    let arg_ptr = self.arg.as_ptr();
    let out_ptr = self.out.as_ptr();
    unsafe { *out_ptr != *arg_ptr; }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}


fn impl_not_fxn(arg_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_urnop_match_arms!(
    Not,
    (arg_value),
    Bool, bool, "Bool";
  )
}

impl_mech_urnop_fxn!(LogicNot,impl_not_fxn);