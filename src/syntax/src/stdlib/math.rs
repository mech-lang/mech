#[macro_use]
use crate::stdlib::*;

// ----------------------------------------------------------------------------
// Math Library
// ----------------------------------------------------------------------------

macro_rules! addto_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
      unsafe { (*$lhs).add_to(&*$rhs,&mut *$out) }
    };}
  
  macro_rules! subto_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
      unsafe { (*$lhs).sub_to(&*$rhs,&mut *$out) }
    };}
  
  macro_rules! component_mul_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
      unsafe { *$out = (*$lhs).component_mul(&*$rhs); }
    };}
  
  macro_rules! component_div_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
      unsafe { *$out = (*$lhs).component_div(&*$rhs); }
    };}
  
  macro_rules! add_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
      unsafe { *$out = *$lhs + *$rhs; }
    };}
  
  macro_rules! sub_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
      unsafe { *$out = *$lhs - *$rhs; }
    };}
  
  macro_rules! mul_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
      unsafe { *$out = *$lhs * *$rhs; }
    };}
  
  macro_rules! div_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
      unsafe { *$out = *$lhs / *$rhs; }
    };}
  
  macro_rules! add_scalar_lhs_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
      unsafe { *$out = (*$lhs).add_scalar(*$rhs); }
    };}
  
  macro_rules! add_scalar_rhs_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
      unsafe { *$out = (*$rhs).add_scalar(*$lhs); }
    };}
  
  macro_rules! sub_scalar_lhs_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
      unsafe {
        for i in 0..(*$lhs).len() {
          (*$out)[i] = (*$lhs)[i] - (*$rhs);
        }}};}
  
  macro_rules! sub_scalar_rhs_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
      unsafe {
        for i in 0..(*$rhs).len() {
          (*$out)[i] = (*$lhs) - (*$rhs)[i];
        }}};}
  
  macro_rules! sub_scalar_lhs_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
      unsafe {
        for i in 0..(*$lhs).len() {
          (*$out)[i] = (*$lhs)[i] - (*$rhs);
        }}};}
  
  macro_rules! sub_scalar_rhs_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
      unsafe {
        for i in 0..(*$rhs).len() {
          (*$out)[i] = (*$lhs) - (*$rhs)[i];
        }}};}
  
  macro_rules! sub_scalar_lhs_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
      unsafe {
        for i in 0..(*$lhs).len() {
          (*$out)[i] = (*$lhs)[i] - (*$rhs);
        }}};}
  
  macro_rules! mul_scalar_lhs_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
      unsafe { *$out = (*$lhs).clone() * *$rhs; }
    };}
  
  macro_rules! mul_scalar_rhs_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
      unsafe { *$out = (*$rhs).clone() * *$lhs;}};}
  
  macro_rules! div_scalar_lhs_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
      unsafe {
        for i in 0..(*$lhs).len() {
          (*$out)[i] = (*$lhs)[i] / (*$rhs);
        }}};}
  
  macro_rules! div_scalar_rhs_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
      unsafe {
        for i in 0..(*$rhs).len() {
          (*$out)[i] = (*$lhs) / (*$rhs)[i];
        }}};}
  
  macro_rules! neg_op {
    ($arg:expr, $out:expr) => {
      unsafe { *$out = -*$arg; }
    };}

  macro_rules! neg_vec_op {
    ($arg:expr, $out:expr) => {
      unsafe { *$out = (*$arg).clone().neg(); }
    };}
  
  
  // Cos ------------------------------------------------------------------------
  
  use libm::cos;
  
  #[derive(Debug)]
  pub struct MathCosScalar {
    val: Ref<F64>,
    out: Ref<F64>,
  }
  
  impl MechFunction for MathCosScalar {
    fn solve(&self) {
      let val_ptr = self.val.as_ptr();
      let out_ptr = self.out.as_ptr();
      unsafe{(*out_ptr).0 = cos((*val_ptr).0);}
    }
    fn out(&self) -> Value { Value::F64(self.out.clone()) }
    fn to_string(&self) -> String { format!("{:#?}", self)}
  }
  
  pub struct MathCos {}
  
  impl NativeFunctionCompiler for MathCos {
    fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
      if arguments.len() != 1 {
        return Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
      }
      match &arguments[0] {
        Value::F64(val) =>
          Ok(Box::new(MathCosScalar{val: val.clone(), out: new_ref(F64(0.0))})),
        Value::MutableReference(val) => match &*val.borrow() {
          Value::F64(val) => Ok(Box::new(MathCosScalar{val: val.clone(), out: new_ref(F64(0.0))})),
          x => Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind})  
        }
        x =>Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind})
      }
    }
  }
  
  // Sin ------------------------------------------------------------------------
  
  use libm::sin;
  
  #[derive(Debug)]
  pub struct MathSinScalar {
    val: Ref<F64>,
    out: Ref<F64>,
  }
  
  impl MechFunction for MathSinScalar {
    fn solve(&self) {
      let val_ptr = self.val.as_ptr();
      let out_ptr = self.out.as_ptr();
      unsafe{(*out_ptr).0 = sin((*val_ptr).0);}
    }
    fn out(&self) -> Value { Value::F64(self.out.clone()) }
    fn to_string(&self) -> String { format!("{:#?}", self)}
  }
  
  pub struct MathSin {}
  
  impl NativeFunctionCompiler for MathSin {
    fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
      if arguments.len() != 1 {
        return Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
      }
      match &arguments[0] {
        Value::F64(val) =>
          Ok(Box::new(MathSinScalar{val: val.clone(), out: new_ref(F64(0.0))})),
        Value::MutableReference(val) => match &*val.borrow() {
          Value::F64(val) => Ok(Box::new(MathSinScalar{val: val.clone(), out: new_ref(F64(0.0))})),
          x => Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind})  
        }
        x =>Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind})
      }
    }
  }
  
  // Add ------------------------------------------------------------------------
  
  impl_binop!(AddScalar, T,T,T, add_op);
  impl_binop!(AddSM2x3, T, Matrix2x3<T>, Matrix2x3<T>,add_scalar_rhs_op);
  impl_binop!(AddSM2, T, Matrix2<T>, Matrix2<T>,add_scalar_rhs_op);
  impl_binop!(AddSM3, T, Matrix3<T>, Matrix3<T>,add_scalar_rhs_op);
  impl_binop!(AddSR2, T, RowVector2<T>, RowVector2<T>,add_scalar_rhs_op);
  impl_binop!(AddSR3, T, RowVector3<T>, RowVector3<T>,add_scalar_rhs_op);
  impl_binop!(AddSR4, T, RowVector4<T>, RowVector4<T>,add_scalar_rhs_op);
  impl_binop!(AddSRD, T, RowDVector<T>, RowDVector<T>,add_scalar_rhs_op);
  impl_binop!(AddSVD, T, DVector<T>, DVector<T>,add_scalar_rhs_op);
  impl_binop!(AddSMD, T, DMatrix<T>, DMatrix<T>,add_scalar_rhs_op);
  impl_binop!(AddM2x3S, Matrix2x3<T>, T, Matrix2x3<T>,add_scalar_lhs_op);
  impl_binop!(AddM2S, Matrix2<T>, T, Matrix2<T>,add_scalar_lhs_op);
  impl_binop!(AddM3S, Matrix3<T>, T, Matrix3<T>,add_scalar_lhs_op);
  impl_binop!(AddR2S, RowVector2<T>, T, RowVector2<T>,add_scalar_lhs_op);
  impl_binop!(AddR3S, RowVector3<T>, T, RowVector3<T>,add_scalar_lhs_op);
  impl_binop!(AddR4S, RowVector4<T>, T, RowVector4<T>,add_scalar_lhs_op);
  impl_binop!(AddRDS, RowDVector<T>, T, RowDVector<T>,add_scalar_lhs_op);
  impl_binop!(AddVDS, DVector<T>, T, DVector<T>,add_scalar_lhs_op);
  impl_binop!(AddMDS, DMatrix<T>, T, DMatrix<T>,add_scalar_lhs_op);
  impl_binop!(AddM2M2, Matrix2<T>,Matrix2<T>,Matrix2<T>, add_op);
  impl_binop!(AddM3M3, Matrix3<T>,Matrix3<T>,Matrix3<T>, add_op);
  impl_binop!(AddM2x3M2x3, Matrix2x3<T>,Matrix2x3<T>,Matrix2x3<T>, add_op);
  impl_binop!(AddR2R2, RowVector2<T>, RowVector2<T>, RowVector2<T>, add_op);
  impl_binop!(AddR3R3, RowVector3<T>, RowVector3<T>, RowVector3<T>, add_op);
  impl_binop!(AddR4R4, RowVector4<T>, RowVector4<T>, RowVector4<T>, add_op);
  impl_binop!(AddRDRD, RowDVector<T>, RowDVector<T>, RowDVector<T>, addto_op);
  impl_binop!(AddVDVD, DVector<T>,DVector<T>,DVector<T>, addto_op);
  impl_binop!(AddMDMD, DMatrix<T>,DMatrix<T>,DMatrix<T>, addto_op);
  
  fn generate_add_fxn(lhs_value: Value, rhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
    generate_binop_match_arms!(
      Add,
      (lhs_value, rhs_value),
      I8,   I8   => MatrixI8,   i8,   i8::zero();
      I16,  I16  => MatrixI16,  i16,  i16::zero();
      I32,  I32  => MatrixI32,  i32,  i32::zero();
      I64,  I64  => MatrixI64,  i64,  i64::zero();
      I128, I128 => MatrixI128, i128, i128::zero();
      U8,   U8   => MatrixU8,   u8,   u8::zero();
      U16,  U16  => MatrixU16,  u16,  u16::zero();
      U32,  U32  => MatrixU32,  u32,  u32::zero();
      U64,  U64  => MatrixU64,  u64,  u64::zero();
      U128, U128 => MatrixU128, u128, u128::zero();
      F32,  F32  => MatrixF32,  F32,  F32::zero();
      F64,  F64  => MatrixF64,  F64,  F64::zero();
    )
  }
  
  pub struct MathAdd {}
  
  impl NativeFunctionCompiler for MathAdd {
    fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
      if arguments.len() != 2 {
        return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
      }
      let lhs_value = arguments[0].clone();
      let rhs_value = arguments[1].clone();
      match generate_add_fxn(lhs_value.clone(), rhs_value.clone()) {
        Ok(fxn) => Ok(fxn),
        Err(_) => {
          match (lhs_value,rhs_value) {
            (Value::MutableReference(lhs),Value::MutableReference(rhs)) => {generate_add_fxn(lhs.borrow().clone(), rhs.borrow().clone())}
            (lhs_value,Value::MutableReference(rhs)) => { generate_add_fxn(lhs_value.clone(), rhs.borrow().clone())}
            (Value::MutableReference(lhs),rhs_value) => { generate_add_fxn(lhs.borrow().clone(), rhs_value.clone()) }
            x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
          }
        }
      }
    }
  }
  
  // Sub ------------------------------------------------------------------------
  
  impl_binop!(SubScalar, T,T,T, sub_op);
  impl_binop!(SubSM2x3, T, Matrix2x3<T>, Matrix2x3<T>,sub_scalar_rhs_op);
  impl_binop!(SubSM2, T, Matrix2<T>, Matrix2<T>,sub_scalar_rhs_op);
  impl_binop!(SubSM3, T, Matrix3<T>, Matrix3<T>,sub_scalar_rhs_op);
  impl_binop!(SubSR2, T, RowVector2<T>, RowVector2<T>,sub_scalar_rhs_op);
  impl_binop!(SubSR3, T, RowVector3<T>, RowVector3<T>,sub_scalar_rhs_op);
  impl_binop!(SubSR4, T, RowVector4<T>, RowVector4<T>,sub_scalar_rhs_op);
  impl_binop!(SubSRD, T, RowDVector<T>, RowDVector<T>,sub_scalar_rhs_op);
  impl_binop!(SubSVD, T, DVector<T>, DVector<T>,sub_scalar_rhs_op);
  impl_binop!(SubSMD, T, DMatrix<T>, DMatrix<T>,sub_scalar_rhs_op);
  impl_binop!(SubM2x3S, Matrix2x3<T>, T, Matrix2x3<T>,sub_scalar_lhs_op);
  impl_binop!(SubM2S, Matrix2<T>, T, Matrix2<T>,sub_scalar_lhs_op);
  impl_binop!(SubM3S, Matrix3<T>, T, Matrix3<T>,sub_scalar_lhs_op);
  impl_binop!(SubR2S, RowVector2<T>, T, RowVector2<T>,sub_scalar_lhs_op);
  impl_binop!(SubR3S, RowVector3<T>, T, RowVector3<T>,sub_scalar_lhs_op);
  impl_binop!(SubR4S, RowVector4<T>, T, RowVector4<T>,sub_scalar_lhs_op);
  impl_binop!(SubRDS, RowDVector<T>, T, RowDVector<T>,sub_scalar_lhs_op);
  impl_binop!(SubVDS, DVector<T>, T, DVector<T>,sub_scalar_lhs_op);
  impl_binop!(SubMDS, DMatrix<T>, T, DMatrix<T>,sub_scalar_lhs_op);
  impl_binop!(SubM2M2, Matrix2<T>,Matrix2<T>,Matrix2<T>, sub_op);
  impl_binop!(SubM3M3, Matrix3<T>,Matrix3<T>,Matrix3<T>, sub_op);
  impl_binop!(SubM2x3M2x3, Matrix2x3<T>,Matrix2x3<T>,Matrix2x3<T>, sub_op);
  impl_binop!(SubR2R2, RowVector2<T>, RowVector2<T>, RowVector2<T>, sub_op);
  impl_binop!(SubR3R3, RowVector3<T>, RowVector3<T>, RowVector3<T>, sub_op);
  impl_binop!(SubR4R4, RowVector4<T>, RowVector4<T>, RowVector4<T>, sub_op);
  impl_binop!(SubRDRD, RowDVector<T>, RowDVector<T>, RowDVector<T>, subto_op);
  impl_binop!(SubVDVD, DVector<T>,DVector<T>,DVector<T>, subto_op);
  impl_binop!(SubMDMD, DMatrix<T>,DMatrix<T>,DMatrix<T>, subto_op);
  
  fn generate_sub_fxn(lhs_value: Value, rhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
    generate_binop_match_arms!(
      Sub,
      (lhs_value, rhs_value),
      I8,   I8   => MatrixI8,   i8,   i8::zero();
      I16,  I16  => MatrixI16,  i16,  i16::zero();
      I32,  I32  => MatrixI32,  i32,  i32::zero();
      I64,  I64  => MatrixI64,  i64,  i64::zero();
      I128, I128 => MatrixI128, i128, i128::zero();
      U8,   U8   => MatrixU8,   u8,   u8::zero();
      U16,  U16  => MatrixU16,  u16,  u16::zero();
      U32,  U32  => MatrixU32,  u32,  u32::zero();
      U64,  U64  => MatrixU64,  u64,  u64::zero();
      U128, U128 => MatrixU128, u128, u128::zero();
      F32,  F32  => MatrixF32,  F32,  F32::zero();
      F64,  F64  => MatrixF64,  F64,  F64::zero();
    )
  }
  
  pub struct MathSub {}
  
  impl NativeFunctionCompiler for MathSub {
    fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
      if arguments.len() != 2 {
        return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
      }
      let lhs_value = arguments[0].clone();
      let rhs_value = arguments[1].clone();
      match generate_sub_fxn(lhs_value.clone(), rhs_value.clone()) {
        Ok(fxn) => Ok(fxn),
        Err(_) => {
          match (lhs_value,rhs_value) {
            (Value::MutableReference(lhs),Value::MutableReference(rhs)) => {generate_sub_fxn(lhs.borrow().clone(), rhs.borrow().clone())}
            (lhs_value,Value::MutableReference(rhs)) => { generate_sub_fxn(lhs_value.clone(), rhs.borrow().clone())}
            (Value::MutableReference(lhs),rhs_value) => { generate_sub_fxn(lhs.borrow().clone(), rhs_value.clone()) }
            x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
          }
        }
      }
    }
  }
  
  // Mul ------------------------------------------------------------------------
  
  impl_binop!(MulScalar, T,T,T, mul_op);
  impl_binop!(MulSM2x3, T, Matrix2x3<T>, Matrix2x3<T>,mul_scalar_rhs_op);
  impl_binop!(MulSM2, T, Matrix2<T>, Matrix2<T>,mul_scalar_rhs_op);
  impl_binop!(MulSM3, T, Matrix3<T>, Matrix3<T>,mul_scalar_rhs_op);
  impl_binop!(MulSR2, T, RowVector2<T>, RowVector2<T>,mul_scalar_rhs_op);
  impl_binop!(MulSR3, T, RowVector3<T>, RowVector3<T>,mul_scalar_rhs_op);
  impl_binop!(MulSR4, T, RowVector4<T>, RowVector4<T>,mul_scalar_rhs_op);
  impl_binop!(MulSRD, T, RowDVector<T>, RowDVector<T>,mul_scalar_rhs_op);
  impl_binop!(MulSVD, T, DVector<T>, DVector<T>,mul_scalar_rhs_op);
  impl_binop!(MulSMD, T, DMatrix<T>, DMatrix<T>,mul_scalar_rhs_op);
  impl_binop!(MulM2x3S, Matrix2x3<T>, T, Matrix2x3<T>,mul_scalar_lhs_op);
  impl_binop!(MulM2S, Matrix2<T>, T, Matrix2<T>,mul_scalar_lhs_op);
  impl_binop!(MulM3S, Matrix3<T>, T, Matrix3<T>,mul_scalar_lhs_op);
  impl_binop!(MulR2S, RowVector2<T>, T, RowVector2<T>,mul_scalar_lhs_op);
  impl_binop!(MulR3S, RowVector3<T>, T, RowVector3<T>,mul_scalar_lhs_op);
  impl_binop!(MulR4S, RowVector4<T>, T, RowVector4<T>,mul_scalar_lhs_op);
  impl_binop!(MulRDS, RowDVector<T>, T, RowDVector<T>,mul_scalar_lhs_op);
  impl_binop!(MulVDS, DVector<T>, T, DVector<T>,mul_scalar_lhs_op);
  impl_binop!(MulMDS, DMatrix<T>, T, DMatrix<T>,mul_scalar_lhs_op);
  impl_binop!(MulM2x3M2x3, Matrix2x3<T>,Matrix2x3<T>,Matrix2x3<T>, component_mul_op);
  impl_binop!(MulM2M2, Matrix2<T>,Matrix2<T>,Matrix2<T>, component_mul_op);
  impl_binop!(MulM3M3, Matrix3<T>,Matrix3<T>,Matrix3<T>, component_mul_op);
  impl_binop!(MulR2R2, RowVector2<T>,RowVector2<T>,RowVector2<T>, component_mul_op);
  impl_binop!(MulR3R3, RowVector3<T>,RowVector3<T>,RowVector3<T>, component_mul_op);
  impl_binop!(MulR4R4, RowVector4<T>,RowVector4<T>,RowVector4<T>, component_mul_op);
  impl_binop!(MulRDRD, RowDVector<T>,RowDVector<T>,RowDVector<T>, component_mul_op);
  impl_binop!(MulVDVD, DVector<T>,DVector<T>,DVector<T>, component_mul_op);
  impl_binop!(MulMDMD, DMatrix<T>,DMatrix<T>,DMatrix<T>, component_mul_op);
  
  fn generate_mul_fxn(lhs_value: Value, rhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
    generate_binop_match_arms!(
      Mul,
      (lhs_value, rhs_value),
      I8,   I8   => MatrixI8,   i8,   i8::zero();
      I16,  I16  => MatrixI16,  i16,  i16::zero();
      I32,  I32  => MatrixI32,  i32,  i32::zero();
      I64,  I64  => MatrixI64,  i64,  i64::zero();
      I128, I128 => MatrixI128, i128, i128::zero();
      U8,   U8   => MatrixU8,   u8,   u8::zero();
      U16,  U16  => MatrixU16,  u16,  u16::zero();
      U32,  U32  => MatrixU32,  u32,  u32::zero();
      U64,  U64  => MatrixU64,  u64,  u64::zero();
      U128, U128 => MatrixU128, u128, u128::zero();
      F32,  F32  => MatrixF32,  F32,  F32::zero();
      F64,  F64  => MatrixF64,  F64,  F64::zero();
    )
  }
  
  pub struct MathMul {}
  
  impl NativeFunctionCompiler for MathMul {
    fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
      if arguments.len() != 2 {
        return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
      }
      let lhs_value = arguments[0].clone();
      let rhs_value = arguments[1].clone();
      match generate_mul_fxn(lhs_value.clone(), rhs_value.clone()) {
        Ok(fxn) => Ok(fxn),
        Err(_) => {
          match (lhs_value,rhs_value) {
            (Value::MutableReference(lhs),Value::MutableReference(rhs)) => {generate_mul_fxn(lhs.borrow().clone(), rhs.borrow().clone())}
            (lhs_value,Value::MutableReference(rhs)) => { generate_mul_fxn(lhs_value.clone(), rhs.borrow().clone())}
            (Value::MutableReference(lhs),rhs_value) => { generate_mul_fxn(lhs.borrow().clone(), rhs_value.clone()) }
            x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
          }
        }
      }
    }
  }
  
  // Div ------------------------------------------------------------------------
  
  impl_binop!(DivScalar, T, T, T, div_op);
  impl_binop!(DivSM2x3, T, Matrix2x3<T>, Matrix2x3<T>,div_scalar_rhs_op);
  impl_binop!(DivSM2, T, Matrix2<T>, Matrix2<T>,div_scalar_rhs_op);
  impl_binop!(DivSM3, T, Matrix3<T>, Matrix3<T>,div_scalar_rhs_op);
  impl_binop!(DivSR2, T, RowVector2<T>, RowVector2<T>,div_scalar_rhs_op);
  impl_binop!(DivSR3, T, RowVector3<T>, RowVector3<T>,div_scalar_rhs_op);
  impl_binop!(DivSR4, T, RowVector4<T>, RowVector4<T>,div_scalar_rhs_op);
  impl_binop!(DivSRD, T, RowDVector<T>, RowDVector<T>,div_scalar_rhs_op);
  impl_binop!(DivSVD, T, DVector<T>, DVector<T>,div_scalar_rhs_op);
  impl_binop!(DivSMD, T, DMatrix<T>, DMatrix<T>,div_scalar_rhs_op);
  impl_binop!(DivM2x3S, Matrix2x3<T>, T, Matrix2x3<T>,div_scalar_lhs_op);
  impl_binop!(DivM2S, Matrix2<T>, T, Matrix2<T>,div_scalar_lhs_op);
  impl_binop!(DivM3S, Matrix3<T>, T, Matrix3<T>,div_scalar_lhs_op);
  impl_binop!(DivR2S, RowVector2<T>, T, RowVector2<T>,div_scalar_lhs_op);
  impl_binop!(DivR3S, RowVector3<T>, T, RowVector3<T>,div_scalar_lhs_op);
  impl_binop!(DivR4S, RowVector4<T>, T, RowVector4<T>,div_scalar_lhs_op);
  impl_binop!(DivRDS, RowDVector<T>, T, RowDVector<T>,div_scalar_lhs_op);
  impl_binop!(DivVDS, DVector<T>, T, DVector<T>,add_scalar_lhs_op);
  impl_binop!(DivMDS, DMatrix<T>, T, DMatrix<T>,add_scalar_lhs_op);
  impl_binop!(DivM2x3M2x3, Matrix2x3<T>,Matrix2x3<T>,Matrix2x3<T>,component_div_op);
  impl_binop!(DivM2M2, Matrix2<T>,Matrix2<T>,Matrix2<T>,component_div_op);
  impl_binop!(DivM3M3, Matrix3<T>,Matrix3<T>,Matrix3<T>,component_div_op);
  impl_binop!(DivR2R2, RowVector2<T>,RowVector2<T>,RowVector2<T>,component_div_op);
  impl_binop!(DivR3R3, RowVector3<T>,RowVector3<T>,RowVector3<T>,component_div_op);
  impl_binop!(DivR4R4, RowVector4<T>,RowVector4<T>,RowVector4<T>,component_div_op);
  impl_binop!(DivRDRD, RowDVector<T>,RowDVector<T>,RowDVector<T>,component_div_op);
  impl_binop!(DivVDVD, DVector<T>,DVector<T>,DVector<T>,component_div_op);
  impl_binop!(DivMDMD, DMatrix<T>,DMatrix<T>,DMatrix<T>,component_div_op);
  
  fn generate_div_fxn(lhs_value: Value, rhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
    generate_binop_match_arms!(
      Div,
      (lhs_value, rhs_value),
      I8,   I8   => MatrixI8,   i8,   i8::zero();
      I16,  I16  => MatrixI16,  i16,  i16::zero();
      I32,  I32  => MatrixI32,  i32,  i32::zero();
      I64,  I64  => MatrixI64,  i64,  i64::zero();
      I128, I128 => MatrixI128, i128, i128::zero();
      U8,   U8   => MatrixU8,   u8,   u8::zero();
      U16,  U16  => MatrixU16,  u16,  u16::zero();
      U32,  U32  => MatrixU32,  u32,  u32::zero();
      U64,  U64  => MatrixU64,  u64,  u64::zero();
      U128, U128 => MatrixU128, u128, u128::zero();
      F32,  F32  => MatrixF32,  F32,  F32::zero();
      F64,  F64  => MatrixF64,  F64,  F64::zero();
    )
  }
  
  pub struct MathDiv {}
  
  impl NativeFunctionCompiler for MathDiv {
    fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
      if arguments.len() != 2 {
        return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
      }
      let lhs_value = arguments[0].clone();
      let rhs_value = arguments[1].clone();
      match generate_div_fxn(lhs_value.clone(), rhs_value.clone()) {
        Ok(fxn) => Ok(fxn),
        Err(_) => {
          match (lhs_value,rhs_value) {
            (Value::MutableReference(lhs),Value::MutableReference(rhs)) => {generate_div_fxn(lhs.borrow().clone(), rhs.borrow().clone())}
            (lhs_value,Value::MutableReference(rhs)) => { generate_div_fxn(lhs_value.clone(), rhs.borrow().clone())}
            (Value::MutableReference(lhs),rhs_value) => { generate_div_fxn(lhs.borrow().clone(), rhs_value.clone()) }
            x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
          }
        }
      }
    }
  }
  
  // Exp ------------------------------------------------------------------------
  
  #[derive(Debug)] 
  struct ExpScalar {
    lhs: Ref<i64>,
    rhs: Ref<i64>,
    out: Ref<i64>,
  }
  
  impl MechFunction for ExpScalar {
    fn solve(&self) {
      let lhs_ptr = self.lhs.as_ptr();
      let rhs_ptr = self.rhs.as_ptr();
      let out_ptr = self.out.as_ptr();
      unsafe {*out_ptr = (*lhs_ptr).pow(*rhs_ptr as u32);}
    }
    fn out(&self) -> Value {
      Value::I64(self.out.clone())
    }
    fn to_string(&self) -> String { format!("{:?}", self) }
  }
  
  pub struct MathExp {}
  
  impl NativeFunctionCompiler for MathExp {
    fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
      if arguments.len() != 2 {
        return Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
      }
      match (arguments[0].clone(), arguments[1].clone()) {
        (Value::I64(lhs), Value::I64(rhs)) =>
          Ok(Box::new(ExpScalar{lhs, rhs, out: new_ref(0)})),
        x => 
          Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind})
      }
    }
  }
  
  // Negate ---------------------------------------------------------------------
  
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
      fn to_string(&self) -> String { format!("{:?}", self) }
    }};}
  
  impl_neg_op!(NegateScalar, T, neg_op);
  impl_neg_op!(NegateM2, Matrix2<T>,neg_op);
  impl_neg_op!(NegateM3, Matrix3<T>,neg_op);
  impl_neg_op!(NegateM2x3, Matrix2x3<T>,neg_op);
  impl_neg_op!(NegateR2, RowVector2<T>,neg_op);
  impl_neg_op!(NegateR3, RowVector3<T>,neg_op);
  impl_neg_op!(NegateR4, RowVector4<T>,neg_op);     
  impl_neg_op!(NegateRD, RowDVector<T>,neg_vec_op);
  impl_neg_op!(NegateVD, DVector<T>,neg_vec_op);
  impl_neg_op!(NegateMD, DMatrix<T>,neg_vec_op);
    
  fn generate_neg_fxn(lhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
    generate_urnop_match_arms!(
      Negate,
      (lhs_value),
      I8 => MatrixI8, i8, i8::zero();
      I16 => MatrixI16, i16, i16::zero();
      I32 => MatrixI32, i32, i32::zero();
      I64 => MatrixI64, i64, i64::zero();
      I128 => MatrixI128, i128, i128::zero();
      F32 => MatrixF32, F32, F32::zero();
      F64 => MatrixF64, F64, F64::zero();
    )
  }
  
  pub struct MathNegate {}
  
  impl NativeFunctionCompiler for MathNegate {
    fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
      if arguments.len() != 1 {
        return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
      }
      let input = arguments[0].clone();
      match generate_neg_fxn(input.clone()) {
        Ok(fxn) => Ok(fxn),
        Err(_) => {
          match (input) {
            (Value::MutableReference(input)) => {generate_neg_fxn(input.borrow().clone())}
            x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
          }
        }
      }
    }
  }