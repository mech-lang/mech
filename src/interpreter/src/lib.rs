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
#[cfg(feature = "io_print")]
use mech_io::IoPrint;
#[cfg(feature = "io_println")]
use mech_io::IoPrintln;
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
#[cfg(feature = "stdlib")]
use crate::stdlib::{
                    access::*,
                    assign::*,
                    convert::*,
                  };
#[cfg(feature = "functions")]
use crate::functions::*;
#[cfg(feature = "matrix_horzcat")]
use crate::stdlib::horzcat::*;
#[cfg(feature = "matrix_vertcat")]
use crate::stdlib::vertcat::*;
use mech_core::{MechError, MechErrorKind, hash_str, MResult, nodes::Kind as NodeKind, nodes::Matrix as Mat, nodes::*};

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

pub use crate::literals::*;
pub use crate::interpreter::*;
pub use crate::structures::*;
#[cfg(feature = "functifons")]
pub use crate::functions::*;
pub use crate::statements::*;
pub use crate::expressions::*;
pub use crate::mechdown::*;


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

  // Preload combinatorics functions
  #[cfg(feature = "combinatorics_n_choose_k")]
  fxns.function_compilers.insert(hash_str("combinatorics/n-choose-k"), Box::new(CombinatoricsNChooseK{}));

  // Preload stats functions
  #[cfg(feature = "stats_sum")]
  fxns.function_compilers.insert(hash_str("stats/sum/row"), Box::new(StatsSumRow{}));
  #[cfg(feature = "stats_sum")]
  fxns.function_compilers.insert(hash_str("stats/sum/column"), Box::new(StatsSumColumn{}));

  // Preload math functions
  #[cfg(feature = "math_add")]
  fxns.function_compilers.insert(hash_str("math/add"),Box::new(MathAdd{}));
  #[cfg(feature = "math_sub")]
  fxns.function_compilers.insert(hash_str("math/sub"),Box::new(MathSub{}));
  #[cfg(feature = "math_mul")]
  fxns.function_compilers.insert(hash_str("math/mul"),Box::new(MathMul{}));
  #[cfg(feature = "math_div")]
  fxns.function_compilers.insert(hash_str("math/div"),Box::new(MathDiv{}));
  #[cfg(feature = "math_mod")]
  fxns.function_compilers.insert(hash_str("math/mod"),Box::new(MathMod{}));
  #[cfg(feature = "math_exp")]
  fxns.function_compilers.insert(hash_str("math/exp"),Box::new(MathExp{}));
  #[cfg(feature = "math_neg")]
  fxns.function_compilers.insert(hash_str("math/neg"),Box::new(MathNegate{}));
  #[cfg(feature = "math_sin")]
  fxns.function_compilers.insert(hash_str("math/sin"),Box::new(MathSin{}));
  #[cfg(feature = "math_cos")]
  fxns.function_compilers.insert(hash_str("math/cos"),Box::new(MathCos{}));
  #[cfg(feature = "math_atan2")]
  fxns.function_compilers.insert(hash_str("math/atan2"),Box::new(MathAtan2{}));
  #[cfg(feature = "math_atan")]
  fxns.function_compilers.insert(hash_str("math/atan"),Box::new(MathAtan{}));
  #[cfg(feature = "math_acos")]
  fxns.function_compilers.insert(hash_str("math/acos"),Box::new(MathAcos{}));
  #[cfg(feature = "math_acosh")]
  fxns.function_compilers.insert(hash_str("math/acosh"),Box::new(MathAcosh{}));
  #[cfg(feature = "math_acot")]
  fxns.function_compilers.insert(hash_str("math/acot"),Box::new(MathAcot{}));
  #[cfg(feature = "math_acsc")]
  fxns.function_compilers.insert(hash_str("math/acsc"),Box::new(MathAcsc{}));
  #[cfg(feature = "math_asec")]
  fxns.function_compilers.insert(hash_str("math/asec"),Box::new(MathAsec{}));
  #[cfg(feature = "math_asin")]
  fxns.function_compilers.insert(hash_str("math/asin"),Box::new(MathAsin{}));
  #[cfg(feature = "math_sinh")]
  fxns.function_compilers.insert(hash_str("math/sinh"),Box::new(MathSinh{}));
  #[cfg(feature = "math_cosh")]
  fxns.function_compilers.insert(hash_str("math/cosh"),Box::new(MathCosh{}));
  #[cfg(feature = "math_tanh")]
  fxns.function_compilers.insert(hash_str("math/tanh"),Box::new(MathTanh{}));
  #[cfg(feature = "math_atanh")]
  fxns.function_compilers.insert(hash_str("math/atanh"),Box::new(MathAtanh{}));
  #[cfg(feature = "math_cot")]
  fxns.function_compilers.insert(hash_str("math/cot"),Box::new(MathCot{}));
  #[cfg(feature = "math_csc")]
  fxns.function_compilers.insert(hash_str("math/csc"),Box::new(MathCsc{}));
  #[cfg(feature = "math_sec")]
  fxns.function_compilers.insert(hash_str("math/sec"),Box::new(MathSec{}));
  #[cfg(feature = "math_tan")]
  fxns.function_compilers.insert(hash_str("math/tan"),Box::new(MathTan{}));

  // Preload io functions
  #[cfg(feature = "io_print")]
  fxns.function_compilers.insert(hash_str("io/print"), Box::new(IoPrint{}));
  #[cfg(feature = "io_println")]
  fxns.function_compilers.insert(hash_str("io/println"), Box::new(IoPrintln{}));

  // Matrix functions
  #[cfg(feature = "matrix_horzcat")]
  fxns.function_compilers.insert(hash_str("matrix/horzcat"), Box::new(MatrixHorzCat{}));
  #[cfg(feature = "matrix_vertcat")]
  fxns.function_compilers.insert(hash_str("matrix/vertcat"), Box::new(MatrixVertCat{}));
  #[cfg(feature = "matrix_transpose")]
  fxns.function_compilers.insert(hash_str("matrix/transpose"), Box::new(MatrixTranspose{}));
  #[cfg(feature = "matrix_matmul")]
  fxns.function_compilers.insert(hash_str("matrix/matmul"), Box::new(MatrixMatMul{}));
  #[cfg(feature = "matrix_dot")]
  fxns.function_compilers.insert(hash_str("matrix/dot"), Box::new(MatrixDot{}));

  // Compare functions
  #[cfg(feature = "compare_eq")]
  fxns.function_compilers.insert(hash_str("compare/eq"), Box::new(CompareEqual{}));
  #[cfg(feature = "compare_neq")]
  fxns.function_compilers.insert(hash_str("compare/neq"), Box::new(CompareNotEqual{}));
  #[cfg(feature = "compare_lte")]
  fxns.function_compilers.insert(hash_str("compare/lte"), Box::new(CompareLessThanEqual{}));
  #[cfg(feature = "compare_gte")]
  fxns.function_compilers.insert(hash_str("compare/gte"), Box::new(CompareGreaterThanEqual{}));
  #[cfg(feature = "compare_lt")]
  fxns.function_compilers.insert(hash_str("compare/lt"), Box::new(CompareLessThan{}));
  #[cfg(feature = "compare_gt")]
  fxns.function_compilers.insert(hash_str("compare/gt"), Box::new(CompareGreaterThan{}));

  // Logic functions
  #[cfg(feature = "logic_and")]
  fxns.function_compilers.insert(hash_str("logic/and"), Box::new(LogicAnd{}));
  #[cfg(feature = "logic_or")]
  fxns.function_compilers.insert(hash_str("logic/or"), Box::new(LogicOr{}));
  #[cfg(feature = "logic_not")]
  fxns.function_compilers.insert(hash_str("logic/not"), Box::new(LogicNot{}));
  #[cfg(feature = "logic_xor")]
  fxns.function_compilers.insert(hash_str("logic/xor"), Box::new(LogicXor{}));  

  for fxn_desc in inventory::iter::<FunctionDescriptor> {
    fxns.insert_function(fxn_desc.clone());
  }

}