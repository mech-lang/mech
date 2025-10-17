use crate::*;
use num_traits::*;

use mech_core::matrix::Matrix;

#[cfg(feature = "set")]
use indexmap::set::IndexSet;
use mech_core::set::MechSet;

// Complement -------------------------------------------------------------------

macro_rules! complement_op {
($lhs:expr, $rhs:expr, $out:expr) => {
	unsafe { let new_set = (*$lhs).set.complement((*$rhs).set);
		*$out = MechSet{kind: (*$lhs).kind, num_elements: new_set.len(), set: new_set}; }
	};}

impl_set_fxns!(Complement);

fn impl_complement_fxn(lhs_value: Value, rhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
	impl_binop_match_arms!(
		Complement,
		register_fxn_descriptor_inner,
		(lhs_value, rhs_value),
		I8,   i8,   "i8";
		I16,  i16,  "i16";
		I32,  i32,  "i32";
		I64,  i64,  "i64";
		I128, i128, "i128";
		U8,   u8,   "u8";
		U16,  u16,  "u16";
		U32,  u32,  "u32";
		U64,  u64,  "u64";
		U128, u128, "u128";
		F32,  F32,  "f32";
		F64,  F64,  "f64";
		R64, R64, "rational";
		C64, C64, "complex";
	)
}

impl_mech_binop_fxn!(SetComplement,impl_complement_fxn);
