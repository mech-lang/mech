#![allow(warnings)]
#![feature(step_trait)]

extern crate nalgebra as na;
#[macro_use]
extern crate mech_core;

use mech_core::matrix::{Matrix, ToMatrix};
use mech_core::kind::Kind;
use mech_core::{Value, ValueKind, ValRef, ToValue};
use mech_core::{MechMap, MechFunction, MechRecord, MechTable, MechSet, MechTuple, MechEnum};
use mech_core::{F64, F32};
use mech_core::{Functions, FunctionsRef, FunctionDefinition, NativeFunctionCompiler, Plan, UserFunction, SymbolTableRef, SymbolTable};
use crate::stdlib::{
                    access::*,
                    assign::*,
                    convert::*,
                    horzcat::*,
                    vertcat::*,
                    math::*,
                    compare::*,
                  };
use mech_core::{MechError, MechErrorKind, hash_str, new_ref, MResult, nodes::Kind as NodeKind, nodes::Matrix as Mat, nodes::*};

use mech_matrix::*;
use mech_stats::*;
use mech_math::*;
use mech_logic::*;
use mech_range::{
  inclusive::RangeInclusive,
  exclusive::RangeExclusive,
};

use na::DMatrix;
use indexmap::set::IndexSet;
use indexmap::map::IndexMap;

pub mod literals;
pub mod structures;
pub mod interpreter;
pub mod stdlib;
pub mod functions;
pub mod statements;
pub mod expressions;
pub mod mechdown;

pub use crate::interpreter::*;
pub use crate::literals::*;
pub use crate::structures::*;
pub use crate::functions::*;
pub use crate::statements::*;
pub use crate::expressions::*;
pub use crate::mechdown::*;