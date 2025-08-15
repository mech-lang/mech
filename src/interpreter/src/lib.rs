#![allow(warnings)]
#![feature(step_trait)]
#![feature(box_patterns)]
#![feature(trivial_bounds)]

#[cfg(feature = "matrix")]
extern crate nalgebra as na;
#[macro_use]
extern crate mech_core;

use mech_core::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::{Matrix, ToMatrix};
use mech_core::kind::Kind;
use mech_core::{Dictionary, Ref, Value, ValueKind, ValRef, ToValue};
use mech_core::{MechMap, MechFunction, MechRecord, MechSet, MechTuple, MechEnum};
#[cfg(feature = "table")]
use mech_core::MechTable;
#[cfg(feature = "f64")]
use mech_core::F64;
#[cfg(feature = "f32")]
use mech_core::F32;
#[cfg(feature = "complex")]
use mech_core::ComplexNumber;
#[cfg(feature = "rational")]
use mech_core::RationalNumber;
use mech_core::{Functions, FunctionsRef, FunctionDefinition, NativeFunctionCompiler, Plan, UserFunction, SymbolTableRef, SymbolTable};
use crate::stdlib::{
                    access::*,
                    assign::*,
                    convert::*,
                  };
#[cfg(feature = "matrix")]
use crate::stdlib::horzcat::*;
#[cfg(feature = "matrix")]
use crate::stdlib::vertcat::*;
use mech_core::{MechError, MechErrorKind, hash_str, new_ref, MResult, nodes::Kind as NodeKind, nodes::Matrix as Mat, nodes::*};

#[cfg(feature = "combinatorics")]
use mech_combinatorics::*;
#[cfg(feature = "io")]
use mech_io::*;
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
#[cfg(feature = "range")]
use mech_range::{
  inclusive::RangeInclusive,
  exclusive::RangeExclusive,
};

#[cfg(feature = "matrix")]
use na::DMatrix;
use indexmap::set::IndexSet;
use indexmap::map::IndexMap;

pub mod literals;
pub mod structures;
pub mod interpreter;
pub mod stdlib;
//pub mod functions;
pub mod statements;
pub mod expressions;
pub mod mechdown;

pub use crate::interpreter::*;
pub use crate::literals::*;
pub use crate::structures::*;
//pub use crate::functions::*;
pub use crate::statements::*;
pub use crate::expressions::*;
pub use crate::mechdown::*;