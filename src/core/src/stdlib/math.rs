#[macro_use]
use crate::stdlib::*;

// ----------------------------------------------------------------------------
// Math Library
// ----------------------------------------------------------------------------

#[macro_export]
macro_rules! impl_math_fxns {
  ($lib:ident) => {
    impl_fxns!($lib,T,T,impl_binop);
  }
}

#[macro_export]
macro_rules! impl_urnop_match_arms2 {
  ($lib:ident, $arg:expr, $($lhs_type:ident => $($matrix_kind:ident, $target_type:ident, $default:expr),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          $(
            (Value::$lhs_type(arg)) => Ok(Box::new([<$lib $lhs_type S>]{arg: arg.clone(), out: new_ref($default) })),
            (Value::$matrix_kind(Matrix::<$target_type>::RowDVector(arg))) => Ok(Box::new([<$lib $lhs_type RD>]{arg: arg.clone(), out: new_ref(RowDVector::from_element(arg.borrow().len(),$default))})),
            (Value::$matrix_kind(Matrix::<$target_type>::DVector(arg))) => Ok(Box::new([<$lib $lhs_type VD>]{arg: arg.clone(), out: new_ref(DVector::from_element(arg.borrow().len(),$default))})),
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

macro_rules! impl_math_urop {
  ($fxn_name:ident, $type:ident, $op_fxn:ident) => {
    paste!{
      impl_urop!([<$fxn_name $type S>], $type, $type, [<$op_fxn _op>]);
      impl_urop!([<$fxn_name $type VD>], DVector<$type>, DVector<$type>, [<$op_fxn _vec_op>]);
      impl_urop!([<$fxn_name $type RD>], RowDVector<$type>, RowDVector<$type>, [<$op_fxn _vec_op>]);
      impl_urop!([<$fxn_name $type MD>], DMatrix<$type>, DMatrix<$type>, [<$op_fxn _vec_op>]);
    }
  }
}

impl_math_urop!(MathCos, F32, cosf);
impl_math_urop!(MathCos, F64, cos);


fn impl_cos_fxn(lhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_urnop_match_arms2!(
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
    match impl_cos_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (input) {
          (Value::MutableReference(input)) => {impl_cos_fxn(input.borrow().clone())}
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

impl_math_urop!(MathSin, F32, sinf);
impl_math_urop!(MathSin, F64, sin);

fn impl_sin_fxn(lhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_urnop_match_arms2!(
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
    match impl_sin_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (input) {
          (Value::MutableReference(input)) => {impl_sin_fxn(input.borrow().clone())}
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

macro_rules! add_vec_op {
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

impl_math_fxns!(Add);

fn impl_add_fxn(lhs_value: Value, rhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_binop_match_arms!(
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

impl_mech_binop_fxn!(MathAdd,impl_add_fxn);

// Sub ------------------------------------------------------------------------

macro_rules! sub_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe { *$out = *$lhs - *$rhs; }
  };}

macro_rules! sub_vec_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe { (*$lhs).sub_to(&*$rhs,&mut *$out) }
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

impl_math_fxns!(Sub);

fn impl_sub_fxn(lhs_value: Value, rhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_binop_match_arms!(
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

impl_mech_binop_fxn!(MathSub,impl_sub_fxn);

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

impl_math_fxns!(Mul);

fn impl_mul_fxn(lhs_value: Value, rhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_binop_match_arms!(
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

impl_mech_binop_fxn!(MathMul,impl_mul_fxn);


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

impl_math_fxns!(Div);

fn impl_div_fxn(lhs_value: Value, rhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_binop_match_arms!(
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

impl_mech_binop_fxn!(MathDiv,impl_div_fxn);

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
macro_rules! impl_math_fxns_exp {
  ($lib:ident) => {
    impl_fxns!($lib,T,T,impl_expop);
  }
}

impl_math_fxns_exp!(Exp);

fn impl_exp_fxn(lhs_value: Value, rhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_binop_match_arms!(
    Exp,
    (lhs_value, rhs_value),
    U8,   U8   => MatrixU8,  u8,  u8::zero();
    U16,  U16  => MatrixU16, u16, u16::zero();
    U32,  U32  => MatrixU32, u32, u32::zero();
    F32,  F32  => MatrixF32,  F32,  F32::zero();
    F64,  F64  => MatrixF64,  F64,  F64::zero();
  )
}

impl_mech_binop_fxn!(MathExp,impl_exp_fxn);

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

impl_neg_op!(NegateS, T, neg_op);
impl_neg_op!(NegateRD, RowDVector<T>,neg_vec_op);
impl_neg_op!(NegateVD, DVector<T>,neg_vec_op);
impl_neg_op!(NegateMD, DMatrix<T>,neg_vec_op);
  
fn impl_neg_fxn(lhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_urnop_match_arms!(
    Negate,
    (lhs_value),
    I8 => MatrixI8,     i8,   i8::zero();
    I16 => MatrixI16,   i16,  i16::zero();
    I32 => MatrixI32,   i32,  i32::zero();
    I64 => MatrixI64,   i64,  i64::zero();
    I128 => MatrixI128, i128, i128::zero();
    F32 => MatrixF32,   F32,  F32::zero();
    F64 => MatrixF64,   F64,  F64::zero();
  )
}

impl_mech_urnop_fxn!(MathNegate,impl_neg_fxn);