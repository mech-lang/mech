#[macro_use]
use crate::stdlib::*;

// ----------------------------------------------------------------------------
// Compare Library
// ----------------------------------------------------------------------------

macro_rules! lt_scalar_lhs_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
      unsafe {
        for i in 0..(*$lhs).len() {
          (*$out)[i] = (*$lhs)[i] < (*$rhs);
        }}};}
  
  macro_rules! lt_scalar_rhs_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
      unsafe {
        for i in 0..(*$rhs).len() {
          (*$out)[i] = (*$lhs) < (*$rhs)[i];
        }}};}
  
  
  macro_rules! lt_vec_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
      unsafe {
        for i in 0..(*$lhs).len() {
          (*$out)[i] = (*$lhs)[i] < (*$rhs)[i];
        }}};}
  
  macro_rules! lt_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
      unsafe {
        (*$out) = (*$lhs) < (*$rhs);
      }};}
  
  macro_rules! gt_scalar_lhs_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
      unsafe {
        for i in 0..(*$lhs).len() {
          (*$out)[i] = (*$lhs)[i] > (*$rhs);
        }}};}
  
  macro_rules! gt_scalar_rhs_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
      unsafe {
        for i in 0..(*$rhs).len() {
          (*$out)[i] = (*$lhs) > (*$rhs)[i];
        }}};}
  
  macro_rules! gt_vec_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
      unsafe {
        for i in 0..(*$lhs).len() {
          (*$out)[i] = (*$lhs)[i] > (*$rhs)[i];
        }}};}
  
  macro_rules! gt_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
      unsafe {
        (*$out) = (*$lhs) > (*$rhs);
      }};}
  
  macro_rules! neq_scalar_lhs_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
      unsafe {
        for i in 0..(*$lhs).len() {
          (*$out)[i] = (*$lhs)[i] != (*$rhs);
        }}};}
  
  macro_rules! neq_scalar_rhs_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
      unsafe {
        for i in 0..(*$rhs).len() {
          (*$out)[i] = (*$lhs) != (*$rhs)[i];
        }}};}
  
  
  macro_rules! neq_vec_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
      unsafe {
        for i in 0..(*$lhs).len() {
          (*$out)[i] = (*$lhs)[i] != (*$rhs)[i];
        }}};}
  
  macro_rules! neq_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
      unsafe {
        (*$out) = (*$lhs) != (*$rhs);
      }};}
  
  macro_rules! eq_scalar_lhs_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
      unsafe {
        for i in 0..(*$lhs).len() {
          (*$out)[i] = (*$lhs)[i] == (*$rhs);
        }}};}
  
  macro_rules! eq_scalar_rhs_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
      unsafe {
        for i in 0..(*$rhs).len() {
          (*$out)[i] = (*$lhs) == (*$rhs)[i];
        }}};}
  
  
  macro_rules! eq_vec_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
      unsafe {
        for i in 0..(*$lhs).len() {
          (*$out)[i] = (*$lhs)[i] == (*$rhs)[i];
        }}};}
  
  macro_rules! eq_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
      unsafe {
        (*$out) = (*$lhs) == (*$rhs);
      }};}
  
  
  macro_rules! lte_scalar_lhs_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
      unsafe {
        for i in 0..(*$lhs).len() {
          (*$out)[i] = (*$lhs)[i] <= (*$rhs);
        }}};}
  
  macro_rules! lte_scalar_rhs_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
      unsafe {
        for i in 0..(*$rhs).len() {
          (*$out)[i] = (*$lhs) <= (*$rhs)[i];
        }}};}
  
  
  macro_rules! lte_vec_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
      unsafe {
        for i in 0..(*$lhs).len() {
          (*$out)[i] = (*$lhs)[i] <= (*$rhs)[i];
        }}};}
  
  macro_rules! lte_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
      unsafe {
        (*$out) = (*$lhs) <= (*$rhs);
      }};}
  
  macro_rules! gte_scalar_lhs_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
      unsafe {
        for i in 0..(*$lhs).len() {
          (*$out)[i] = (*$lhs)[i] >= (*$rhs);
        }}};}
  
  macro_rules! gte_scalar_rhs_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
      unsafe {
        for i in 0..(*$rhs).len() {
          (*$out)[i] = (*$lhs) >= (*$rhs)[i];
        }}};}
  
  
  macro_rules! gte_vec_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
      unsafe {
        for i in 0..(*$lhs).len() {
          (*$out)[i] = (*$lhs)[i] >= (*$rhs)[i];
        }}};}
  
  macro_rules! gte_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
      unsafe {
        (*$out) = (*$lhs) >= (*$rhs);
      }};}
  
  // Greater Than ---------------------------------------------------------------
  
  impl_binop!(GTScalar, T, T, bool, gt_op);
  impl_binop!(GTSM2x3, T, Matrix2x3<T>, Matrix2x3<bool>,gt_scalar_rhs_op);
  impl_binop!(GTSM2, T, Matrix2<T>, Matrix2<bool>,gt_scalar_rhs_op);
  impl_binop!(GTSM3, T, Matrix3<T>, Matrix3<bool>,gt_scalar_rhs_op);
  impl_binop!(GTSR2, T, RowVector2<T>, RowVector2<bool>,gt_scalar_rhs_op);
  impl_binop!(GTSR3, T, RowVector3<T>, RowVector3<bool>,gt_scalar_rhs_op);
  impl_binop!(GTSR4, T, RowVector4<T>, RowVector4<bool>,gt_scalar_rhs_op);
  impl_binop!(GTSRD, T, RowDVector<T>, RowDVector<bool>,gt_scalar_rhs_op);
  impl_binop!(GTSVD, T, DVector<T>, DVector<bool>,gt_scalar_rhs_op);
  impl_binop!(GTSMD, T, DMatrix<T>, DMatrix<bool>,gt_scalar_rhs_op);
  impl_binop!(GTM2x3S, Matrix2x3<T>, T, Matrix2x3<bool>,gt_scalar_lhs_op);
  impl_binop!(GTM2S, Matrix2<T>, T, Matrix2<bool>,gt_scalar_lhs_op);
  impl_binop!(GTM3S, Matrix3<T>, T, Matrix3<bool>,gt_scalar_lhs_op);
  impl_binop!(GTR2S, RowVector2<T>, T, RowVector2<bool>,gt_scalar_lhs_op);
  impl_binop!(GTR3S, RowVector3<T>, T, RowVector3<bool>,gt_scalar_lhs_op);
  impl_binop!(GTR4S, RowVector4<T>, T, RowVector4<bool>,gt_scalar_lhs_op);
  impl_binop!(GTRDS, RowDVector<T>, T, RowDVector<bool>,gt_scalar_lhs_op);
  impl_binop!(GTVDS, DVector<T>, T, DVector<bool>,gt_scalar_lhs_op);
  impl_binop!(GTMDS, DMatrix<T>, T, DMatrix<bool>,gt_scalar_lhs_op);
  impl_binop!(GTM2x3M2x3, Matrix2x3<T>, Matrix2x3<T>, Matrix2x3<bool>, gt_vec_op);
  impl_binop!(GTM2M2, Matrix2<T>, Matrix2<T>, Matrix2<bool>, gt_vec_op);
  impl_binop!(GTM3M3, Matrix3<T>,Matrix3<T>, Matrix3<bool>, gt_vec_op);
  impl_binop!(GTR2R2, RowVector2<T>, RowVector2<T>, RowVector2<bool>, gt_vec_op);
  impl_binop!(GTR3R3, RowVector3<T>, RowVector3<T>, RowVector3<bool>, gt_vec_op);
  impl_binop!(GTR4R4, RowVector4<T>, RowVector4<T>, RowVector4<bool>, gt_vec_op);
  impl_binop!(GTRDRD, RowDVector<T>, RowDVector<T>, RowDVector<bool>, gt_vec_op);
  impl_binop!(GTVDVD, DVector<T>, DVector<T>, DVector<bool>, gt_vec_op);
  impl_binop!(GTMDMD, DMatrix<T>, DMatrix<T>, DMatrix<bool>, gt_vec_op);
  
  fn generate_gt_fxn(lhs_value: Value, rhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
    generate_binop_match_arms!(
      GT,
      (lhs_value, rhs_value),
      I8,   I8   => MatrixI8,   i8,   false;
      I16,  I16  => MatrixI16,  i16,  false;
      I32,  I32  => MatrixI32,  i32,  false;
      I64,  I64  => MatrixI64,  i64,  false;
      I128, I128 => MatrixI128, i128, false;
      U8,   U8   => MatrixU8,   u8,   false;
      U16,  U16  => MatrixU16,  u16,  false;
      U32,  U32  => MatrixU32,  u32,  false;
      U64,  U64  => MatrixU64,  u64,  false;
      U128, U128 => MatrixU128, u128, false;
      F32,  F32  => MatrixF32,  F32,  false;
      F64,  F64  => MatrixF64,  F64,  false;
    )
  }
  
  pub struct CompareGreaterThan {}
  
  impl NativeFunctionCompiler for CompareGreaterThan {
    fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
      if arguments.len() != 2 {
        return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
      }
      let lhs_value = arguments[0].clone();
      let rhs_value = arguments[1].clone();
      match generate_gt_fxn(lhs_value.clone(), rhs_value.clone()) {
        Ok(fxn) => Ok(fxn),
        Err(_) => {
          match (lhs_value,rhs_value) {
            (Value::MutableReference(lhs),Value::MutableReference(rhs)) => {generate_gt_fxn(lhs.borrow().clone(), rhs.borrow().clone())}
            (lhs_value,Value::MutableReference(rhs)) => { generate_gt_fxn(lhs_value.clone(), rhs.borrow().clone())}
            (Value::MutableReference(lhs),rhs_value) => { generate_gt_fxn(lhs.borrow().clone(), rhs_value.clone()) }
            x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
          }
        }
      }
    }
  }
  
  // Greater Than Equal ---------------------------------------------------------------
  
  impl_binop!(GTEScalar, T, T, bool, gte_op);
  impl_binop!(GTESM2x3, T, Matrix2x3<T>, Matrix2x3<bool>,gte_scalar_rhs_op);
  impl_binop!(GTESM2, T, Matrix2<T>, Matrix2<bool>,gte_scalar_rhs_op);
  impl_binop!(GTESM3, T, Matrix3<T>, Matrix3<bool>,gte_scalar_rhs_op);
  impl_binop!(GTESR2, T, RowVector2<T>, RowVector2<bool>,gte_scalar_rhs_op);
  impl_binop!(GTESR3, T, RowVector3<T>, RowVector3<bool>,gte_scalar_rhs_op);
  impl_binop!(GTESR4, T, RowVector4<T>, RowVector4<bool>,gte_scalar_rhs_op);
  impl_binop!(GTESRD, T, RowDVector<T>, RowDVector<bool>,gte_scalar_rhs_op);
  impl_binop!(GTESVD, T, DVector<T>, DVector<bool>,gte_scalar_rhs_op);
  impl_binop!(GTESMD, T, DMatrix<T>, DMatrix<bool>,gte_scalar_rhs_op);
  impl_binop!(GTEM2x3S, Matrix2x3<T>, T, Matrix2x3<bool>,gte_scalar_lhs_op);
  impl_binop!(GTEM2S, Matrix2<T>, T, Matrix2<bool>,gte_scalar_lhs_op);
  impl_binop!(GTEM3S, Matrix3<T>, T, Matrix3<bool>,gte_scalar_lhs_op);
  impl_binop!(GTER2S, RowVector2<T>, T, RowVector2<bool>,gte_scalar_lhs_op);
  impl_binop!(GTER3S, RowVector3<T>, T, RowVector3<bool>,gte_scalar_lhs_op);
  impl_binop!(GTER4S, RowVector4<T>, T, RowVector4<bool>,gte_scalar_lhs_op);
  impl_binop!(GTERDS, RowDVector<T>, T, RowDVector<bool>,gte_scalar_lhs_op);
  impl_binop!(GTEVDS, DVector<T>, T, DVector<bool>,gte_scalar_lhs_op);
  impl_binop!(GTEMDS, DMatrix<T>, T, DMatrix<bool>,gte_scalar_lhs_op);
  impl_binop!(GTEM2x3M2x3, Matrix2x3<T>, Matrix2x3<T>, Matrix2x3<bool>, gte_vec_op);
  impl_binop!(GTEM2M2, Matrix2<T>, Matrix2<T>, Matrix2<bool>, gte_vec_op);
  impl_binop!(GTEM3M3, Matrix3<T>,Matrix3<T>, Matrix3<bool>, gte_vec_op);
  impl_binop!(GTER2R2, RowVector2<T>, RowVector2<T>, RowVector2<bool>, gte_vec_op);
  impl_binop!(GTER3R3, RowVector3<T>, RowVector3<T>, RowVector3<bool>, gte_vec_op);
  impl_binop!(GTER4R4, RowVector4<T>, RowVector4<T>, RowVector4<bool>, gte_vec_op);
  impl_binop!(GTERDRD, RowDVector<T>, RowDVector<T>, RowDVector<bool>, gte_vec_op);
  impl_binop!(GTEVDVD, DVector<T>, DVector<T>, DVector<bool>, gte_vec_op);
  impl_binop!(GTEMDMD, DMatrix<T>, DMatrix<T>, DMatrix<bool>, gte_vec_op);
  
  fn generate_gte_fxn(lhs_value: Value, rhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
    generate_binop_match_arms!(
      GTE,
      (lhs_value, rhs_value),
      I8,   I8   => MatrixI8,   i8,   false;
      I16,  I16  => MatrixI16,  i16,  false;
      I32,  I32  => MatrixI32,  i32,  false;
      I64,  I64  => MatrixI64,  i64,  false;
      I128, I128 => MatrixI128, i128, false;
      U8,   U8   => MatrixU8,   u8,   false;
      U16,  U16  => MatrixU16,  u16,  false;
      U32,  U32  => MatrixU32,  u32,  false;
      U64,  U64  => MatrixU64,  u64,  false;
      U128, U128 => MatrixU128, u128, false;
      F32,  F32  => MatrixF32,  F32,  false;
      F64,  F64  => MatrixF64,  F64,  false;
    )
  }
  
  pub struct CompareGreaterThanEqual {}
  
  impl NativeFunctionCompiler for CompareGreaterThanEqual {
    fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
      if arguments.len() != 2 {
        return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
      }
      let lhs_value = arguments[0].clone();
      let rhs_value = arguments[1].clone();
      match generate_gte_fxn(lhs_value.clone(), rhs_value.clone()) {
        Ok(fxn) => Ok(fxn),
        Err(_) => {
          match (lhs_value,rhs_value) {
            (Value::MutableReference(lhs),Value::MutableReference(rhs)) => {generate_gte_fxn(lhs.borrow().clone(), rhs.borrow().clone())}
            (lhs_value,Value::MutableReference(rhs)) => { generate_gte_fxn(lhs_value.clone(), rhs.borrow().clone())}
            (Value::MutableReference(lhs),rhs_value) => { generate_gte_fxn(lhs.borrow().clone(), rhs_value.clone()) }
            x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
          }
        }
      }
    }
  }
  
  // Less Than Equal ---------------------------------------------------------------
  
  impl_bool_binop!(LTEScalar, T, T, bool, lte_op);
  impl_bool_binop!(LTESM2x3, T, Matrix2x3<T>, Matrix2x3<bool>,lte_scalar_rhs_op);
  impl_bool_binop!(LTESM2, T, Matrix2<T>, Matrix2<bool>,lte_scalar_rhs_op);
  impl_bool_binop!(LTESM3, T, Matrix3<T>, Matrix3<bool>,lte_scalar_rhs_op);
  impl_bool_binop!(LTESR2, T, RowVector2<T>, RowVector2<bool>,lte_scalar_rhs_op);
  impl_bool_binop!(LTESR3, T, RowVector3<T>, RowVector3<bool>,lte_scalar_rhs_op);
  impl_bool_binop!(LTESR4, T, RowVector4<T>, RowVector4<bool>,lte_scalar_rhs_op);
  impl_bool_binop!(LTESRD, T, RowDVector<T>, RowDVector<bool>,lte_scalar_rhs_op);
  impl_bool_binop!(LTESVD, T, DVector<T>, DVector<bool>,lte_scalar_rhs_op);
  impl_bool_binop!(LTESMD, T, DMatrix<T>, DMatrix<bool>,lte_scalar_rhs_op);
  impl_bool_binop!(LTEM2x3S, Matrix2x3<T>, T, Matrix2x3<bool>,lte_scalar_lhs_op);
  impl_bool_binop!(LTEM2S, Matrix2<T>, T, Matrix2<bool>,lte_scalar_lhs_op);
  impl_bool_binop!(LTEM3S, Matrix3<T>, T, Matrix3<bool>,lte_scalar_lhs_op);
  impl_bool_binop!(LTER2S, RowVector2<T>, T, RowVector2<bool>,lte_scalar_lhs_op);
  impl_bool_binop!(LTER3S, RowVector3<T>, T, RowVector3<bool>,lte_scalar_lhs_op);
  impl_bool_binop!(LTER4S, RowVector4<T>, T, RowVector4<bool>,lte_scalar_lhs_op);
  impl_bool_binop!(LTERDS, RowDVector<T>, T, RowDVector<bool>,lte_scalar_lhs_op);
  impl_bool_binop!(LTEVDS, DVector<T>, T, DVector<bool>,lte_scalar_lhs_op);
  impl_bool_binop!(LTEMDS, DMatrix<T>, T, DMatrix<bool>,lte_scalar_lhs_op);
  impl_bool_binop!(LTEM2x3M2x3, Matrix2x3<T>, Matrix2x3<T>, Matrix2x3<bool>, lte_vec_op);
  impl_bool_binop!(LTEM2M2, Matrix2<T>, Matrix2<T>, Matrix2<bool>, lte_vec_op);
  impl_bool_binop!(LTEM3M3, Matrix3<T>,Matrix3<T>, Matrix3<bool>, lte_vec_op);
  impl_bool_binop!(LTER2R2, RowVector2<T>, RowVector2<T>, RowVector2<bool>, lte_vec_op);
  impl_bool_binop!(LTER3R3, RowVector3<T>, RowVector3<T>, RowVector3<bool>, lte_vec_op);
  impl_bool_binop!(LTER4R4, RowVector4<T>, RowVector4<T>, RowVector4<bool>, lte_vec_op);
  impl_bool_binop!(LTERDRD, RowDVector<T>, RowDVector<T>, RowDVector<bool>, lte_vec_op);
  impl_bool_binop!(LTEVDVD, DVector<T>, DVector<T>, DVector<bool>, lte_vec_op);
  impl_bool_binop!(LTEMDMD, DMatrix<T>, DMatrix<T>, DMatrix<bool>, lte_vec_op);
  
  fn generate_lte_fxn(lhs_value: Value, rhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
    generate_binop_match_arms!(
      LTE,
      (lhs_value, rhs_value),
      Bool, Bool => MatrixBool, bool, false;
      I8,   I8   => MatrixI8,   i8,   false;
      I16,  I16  => MatrixI16,  i16,  false;
      I32,  I32  => MatrixI32,  i32,  false;
      I64,  I64  => MatrixI64,  i64,  false;
      I128, I128 => MatrixI128, i128, false;
      U8,   U8   => MatrixU8,   u8,   false;
      U16,  U16  => MatrixU16,  u16,  false;
      U32,  U32  => MatrixU32,  u32,  false;
      U64,  U64  => MatrixU64,  u64,  false;
      U128, U128 => MatrixU128, u128, false;
      F32,  F32  => MatrixF32,  F32,  false;
      F64,  F64  => MatrixF64,  F64,  false;
    )
  }
  
  pub struct CompareLessThanEqual {}
  
  impl NativeFunctionCompiler for CompareLessThanEqual {
    fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
      if arguments.len() != 2 {
        return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
      }
      let lhs_value = arguments[0].clone();
      let rhs_value = arguments[1].clone();
      match generate_lte_fxn(lhs_value.clone(), rhs_value.clone()) {
        Ok(fxn) => Ok(fxn),
        Err(_) => {
          match (lhs_value,rhs_value) {
            (Value::MutableReference(lhs),Value::MutableReference(rhs)) => {generate_lte_fxn(lhs.borrow().clone(), rhs.borrow().clone())}
            (lhs_value,Value::MutableReference(rhs)) => { generate_lte_fxn(lhs_value.clone(), rhs.borrow().clone())}
            (Value::MutableReference(lhs),rhs_value) => { generate_lte_fxn(lhs.borrow().clone(), rhs_value.clone()) }
            x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
          }
        }
      }
    }
  }
  
  // Equal ---------------------------------------------------------------
  
  impl_bool_binop!(EQScalar, T, T, bool, eq_op);
  impl_bool_binop!(EQSM2x3, T, Matrix2x3<T>, Matrix2x3<bool>,eq_scalar_rhs_op);
  impl_bool_binop!(EQSM2, T, Matrix2<T>, Matrix2<bool>,eq_scalar_rhs_op);
  impl_bool_binop!(EQSM3, T, Matrix3<T>, Matrix3<bool>,eq_scalar_rhs_op);
  impl_bool_binop!(EQSR2, T, RowVector2<T>, RowVector2<bool>,eq_scalar_rhs_op);
  impl_bool_binop!(EQSR3, T, RowVector3<T>, RowVector3<bool>,eq_scalar_rhs_op);
  impl_bool_binop!(EQSR4, T, RowVector4<T>, RowVector4<bool>,eq_scalar_rhs_op);
  impl_bool_binop!(EQSRD, T, RowDVector<T>, RowDVector<bool>,eq_scalar_rhs_op);
  impl_bool_binop!(EQSVD, T, DVector<T>, DVector<bool>,eq_scalar_rhs_op);
  impl_bool_binop!(EQSMD, T, DMatrix<T>, DMatrix<bool>,eq_scalar_rhs_op);
  impl_bool_binop!(EQM2x3S, Matrix2x3<T>, T, Matrix2x3<bool>,eq_scalar_lhs_op);
  impl_bool_binop!(EQM2S, Matrix2<T>, T, Matrix2<bool>,eq_scalar_lhs_op);
  impl_bool_binop!(EQM3S, Matrix3<T>, T, Matrix3<bool>,eq_scalar_lhs_op);
  impl_bool_binop!(EQR2S, RowVector2<T>, T, RowVector2<bool>,eq_scalar_lhs_op);
  impl_bool_binop!(EQR3S, RowVector3<T>, T, RowVector3<bool>,eq_scalar_lhs_op);
  impl_bool_binop!(EQR4S, RowVector4<T>, T, RowVector4<bool>,eq_scalar_lhs_op);
  impl_bool_binop!(EQRDS, RowDVector<T>, T, RowDVector<bool>,eq_scalar_lhs_op);
  impl_bool_binop!(EQVDS, DVector<T>, T, DVector<bool>,eq_scalar_lhs_op);
  impl_bool_binop!(EQMDS, DMatrix<T>, T, DMatrix<bool>,eq_scalar_lhs_op);
  impl_bool_binop!(EQM2x3M2x3, Matrix2x3<T>, Matrix2x3<T>, Matrix2x3<bool>, eq_vec_op);
  impl_bool_binop!(EQM2M2, Matrix2<T>, Matrix2<T>, Matrix2<bool>, eq_vec_op);
  impl_bool_binop!(EQM3M3, Matrix3<T>,Matrix3<T>, Matrix3<bool>, eq_vec_op);
  impl_bool_binop!(EQR2R2, RowVector2<T>, RowVector2<T>, RowVector2<bool>, eq_vec_op);
  impl_bool_binop!(EQR3R3, RowVector3<T>, RowVector3<T>, RowVector3<bool>, eq_vec_op);
  impl_bool_binop!(EQR4R4, RowVector4<T>, RowVector4<T>, RowVector4<bool>, eq_vec_op);
  impl_bool_binop!(EQRDRD, RowDVector<T>, RowDVector<T>, RowDVector<bool>, eq_vec_op);
  impl_bool_binop!(EQVDVD, DVector<T>, DVector<T>, DVector<bool>, eq_vec_op);
  impl_bool_binop!(EQMDMD, DMatrix<T>, DMatrix<T>, DMatrix<bool>, eq_vec_op);
  
  fn generate_eq_fxn(lhs_value: Value, rhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
    generate_binop_match_arms!(
      EQ,
      (lhs_value, rhs_value),
      Bool, Bool => MatrixBool, bool, false;
      I8,   I8   => MatrixI8,   i8,   false;
      I16,  I16  => MatrixI16,  i16,  false;
      I32,  I32  => MatrixI32,  i32,  false;
      I64,  I64  => MatrixI64,  i64,  false;
      I128, I128 => MatrixI128, i128, false;
      U8,   U8   => MatrixU8,   u8,   false;
      U16,  U16  => MatrixU16,  u16,  false;
      U32,  U32  => MatrixU32,  u32,  false;
      U64,  U64  => MatrixU64,  u64,  false;
      U128, U128 => MatrixU128, u128, false;
      F32,  F32  => MatrixF32,  F32,  false;
      F64,  F64  => MatrixF64,  F64,  false;
    )
  }
  
  pub struct CompareEqual {}
  
  impl NativeFunctionCompiler for CompareEqual {
    fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
      if arguments.len() != 2 {
        return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
      }
      let lhs_value = arguments[0].clone();
      let rhs_value = arguments[1].clone();
      match generate_eq_fxn(lhs_value.clone(), rhs_value.clone()) {
        Ok(fxn) => Ok(fxn),
        Err(_) => {
          match (lhs_value,rhs_value) {
            (Value::MutableReference(lhs),Value::MutableReference(rhs)) => {generate_eq_fxn(lhs.borrow().clone(), rhs.borrow().clone())}
            (lhs_value,Value::MutableReference(rhs)) => { generate_eq_fxn(lhs_value.clone(), rhs.borrow().clone())}
            (Value::MutableReference(lhs),rhs_value) => { generate_eq_fxn(lhs.borrow().clone(), rhs_value.clone()) }
            x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
          }
        }
      }
    }
  }
  
  // Not Equal ---------------------------------------------------------------
  
  impl_binop!(NEQScalar, T, T, bool, neq_op);
  impl_binop!(NEQSM2x3, T, Matrix2x3<T>, Matrix2x3<bool>,neq_scalar_rhs_op);
  impl_binop!(NEQSM2, T, Matrix2<T>, Matrix2<bool>,neq_scalar_rhs_op);
  impl_binop!(NEQSM3, T, Matrix3<T>, Matrix3<bool>,neq_scalar_rhs_op);
  impl_binop!(NEQSR2, T, RowVector2<T>, RowVector2<bool>,neq_scalar_rhs_op);
  impl_binop!(NEQSR3, T, RowVector3<T>, RowVector3<bool>,neq_scalar_rhs_op);
  impl_binop!(NEQSR4, T, RowVector4<T>, RowVector4<bool>,neq_scalar_rhs_op);
  impl_binop!(NEQSRD, T, RowDVector<T>, RowDVector<bool>,neq_scalar_rhs_op);
  impl_binop!(NEQSVD, T, DVector<T>, DVector<bool>,neq_scalar_rhs_op);
  impl_binop!(NEQSMD, T, DMatrix<T>, DMatrix<bool>,neq_scalar_rhs_op);
  impl_binop!(NEQM2x3S, Matrix2x3<T>, T, Matrix2x3<bool>,neq_scalar_lhs_op);
  impl_binop!(NEQM2S, Matrix2<T>, T, Matrix2<bool>,neq_scalar_lhs_op);
  impl_binop!(NEQM3S, Matrix3<T>, T, Matrix3<bool>,neq_scalar_lhs_op);
  impl_binop!(NEQR2S, RowVector2<T>, T, RowVector2<bool>,neq_scalar_lhs_op);
  impl_binop!(NEQR3S, RowVector3<T>, T, RowVector3<bool>,neq_scalar_lhs_op);
  impl_binop!(NEQR4S, RowVector4<T>, T, RowVector4<bool>,neq_scalar_lhs_op);
  impl_binop!(NEQRDS, RowDVector<T>, T, RowDVector<bool>,neq_scalar_lhs_op);
  impl_binop!(NEQVDS, DVector<T>, T, DVector<bool>,neq_scalar_lhs_op);
  impl_binop!(NEQMDS, DMatrix<T>, T, DMatrix<bool>,neq_scalar_lhs_op);
  impl_binop!(NEQM2x3M2x3, Matrix2x3<T>, Matrix2x3<T>, Matrix2x3<bool>, neq_vec_op);
  impl_binop!(NEQM2M2, Matrix2<T>, Matrix2<T>, Matrix2<bool>, neq_vec_op);
  impl_binop!(NEQM3M3, Matrix3<T>,Matrix3<T>, Matrix3<bool>, neq_vec_op);
  impl_binop!(NEQR2R2, RowVector2<T>, RowVector2<T>, RowVector2<bool>, neq_vec_op);
  impl_binop!(NEQR3R3, RowVector3<T>, RowVector3<T>, RowVector3<bool>, neq_vec_op);
  impl_binop!(NEQR4R4, RowVector4<T>, RowVector4<T>, RowVector4<bool>, neq_vec_op);
  impl_binop!(NEQRDRD, RowDVector<T>, RowDVector<T>, RowDVector<bool>, neq_vec_op);
  impl_binop!(NEQVDVD, DVector<T>, DVector<T>, DVector<bool>, neq_vec_op);
  impl_binop!(NEQMDMD, DMatrix<T>, DMatrix<T>, DMatrix<bool>, neq_vec_op);
  
  fn generate_neq_fxn(lhs_value: Value, rhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
    generate_binop_match_arms!(
      NEQ,
      (lhs_value, rhs_value),
      I8,   I8   => MatrixI8,   i8,   false;
      I16,  I16  => MatrixI16,  i16,  false;
      I32,  I32  => MatrixI32,  i32,  false;
      I64,  I64  => MatrixI64,  i64,  false;
      I128, I128 => MatrixI128, i128, false;
      U8,   U8   => MatrixU8,   u8,   false;
      U16,  U16  => MatrixU16,  u16,  false;
      U32,  U32  => MatrixU32,  u32,  false;
      U64,  U64  => MatrixU64,  u64,  false;
      U128, U128 => MatrixU128, u128, false;
      F32,  F32  => MatrixF32,  F32,  false;
      F64,  F64  => MatrixF64,  F64,  false;
    )
  }
  
  pub struct CompareNotEqual {}
  
  impl NativeFunctionCompiler for CompareNotEqual {
    fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
      if arguments.len() != 2 {
        return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
      }
      let lhs_value = arguments[0].clone();
      let rhs_value = arguments[1].clone();
      match generate_neq_fxn(lhs_value.clone(), rhs_value.clone()) {
        Ok(fxn) => Ok(fxn),
        Err(_) => {
          match (lhs_value,rhs_value) {
            (Value::MutableReference(lhs),Value::MutableReference(rhs)) => {generate_neq_fxn(lhs.borrow().clone(), rhs.borrow().clone())}
            (lhs_value,Value::MutableReference(rhs)) => { generate_neq_fxn(lhs_value.clone(), rhs.borrow().clone())}
            (Value::MutableReference(lhs),rhs_value) => { generate_neq_fxn(lhs.borrow().clone(), rhs_value.clone()) }
            x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
          }
        }
      }
    }
  }
  
  // Less Than ------------------------------------------------------------------
  
  impl_binop!(LTScalar, T, T, bool, lt_op);
  impl_binop!(LTSM2x3, T, Matrix2x3<T>, Matrix2x3<bool>,lt_scalar_rhs_op);
  impl_binop!(LTSM2, T, Matrix2<T>, Matrix2<bool>,lt_scalar_rhs_op);
  impl_binop!(LTSM3, T, Matrix3<T>, Matrix3<bool>,lt_scalar_rhs_op);
  impl_binop!(LTSR2, T, RowVector2<T>, RowVector2<bool>,lt_scalar_rhs_op);
  impl_binop!(LTSR3, T, RowVector3<T>, RowVector3<bool>,lt_scalar_rhs_op);
  impl_binop!(LTSR4, T, RowVector4<T>, RowVector4<bool>,lt_scalar_rhs_op);
  impl_binop!(LTSRD, T, RowDVector<T>, RowDVector<bool>,lt_scalar_rhs_op);
  impl_binop!(LTSVD, T, DVector<T>, DVector<bool>,lt_scalar_rhs_op);
  impl_binop!(LTSMD, T, DMatrix<T>, DMatrix<bool>,lt_scalar_rhs_op);
  impl_binop!(LTM2x3S, Matrix2x3<T>, T, Matrix2x3<bool>,lt_scalar_lhs_op);
  impl_binop!(LTM2S, Matrix2<T>, T, Matrix2<bool>,lt_scalar_lhs_op);
  impl_binop!(LTM3S, Matrix3<T>, T, Matrix3<bool>,lt_scalar_lhs_op);
  impl_binop!(LTR2S, RowVector2<T>, T, RowVector2<bool>,lt_scalar_lhs_op);
  impl_binop!(LTR3S, RowVector3<T>, T, RowVector3<bool>,lt_scalar_lhs_op);
  impl_binop!(LTR4S, RowVector4<T>, T, RowVector4<bool>,lt_scalar_lhs_op);
  impl_binop!(LTRDS, RowDVector<T>, T, RowDVector<bool>,lt_scalar_lhs_op);
  impl_binop!(LTVDS, DVector<T>, T, DVector<bool>,lt_scalar_lhs_op);
  impl_binop!(LTMDS, DMatrix<T>, T, DMatrix<bool>,lt_scalar_lhs_op);
  impl_binop!(LTM2x3M2x3, Matrix2x3<T>, Matrix2x3<T>, Matrix2x3<bool>, lt_vec_op);
  impl_binop!(LTM2M2, Matrix2<T>, Matrix2<T>, Matrix2<bool>, lt_vec_op);
  impl_binop!(LTM3M3, Matrix3<T>,Matrix3<T>, Matrix3<bool>, lt_vec_op);
  impl_binop!(LTR2R2, RowVector2<T>, RowVector2<T>, RowVector2<bool>, lt_vec_op);
  impl_binop!(LTR3R3, RowVector3<T>, RowVector3<T>, RowVector3<bool>, lt_vec_op);
  impl_binop!(LTR4R4, RowVector4<T>, RowVector4<T>, RowVector4<bool>, lt_vec_op);
  impl_binop!(LTRDRD, RowDVector<T>, RowDVector<T>, RowDVector<bool>, lt_vec_op);
  impl_binop!(LTVDVD, DVector<T>, DVector<T>, DVector<bool>, lt_vec_op);
  impl_binop!(LTMDMD, DMatrix<T>, DMatrix<T>, DMatrix<bool>, lt_vec_op);
  
  fn generate_lt_fxn(lhs_value: Value, rhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
    generate_binop_match_arms!(
      LT,
      (lhs_value, rhs_value),
      I8,   I8   => MatrixI8,   i8,   false;
      I16,  I16  => MatrixI16,  i16,  false;
      I32,  I32  => MatrixI32,  i32,  false;
      I64,  I64  => MatrixI64,  i64,  false;
      I128, I128 => MatrixI128, i128, false;
      U8,   U8   => MatrixU8,   u8,   false;
      U16,  U16  => MatrixU16,  u16,  false;
      U32,  U32  => MatrixU32,  u32,  false;
      U64,  U64  => MatrixU64,  u64,  false;
      U128, U128 => MatrixU128, u128, false;
      F32,  F32  => MatrixF32,  F32,  false;
      F64,  F64  => MatrixF64,  F64,  false;
    )
  }
  
  pub struct CompareLessThan {}
  
  impl NativeFunctionCompiler for CompareLessThan {
    fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
      if arguments.len() != 2 {
        return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
      }
      let lhs_value = arguments[0].clone();
      let rhs_value = arguments[1].clone();
      match generate_lt_fxn(lhs_value.clone(), rhs_value.clone()) {
        Ok(fxn) => Ok(fxn),
        Err(_) => {
          match (lhs_value,rhs_value) {
            (Value::MutableReference(lhs),Value::MutableReference(rhs)) => {generate_lt_fxn(lhs.borrow().clone(), rhs.borrow().clone())}
            (lhs_value,Value::MutableReference(rhs)) => { generate_lt_fxn(lhs_value.clone(), rhs.borrow().clone())}
            (Value::MutableReference(lhs),rhs_value) => { generate_lt_fxn(lhs.borrow().clone(), rhs_value.clone()) }
            x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
          }
        }
      }
    }
  }