use crate::*;
use mech_core::*;
use na::{Vector3, DVector, Vector2, Vector4, RowDVector, Matrix1, Matrix3, Matrix4, RowVector3, RowVector4, RowVector2, DMatrix, Rotation3, Matrix2x3, Matrix3x2, Matrix6, Matrix2};
use paste::paste;
use mech_core::matrix::Matrix;
use std::ops::Neg;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::Not;

// Not ------------------------------------------------------------------------


#[derive(Debug)]
struct NotS<T> {
  pub arg: Ref<T>,
  pub out: Ref<T>,
  pub _marker: PhantomData<T>,
}

impl<T> MechFunction for NotS<T>
where
  T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static + Not<Output = T>,
  Ref<T>: ToValue,
{
  fn solve(&self) {
    let arg_ptr = self.arg.as_ptr();
    let out_ptr = self.out.as_mut_ptr();
    unsafe { *out_ptr = !*arg_ptr; }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    todo!();
  }
}

#[derive(Debug)]
pub struct NotV<T, MatA> {
  pub arg: Ref<MatA>,
  pub out: Ref<MatA>,
  pub _marker: PhantomData<T>,
}

impl<T, MatA> MechFunction for NotV<T, MatA>
where
  Ref<MatA>: ToValue,
  T: Debug + Clone + Sync + Send + 'static + Not<Output = T>,
  for<'a> &'a MatA: IntoIterator<Item = &'a T>,
  for<'a> &'a mut MatA: IntoIterator<Item = &'a mut T>,
  MatA: Debug,
{
  fn solve(&self) {
    unsafe {
      let sink_ptr = self.out.as_mut_ptr();
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

fn impl_not_fxn(arg_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_urnop_match_arms!(
    Not,
    (arg_value),
    Bool, bool, "bool";
  )
}

impl_mech_urnop_fxn!(LogicNot,impl_not_fxn);