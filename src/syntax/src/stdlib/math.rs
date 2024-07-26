#[macro_use]
use crate::stdlib::*;

// ----------------------------------------------------------------------------
// Math Library
// ----------------------------------------------------------------------------

#[macro_export]
macro_rules! generate_math_fxns {
  ($lib:ident) => {
    generate_fxns!($lib,T,T,impl_binop);
  }
}

#[macro_export]
macro_rules! generate_urnop_match_arms2 {
  ($lib:ident, $arg:expr, $($lhs_type:ident => $($matrix_kind:ident, $target_type:ident, $default:expr),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          $(
            (Value::$lhs_type(arg)) => {
              Ok(Box::new([<$lib $lhs_type Scalar>]{arg: arg.clone(), out: new_ref($default) }))},
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(arg))) => {
              Ok(Box::new([<$lib $lhs_type M2>]{arg, out: new_ref(Matrix2::from_element($default))}))},
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(arg))) => {
              Ok(Box::new([<$lib $lhs_type M3>]{arg, out: new_ref(Matrix3::from_element($default))}))},
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector2(arg))) => {
              Ok(Box::new([<$lib $lhs_type R2>]{arg: arg.clone(), out: new_ref(RowVector2::from_element($default)) }))},
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector3(arg))) => {
              Ok(Box::new([<$lib $lhs_type R3>]{arg: arg.clone(), out: new_ref(RowVector3::from_element($default)) }))},
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector4(arg))) => {
              Ok(Box::new([<$lib $lhs_type R4>]{arg: arg.clone(), out: new_ref(RowVector4::from_element($default)) }))},
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2x3(arg))) => {
              Ok(Box::new([<$lib $lhs_type M2x3>]{arg, out: new_ref(Matrix2x3::from_element($default))}))},          
            (Value::$matrix_kind(Matrix::<$target_type>::RowDVector(arg))) => {
              let length = {arg.borrow().len()};
              Ok(Box::new([<$lib $lhs_type RD>]{arg, out: new_ref(RowDVector::from_element(length,$default))}))},
            (Value::$matrix_kind(Matrix::<$target_type>::DVector(arg))) => {
              let length = {arg.borrow().len()};
              Ok(Box::new([<$lib $lhs_type VD>]{arg, out: new_ref(DVector::from_element(length,$default))}))},
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(arg))) => {
              let (rows,cols) = {arg.borrow().shape()};
              Ok(Box::new([<$lib $lhs_type MD>]{arg, out: new_ref(DMatrix::from_element(rows,cols,$default))}))},
          )+
        )+
        x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

// Cos ------------------------------------------------------------------------


use libm::{cos,cosf};
macro_rules! cos_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = cos((*$arg).0);}
  };}

macro_rules! cos_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((*$out)[i]).0 = cos(((*$arg)[i]).0);
      }}};}

macro_rules! cosf_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = cosf((*$arg).0);}
  };}  

macro_rules! cosf_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((*$out)[i]).0 = cosf(((*$arg)[i]).0);
      }}};}

impl_urop!(MathCosF32Scalar, F32, F32, cosf_op);
impl_urop!(MathCosF32M2, Matrix2<F32>, Matrix2<F32>, cosf_vec_op);
impl_urop!(MathCosF32M3, Matrix3<F32>, Matrix3<F32>, cosf_vec_op);
impl_urop!(MathCosF32R2, RowVector2<F32>, RowVector2<F32>, cosf_vec_op);
impl_urop!(MathCosF32R3, RowVector3<F32>, RowVector3<F32>, cosf_vec_op);
impl_urop!(MathCosF32R4, RowVector4<F32>, RowVector4<F32>, cosf_vec_op);
impl_urop!(MathCosF32M2x3, Matrix2x3<F32>, Matrix2x3<F32>, cosf_vec_op);
impl_urop!(MathCosF32VD, DVector<F32>, DVector<F32>, cosf_vec_op);
impl_urop!(MathCosF32RD, RowDVector<F32>, RowDVector<F32>, cosf_vec_op);
impl_urop!(MathCosF32MD, DMatrix<F32>, DMatrix<F32>, cosf_vec_op);

impl_urop!(MathCosF64Scalar, F64, F64, cos_op);
impl_urop!(MathCosF64M2, Matrix2<F64>, Matrix2<F64>, cos_vec_op);
impl_urop!(MathCosF64M3, Matrix3<F64>, Matrix3<F64>, cos_vec_op);
impl_urop!(MathCosF64R2, RowVector2<F64>, RowVector2<F64>, cos_vec_op);
impl_urop!(MathCosF64R3, RowVector3<F64>, RowVector3<F64>, cos_vec_op);
impl_urop!(MathCosF64R4, RowVector4<F64>, RowVector4<F64>, cos_vec_op);
impl_urop!(MathCosF64M2x3, Matrix2x3<F64>, Matrix2x3<F64>, cos_vec_op);
impl_urop!(MathCosF64VD, DVector<F64>, DVector<F64>, cos_vec_op);
impl_urop!(MathCosF64RD, RowDVector<F64>, RowDVector<F64>, cos_vec_op);
impl_urop!(MathCosF64MD, DMatrix<F64>, DMatrix<F64>, cos_vec_op);

fn generate_cos_fxn(lhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  generate_urnop_match_arms2!(
    MathCos,
    (lhs_value),
    F32 => MatrixF32, F32, F32::zero();
    F64 => MatrixF64, F64, F64::zero();
  )
}

pub struct MathCos {}

impl NativeFunctionCompiler for MathCos {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let input = arguments[0].clone();
    match generate_cos_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (input) {
          (Value::MutableReference(input)) => {generate_cos_fxn(input.borrow().clone())}
          x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// Sin ------------------------------------------------------------------------

use libm::{sin,sinf};
macro_rules! sin_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = sin((*$arg).0);}
  };}

macro_rules! sin_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((*$out)[i]).0 = sin(((*$arg)[i]).0);
      }}};}

macro_rules! sinf_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = sinf((*$arg).0);}
  };}  

macro_rules! sinf_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((*$out)[i]).0 = sinf(((*$arg)[i]).0);
      }}};}

impl_urop!(MathSinF32Scalar, F32, F32, sinf_op);
impl_urop!(MathSinF32M2, Matrix2<F32>, Matrix2<F32>, sinf_vec_op);
impl_urop!(MathSinF32M3, Matrix3<F32>, Matrix3<F32>, sinf_vec_op);
impl_urop!(MathSinF32R2, RowVector2<F32>, RowVector2<F32>, sinf_vec_op);
impl_urop!(MathSinF32R3, RowVector3<F32>, RowVector3<F32>, sinf_vec_op);
impl_urop!(MathSinF32R4, RowVector4<F32>, RowVector4<F32>, sinf_vec_op);
impl_urop!(MathSinF32M2x3, Matrix2x3<F32>, Matrix2x3<F32>, sinf_vec_op);
impl_urop!(MathSinF32VD, DVector<F32>, DVector<F32>, sinf_vec_op);
impl_urop!(MathSinF32RD, RowDVector<F32>, RowDVector<F32>, sinf_vec_op);
impl_urop!(MathSinF32MD, DMatrix<F32>, DMatrix<F32>, sinf_vec_op);

impl_urop!(MathSinF64Scalar, F64, F64, sin_op);
impl_urop!(MathSinF64M2, Matrix2<F64>, Matrix2<F64>, sin_vec_op);
impl_urop!(MathSinF64M3, Matrix3<F64>, Matrix3<F64>, sin_vec_op);
impl_urop!(MathSinF64R2, RowVector2<F64>, RowVector2<F64>, sin_vec_op);
impl_urop!(MathSinF64R3, RowVector3<F64>, RowVector3<F64>, sin_vec_op);
impl_urop!(MathSinF64R4, RowVector4<F64>, RowVector4<F64>, sin_vec_op);
impl_urop!(MathSinF64M2x3, Matrix2x3<F64>, Matrix2x3<F64>, sin_vec_op);
impl_urop!(MathSinF64VD, DVector<F64>, DVector<F64>, sin_vec_op);
impl_urop!(MathSinF64RD, RowDVector<F64>, RowDVector<F64>, sin_vec_op);
impl_urop!(MathSinF64MD, DMatrix<F64>, DMatrix<F64>, sin_vec_op);

fn generate_sin_fxn(lhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  generate_urnop_match_arms2!(
    MathSin,
    (lhs_value),
    F32 => MatrixF32, F32, F32::zero();
    F64 => MatrixF64, F64, F64::zero();
  )
}

pub struct MathSin {}

impl NativeFunctionCompiler for MathSin {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let input = arguments[0].clone();
    match generate_sin_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (input) {
          (Value::MutableReference(input)) => {generate_sin_fxn(input.borrow().clone())}
          x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// Add ------------------------------------------------------------------------

macro_rules! add_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe { *$out = *$lhs + *$rhs; }
  };}

macro_rules! addto_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe { (*$lhs).add_to(&*$rhs,&mut *$out) }
  };}

macro_rules! add_scalar_lhs_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe { *$out = (*$lhs).add_scalar(*$rhs); }
  };}

macro_rules! add_scalar_rhs_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe { *$out = (*$rhs).add_scalar(*$lhs); }
  };}

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

impl_mech_binop_fxn!(MathAdd,generate_add_fxn);

// Sub ------------------------------------------------------------------------
macro_rules! subto_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe { (*$lhs).sub_to(&*$rhs,&mut *$out) }
  };}

macro_rules! sub_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe { *$out = *$lhs - *$rhs; }
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

impl_mech_binop_fxn!(MathSub,generate_sub_fxn);

// Mul ------------------------------------------------------------------------

macro_rules! mul_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe { *$out = *$lhs * *$rhs; }};}

macro_rules! mul_vec_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe { *$out = (*$lhs).component_mul(&*$rhs); }};}

macro_rules! mul_scalar_lhs_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe { *$out = (*$lhs).clone() * *$rhs; }};}

macro_rules! mul_scalar_rhs_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe { *$out = (*$rhs).clone() * *$lhs;}};}

generate_math_fxns!(Mul);

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

impl_mech_binop_fxn!(MathMul,generate_mul_fxn);


// Div ------------------------------------------------------------------------

macro_rules! div_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe { *$out = *$lhs / *$rhs; }
  };}

macro_rules! div_vec_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe { *$out = (*$lhs).component_div(&*$rhs); }
  };}

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

generate_math_fxns!(Div);

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

impl_mech_binop_fxn!(MathDiv,generate_div_fxn);

// Exp ------------------------------------------------------------------------

macro_rules! exp_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {*$out = (*$lhs).pow(*$rhs);}
  };}

macro_rules! exp_vec_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$lhs).len() {
        (*$out)[i] = (*$lhs)[i].pow((*$rhs)[i]);
      }}};}

macro_rules! exp_scalar_lhs_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$lhs).len() {
        (*$out)[i] = (*$lhs)[i].pow((*$rhs));
      }}};}

macro_rules! exp_scalar_rhs_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$rhs).len() {
        (*$out)[i] = (*$lhs).pow((*$rhs)[i]);
      }}};}

#[macro_export]
macro_rules! impl_expop {
  ($struct_name:ident, $arg1_type:ty, $arg2_type:ty, $out_type:ty, $op:ident) => {
    #[derive(Debug)]
    struct $struct_name<T> {
      lhs: Ref<$arg1_type>,
      rhs: Ref<$arg2_type>,
      out: Ref<$out_type>,
    }
    impl<T> MechFunction for $struct_name<T>
    where
      T: Copy + Debug + Clone + Sync + Send + 'static + 
      PartialEq + PartialOrd +
      Add<Output = T> + AddAssign +
      Sub<Output = T> + SubAssign +
      Mul<Output = T> + MulAssign +
      Div<Output = T> + DivAssign +
      Pow<T, Output = T> +
      Zero + One,
      Ref<$out_type>: ToValue
    {
      fn solve(&self) {
        let lhs_ptr = self.lhs.as_ptr();
        let rhs_ptr = self.rhs.as_ptr();
        let out_ptr = self.out.as_ptr();
        $op!(lhs_ptr,rhs_ptr,out_ptr);
      }
      fn out(&self) -> Value { self.out.to_value() }
      fn to_string(&self) -> String { format!("{:?}", self) }
    }};}

#[macro_export]
macro_rules! generate_math_fxns_exp {
  ($lib:ident) => {
    generate_fxns!($lib,T,T,impl_expop);
  }
}

generate_math_fxns_exp!(Exp);

fn generate_exp_fxn(lhs_value: Value, rhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  generate_binop_match_arms!(
    Exp,
    (lhs_value, rhs_value),
    U8,   U8   => MatrixU8,  u8,  u8::zero();
    U16,  U16  => MatrixU16, u16, u16::zero();
    U32,  U32  => MatrixU32, u32, u32::zero();
    F32,  F32  => MatrixF32,  F32,  F32::zero();
    F64,  F64  => MatrixF64,  F64,  F64::zero();
  )
}

impl_mech_binop_fxn!(MathExp,generate_exp_fxn);

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

impl_mech_urnop_fxn!(MathNegate,generate_neg_fxn);