use crate::*;
use mech_core::*;

// Not ------------------------------------------------------------------------

macro_rules! not_op {
    ($arg:expr, $out:expr) => {
      unsafe {*$out = !*$arg;}
      };}
  
  macro_rules! not_vec_op {
    ($arg:expr, $out:expr) => {
      unsafe {
        for i in 0..(*$arg).len() {
          (&mut (*$out))[i] = !(&(*$arg))[i];
        }}};}
  
  impl_logic_urnop!(NotS, bool, bool, not_op);
  #[cfg(feature = "Matrix1")]
  impl_logic_urnop!(NotM1, Matrix1<bool>, Matrix1<bool>, not_vec_op);
  #[cfg(feature = "Matrix2")]
  impl_logic_urnop!(NotM2, Matrix2<bool>, Matrix2<bool>, not_vec_op);
  #[cfg(feature = "Matrix3")]
  impl_logic_urnop!(NotM3, Matrix3<bool>, Matrix3<bool>, not_vec_op);
  #[cfg(feature = "Matrix4")]
  impl_logic_urnop!(NotM4, Matrix4<bool>, Matrix4<bool>, not_vec_op);
  #[cfg(feature = "Matrix2x3")]
  impl_logic_urnop!(NotM2x3, Matrix2x3<bool>, Matrix2x3<bool>, not_vec_op);
  #[cfg(feature = "Matrix3x2")]
  impl_logic_urnop!(NotM3x2, Matrix3x2<bool>, Matrix3x2<bool>, not_vec_op);
  #[cfg(feature = "MatrixD")]
  impl_logic_urnop!(NotMD, DMatrix<bool>, DMatrix<bool>, not_vec_op);
  #[cfg(feature = "RowVector2")]
  impl_logic_urnop!(NotR2, RowVector2<bool>, RowVector2<bool>, not_vec_op);
  #[cfg(feature = "RowVector3")]
  impl_logic_urnop!(NotR3, RowVector3<bool>, RowVector3<bool>, not_vec_op);
  #[cfg(feature = "RowVector4")]
  impl_logic_urnop!(NotR4, RowVector4<bool>, RowVector4<bool>, not_vec_op);
  #[cfg(feature = "RowVectorD")]
  impl_logic_urnop!(NotRD, RowDVector<bool>, RowDVector<bool>, not_vec_op);
  #[cfg(feature = "Vector2")]
  impl_logic_urnop!(NotV2, Vector2<bool>, Vector2<bool>, not_vec_op);
  #[cfg(feature = "Vector3")]
  impl_logic_urnop!(NotV3, Vector3<bool>, Vector3<bool>, not_vec_op);
  #[cfg(feature = "Vector4")]
  impl_logic_urnop!(NotV4, Vector4<bool>, Vector4<bool>, not_vec_op);
  #[cfg(feature = "VectorD")]
  impl_logic_urnop!(NotVD, DVector<bool>, DVector<bool>, not_vec_op);
  
  fn impl_not_fxn(arg_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
    impl_urnop_match_arms!(
      Not,
      (arg_value),
      Bool => MatrixBool, bool, false, "Bool";
    )
  }
  
  impl_mech_urnop_fxn!(LogicNot,impl_not_fxn);