#[macro_use]
use crate::stdlib::*;

// Negate ---------------------------------------------------------------------
  
macro_rules! neg_op {
    ($arg:expr, $out:expr) => {
      unsafe { *$out = -*$arg; }
    };}
  
macro_rules! neg_vec_op {
($arg:expr, $out:expr) => {
    unsafe { *$out = (*$arg).clone().neg(); }
    };}

macro_rules! impl_neg_op {
($struct_name:ident, $out_type:ty, $op:ident) => {
    #[derive(Debug)]
    struct $struct_name<T> {
    arg: Ref<$out_type>,
    out: Ref<$out_type>,
    }
    impl<T> MechFunction for $struct_name<T>
    where
    T: Copy + Debug + Clone + Sync + Send + Neg + ClosedNeg + PartialEq + 'static,
    Ref<$out_type>: ToValue
    {
    fn solve(&self) {
        let arg_ptr = self.arg.as_ptr();
        let out_ptr = self.out.as_ptr();
        $op!(arg_ptr,out_ptr);
    }
    fn out(&self) -> Value { self.out.to_value() }
    fn to_string(&self) -> String { format!("{:#?}", self) }
    }};}

impl_neg_op!(NegateS, T, neg_op);
#[cfg(feature = "Matrix1")]
impl_neg_op!(NegateM1, Matrix1<T>,neg_op);
#[cfg(feature = "Matrix2")]
impl_neg_op!(NegateM2, Matrix2<T>,neg_op);
#[cfg(feature = "Matrix3")]
impl_neg_op!(NegateM3, Matrix3<T>,neg_op);
#[cfg(feature = "Matrix4")]
impl_neg_op!(NegateM4, Matrix4<T>,neg_op);
#[cfg(feature = "Matrix2x3")]
impl_neg_op!(NegateM2x3, Matrix2x3<T>,neg_op);
#[cfg(feature = "Matrix3x2")]
impl_neg_op!(NegateM3x2, Matrix3x2<T>,neg_op);
#[cfg(feature = "MatrixD")]
impl_neg_op!(NegateMD, DMatrix<T>,neg_vec_op);
#[cfg(feature = "RowVector2")]
impl_neg_op!(NegateR2, RowVector2<T>,neg_op);
#[cfg(feature = "RowVector3")]
impl_neg_op!(NegateR3, RowVector3<T>,neg_op);
#[cfg(feature = "RowVector4")]
impl_neg_op!(NegateR4, RowVector4<T>,neg_op);     
#[cfg(feature = "RowVectorD")]
impl_neg_op!(NegateRD, RowDVector<T>,neg_vec_op);
#[cfg(feature = "Vector2")]
impl_neg_op!(NegateV2, Vector2<T>,neg_op);
#[cfg(feature = "Vector3")]
impl_neg_op!(NegateV3, Vector3<T>,neg_op);
#[cfg(feature = "Vector4")]
impl_neg_op!(NegateV4, Vector4<T>,neg_op);     
#[cfg(feature = "VectorD")]
impl_neg_op!(NegateVD, DVector<T>,neg_vec_op);

fn impl_neg_fxn(lhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
impl_urnop_match_arms!(
    Negate,
    (lhs_value),
    I8 => MatrixI8, i8, i8::zero(), "I8";
    I16 => MatrixI16, i16, i16::zero(), "I16";
    I32 => MatrixI32, i32, i32::zero(), "I32";
    I64 => MatrixI64, i64, i64::zero(), "I64";
    I128 => MatrixI128, i128, i128::zero(), "I128";
    F32 => MatrixF32, F32, F32::zero(), "F32";
    F64 => MatrixF64, F64, F64::zero(), "F64";
)
}

impl_mech_urnop_fxn!(MathNegate,impl_neg_fxn);