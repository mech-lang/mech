use crate::*;
use mech_core::*;

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

impl_logic_fxns!(Xor);

fn impl_xor_fxn(lhs_value: Value, rhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_binop_match_arms!(
    Xor,
    (lhs_value, rhs_value),
    Bool, Bool => MatrixBool, bool, false, "Bool";
  )
}

impl_mech_binop_fxn!(LogicXor,impl_xor_fxn);