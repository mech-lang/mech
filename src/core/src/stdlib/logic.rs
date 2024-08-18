#[macro_use]
use crate::stdlib::*;

// ----------------------------------------------------------------------------
// Logic Library
// ----------------------------------------------------------------------------



#[macro_export]
macro_rules! impl_logic_binop {
  ($struct_name:ident, $arg1_type:ty, $arg2_type:ty, $out_type:ty, $op:ident) => {
    #[derive(Debug)]
    struct $struct_name {
      lhs: Ref<$arg1_type>,
      rhs: Ref<$arg2_type>,
      out: Ref<$out_type>,
    }
    impl MechFunction for $struct_name {
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
macro_rules! impl_logic_urnop {
  ($struct_name:ident, $arg_type:ty, $out_type:ty, $op:ident) => {
    #[derive(Debug)]
    struct $struct_name {
      arg: Ref<$arg_type>,
      out: Ref<$out_type>,
    }
    impl MechFunction for $struct_name {
      fn solve(&self) {
        let arg_ptr = self.arg.as_ptr();
        let out_ptr = self.out.as_ptr();
        $op!(arg_ptr,out_ptr);
      }
      fn out(&self) -> Value { self.out.to_value() }
      fn to_string(&self) -> String { format!("{:?}", self) }
    }};}

#[macro_export]
macro_rules! generate_logic_fxns {
  ($lib:ident) => {
    generate_fxns!($lib,bool,bool,impl_logic_binop);
  }
}

// And ------------------------------------------------------------------------

macro_rules! and_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {*$out = *$lhs && *$rhs;}
    };}

macro_rules! and_vec_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$lhs).len() {
        (*$out)[i] = (*$lhs)[i] && (*$rhs)[i];
      }}};}
    
macro_rules! and_scalar_rhs_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$rhs).len() {
        (*$out)[i] = (*$lhs) && (*$rhs)[i];
      }}};}
      

macro_rules! and_scalar_lhs_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$lhs).len() {
        (*$out)[i] = (*$lhs)[i] && (*$rhs);
      }}};}

generate_logic_fxns!(And);

fn generate_and_fxn(lhs_value: Value, rhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  generate_binop_match_arms!(
    And,
    (lhs_value, rhs_value),
    Bool, Bool => MatrixBool, bool, false;
  )
}

impl_mech_binop_fxn!(LogicAnd,generate_and_fxn);

// Or ------------------------------------------------------------------------

macro_rules! or_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {*$out = *$lhs || *$rhs;}
    };}

macro_rules! or_vec_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$lhs).len() {
        (*$out)[i] = (*$lhs)[i] || (*$rhs)[i];
      }}};}
    
macro_rules! or_scalar_rhs_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$rhs).len() {
        (*$out)[i] = (*$lhs) || (*$rhs)[i];
      }}};}
      

macro_rules! or_scalar_lhs_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$lhs).len() {
        (*$out)[i] = (*$lhs)[i] || (*$rhs);
      }}};}

generate_logic_fxns!(Or);

fn generate_or_fxn(lhs_value: Value, rhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  generate_binop_match_arms!(
    Or,
    (lhs_value, rhs_value),
    Bool, Bool => MatrixBool, bool, false;
  )
}

impl_mech_binop_fxn!(LogicOr,generate_or_fxn);

// Xor ------------------------------------------------------------------------
macro_rules! xor_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {*$out = *$lhs ^ *$rhs;}
    };}

macro_rules! xor_vec_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$lhs).len() {
        (*$out)[i] = (*$lhs)[i] ^ (*$rhs)[i];
      }}};}
    
macro_rules! xor_scalar_rhs_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$rhs).len() {
        (*$out)[i] = (*$lhs) ^ (*$rhs)[i];
      }}};}
      

macro_rules! xor_scalar_lhs_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$lhs).len() {
        (*$out)[i] = (*$lhs)[i] ^ (*$rhs);
      }}};} 

generate_logic_fxns!(Xor);

fn generate_xor_fxn(lhs_value: Value, rhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  generate_binop_match_arms!(
    Xor,
    (lhs_value, rhs_value),
    Bool, Bool => MatrixBool, bool, false;
  )
}

impl_mech_binop_fxn!(LogicXor,generate_xor_fxn);

// Not ------------------------------------------------------------------------

macro_rules! not_op {
  ($arg:expr, $out:expr) => {
    unsafe {*$out = !*$arg;}
    };}

macro_rules! not_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        (*$out)[i] = !(*$arg)[i];
      }}};}

impl_logic_urnop!(NotS, bool, bool, not_op);
impl_logic_urnop!(NotM1, Matrix1<bool>, Matrix1<bool>, not_vec_op);
impl_logic_urnop!(NotM2, Matrix2<bool>, Matrix2<bool>, not_vec_op);
impl_logic_urnop!(NotM3, Matrix3<bool>, Matrix3<bool>, not_vec_op);
impl_logic_urnop!(NotM4, Matrix4<bool>, Matrix4<bool>, not_vec_op);
impl_logic_urnop!(NotM2x3, Matrix2x3<bool>, Matrix2x3<bool>, not_vec_op);
impl_logic_urnop!(NotM3x2, Matrix3x2<bool>, Matrix3x2<bool>, not_vec_op);
impl_logic_urnop!(NotR2, RowVector2<bool>, RowVector2<bool>, not_vec_op);
impl_logic_urnop!(NotR3, RowVector3<bool>, RowVector3<bool>, not_vec_op);
impl_logic_urnop!(NotR4, RowVector4<bool>, RowVector4<bool>, not_vec_op);
impl_logic_urnop!(NotRD, RowDVector<bool>, RowDVector<bool>, not_vec_op);
impl_logic_urnop!(NotV2, Vector2<bool>, Vector2<bool>, not_vec_op);
impl_logic_urnop!(NotV3, Vector3<bool>, Vector3<bool>, not_vec_op);
impl_logic_urnop!(NotV4, Vector4<bool>, Vector4<bool>, not_vec_op);
impl_logic_urnop!(NotVD, DVector<bool>, DVector<bool>, not_vec_op);
impl_logic_urnop!(NotMD, DMatrix<bool>, DMatrix<bool>, not_vec_op);

fn generate_not_fxn(arg_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  generate_urnop_match_arms!(
    Not,
    (arg_value),
    Bool => MatrixBool, bool, false;
  )
}

impl_mech_urnop_fxn!(LogicNot,generate_not_fxn);