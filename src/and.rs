use crate::*;
use mech_core::*;

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
  
  impl_logic_fxns!(And);
  
  fn impl_and_fxn(lhs_value: Value, rhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
    impl_binop_match_arms!(
      And,
      (lhs_value, rhs_value),
      Bool, Bool => MatrixBool, bool, false, "Bool";
    )
  }
  
  impl_mech_binop_fxn!(LogicAnd,impl_and_fxn);