#![allow(warnings)]
#![feature(step_trait)]
#![feature(box_patterns)]
#![feature(trivial_bounds)]
#![feature(where_clause_attrs)]

#[cfg(feature = "matrix")]
extern crate nalgebra as na;
#[macro_use]
extern crate mech_core;

use mech_core::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::{Matrix, ToMatrix};
use mech_core::kind::Kind;
use mech_core::{Dictionary, Ref, Value, ValueKind, ValRef, ToValue};
use mech_core::{MechError, MechErrorKind, hash_str, MResult, nodes::Kind as NodeKind, nodes::Matrix as Mat, nodes::*};
#[cfg(feature = "map")]
use mech_core::MechMap;
#[cfg(feature = "record")]
use mech_core::MechRecord;
#[cfg(feature = "set")]
use mech_core::MechSet;
#[cfg(feature = "tuple")]
use mech_core::MechTuple;
#[cfg(feature = "enum")]
use mech_core::MechEnum;
#[cfg(feature = "table")]
use mech_core::MechTable;
#[cfg(feature = "f64")]
use mech_core::F64;
#[cfg(feature = "f32")]
use mech_core::F32;
#[cfg(feature = "complex")]
use mech_core::C64;
#[cfg(feature = "rational")]
use mech_core::R64;
#[cfg(feature = "functions")]
use crate::functions::*;
#[cfg(feature = "access")]
use crate::stdlib::access::*;
#[cfg(feature = "assign")]
use crate::stdlib::assign::*;
#[cfg(feature = "convert")]
use crate::stdlib::convert::*;
#[cfg(feature = "matrix_horzcat")]
use crate::stdlib::horzcat::*;
#[cfg(feature = "matrix_vertcat")]
use crate::stdlib::vertcat::*;
#[cfg(feature = "combinatorics")]
use mech_combinatorics::*;
#[cfg(feature = "matrix")]
use mech_matrix::*;
#[cfg(feature = "stats")]
use mech_stats::*;
#[cfg(feature = "math")]
use mech_math::*;
#[cfg(feature = "logic")]
use mech_logic::*;
#[cfg(feature = "compare")]
use mech_compare::*;
#[cfg(feature = "range_inclusive")]
use mech_range::inclusive::RangeInclusive;
#[cfg(feature = "range_exclusive")]
use mech_range::exclusive::RangeExclusive;
#[cfg(feature = "set")]
use mech_set::*;

#[cfg(feature = "matrix")]
use na::DMatrix;
#[cfg(feature = "set")]
use indexmap::set::IndexSet;
#[cfg(any(feature = "map", feature = "table", feature = "record"))]
use indexmap::map::IndexMap;

pub mod literals;
pub mod structures;
pub mod interpreter;
pub mod stdlib;
#[cfg(feature = "functions")]
pub mod functions;
pub mod statements;
pub mod expressions;
pub mod mechdown;

pub use mech_core::*;

pub use crate::literals::*;
pub use crate::interpreter::*;
pub use crate::structures::*;
#[cfg(feature = "functifons")]
pub use crate::functions::*;
pub use crate::statements::*;
pub use crate::expressions::*;
pub use crate::mechdown::*;

#[cfg(feature = "access")]
pub use crate::stdlib::access::*;
#[cfg(feature = "assign")]
pub use crate::stdlib::assign::*;
#[cfg(feature = "convert")]
pub use crate::stdlib::convert::*;
#[cfg(feature = "matrix_horzcat")]
pub use crate::stdlib::horzcat::*;
#[cfg(feature = "matrix_vertcat")]
pub use crate::stdlib::vertcat::*;
#[cfg(feature = "combinatorics")]
pub use mech_combinatorics::*;
#[cfg(feature = "matrix")]
pub use mech_matrix::*;
#[cfg(feature = "stats")]
pub use mech_stats::*;
#[cfg(feature = "math")]
pub use mech_math::*;
#[cfg(feature = "logic")]
pub use mech_logic::*;
#[cfg(feature = "compare")]
pub use mech_compare::*;
#[cfg(feature = "set")]
pub use mech_set::*;

pub fn load_stdkinds(kinds: &mut KindTable) {
  #[cfg(feature = "u8")]
  kinds.insert(hash_str("u8"),ValueKind::U8);
  #[cfg(feature = "u16")]
  kinds.insert(hash_str("u16"),ValueKind::U16);
  #[cfg(feature = "u32")]
  kinds.insert(hash_str("u32"),ValueKind::U32);
  #[cfg(feature = "u64")]
  kinds.insert(hash_str("u64"),ValueKind::U64);
  #[cfg(feature = "u128")]
  kinds.insert(hash_str("u128"),ValueKind::U128);
  #[cfg(feature = "i8")]
  kinds.insert(hash_str("i8"),ValueKind::I8);
  #[cfg(feature = "i16")]
  kinds.insert(hash_str("i16"),ValueKind::I16);
  #[cfg(feature = "i32")]
  kinds.insert(hash_str("i32"),ValueKind::I32);
  #[cfg(feature = "i64")]
  kinds.insert(hash_str("i64"),ValueKind::I64);
  #[cfg(feature = "i128")]
  kinds.insert(hash_str("i128"),ValueKind::I128);
  #[cfg(feature = "f32")]
  kinds.insert(hash_str("f32"),ValueKind::F32);
  #[cfg(feature = "f64")]
  kinds.insert(hash_str("f64"),ValueKind::F64);
  #[cfg(feature = "c64")]
  kinds.insert(hash_str("c64"),ValueKind::C64);
  #[cfg(feature = "r64")]
  kinds.insert(hash_str("r64"),ValueKind::R64);
  #[cfg(feature = "string")]
  kinds.insert(hash_str("string"),ValueKind::String);
  #[cfg(feature = "bool")]
  kinds.insert(hash_str("bool"),ValueKind::Bool);
}

#[cfg(feature = "functions")]
pub fn load_stdlib(fxns: &mut Functions) {

  for fxn_desc in inventory::iter::<FunctionDescriptor> {
    fxns.insert_function(fxn_desc.clone());
  }

  for fxn_comp in inventory::iter::<FunctionCompilerDescriptor> {
    fxns.function_compilers.insert(hash_str(fxn_comp.name), fxn_comp.ptr);
  }

}