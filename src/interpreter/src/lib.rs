#![allow(warnings)]
#![feature(where_clause_attrs)]

#[cfg(feature = "matrix")]
extern crate nalgebra as na;
#[macro_use]
extern crate mech_core;

#[cfg(feature = "trace")]
#[macro_export]
macro_rules! trace_println {
  ($interpreter:expr, $($arg:tt)*) => {
    if $interpreter.trace {
      let __trace_line = format!($($arg)*);
      $interpreter.push_trace_line(__trace_line.clone());
      if $interpreter.trace_to_stdout {
        println!("{}", __trace_line);
      }
    }
  };
}

#[cfg(not(feature = "trace"))]
#[macro_export]
macro_rules! trace_println {
  ($interpreter:expr, $($arg:tt)*) => {};
}

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
#[cfg(feature = "table")]
use crate::stdlib::table_ops::*;
#[cfg(feature = "matrix_vertcat")]
use crate::stdlib::vertcat::*;
#[cfg(feature = "combinatorics")]
use mech_combinatorics::*;
#[cfg(feature = "compare")]
use mech_compare::*;
use mech_core::kind::Kind;
#[cfg(feature = "matrix")]
use mech_core::matrix::{Matrix, ToMatrix};
#[cfg(feature = "enum")]
use mech_core::MechEnum;
#[cfg(feature = "map")]
use mech_core::MechMap;
#[cfg(feature = "record")]
use mech_core::MechRecord;
#[cfg(feature = "set")]
use mech_core::MechSet;
#[cfg(feature = "table")]
use mech_core::MechTable;
#[cfg(feature = "tuple")]
use mech_core::MechTuple;
#[cfg(feature = "complex")]
use mech_core::C64;
#[cfg(feature = "rational")]
use mech_core::R64;
use mech_core::*;
use mech_core::{hash_str, nodes::Kind as NodeKind, nodes::Matrix as Mat, nodes::*, MResult};
use mech_core::{Dictionary, Ref, ToValue, ValRef, Value, ValueKind};
#[cfg(feature = "logic")]
use mech_logic::*;
#[cfg(feature = "math")]
use mech_math::*;
#[cfg(feature = "matrix")]
use mech_matrix::*;
#[cfg(feature = "range_exclusive")]
use mech_range::exclusive::*;
#[cfg(feature = "range_exclusive")]
use mech_range::exclusive_increment::*;
#[cfg(feature = "range_inclusive")]
use mech_range::inclusive::*;
#[cfg(feature = "range_inclusive")]
use mech_range::inclusive_increment::*;
#[cfg(feature = "set")]
use mech_set::*;
#[cfg(feature = "stats")]
use mech_stats::*;
#[cfg(feature = "string")]
use mech_string::*;

#[cfg(any(feature = "map", feature = "table", feature = "record"))]
use indexmap::map::IndexMap;
#[cfg(feature = "set")]
use indexmap::set::IndexSet;
#[cfg(feature = "matrix")]
use na::DMatrix;
use std::time::Duration;

pub mod expressions;
#[cfg(feature = "functions")]
pub mod functions;
pub mod interpreter;
pub mod literals;
pub mod mechdown;
pub mod patterns;
#[cfg(feature = "state_machines")]
pub mod state_machines;
pub mod statements;
pub mod stdlib;
pub mod structures;
pub mod tracing;

pub use mech_core::*;

pub use crate::expressions::*;
#[cfg(feature = "functions")]
pub use crate::functions::*;
pub use crate::interpreter::*;
pub use crate::literals::*;
pub use crate::mechdown::*;
pub use crate::patterns::*;
#[cfg(feature = "state_machines")]
pub use crate::state_machines::*;
pub use crate::statements::*;
pub use crate::structures::*;
pub use crate::tracing::*;

#[cfg(feature = "access")]
pub use crate::stdlib::access::*;
#[cfg(feature = "assign")]
pub use crate::stdlib::assign::*;
#[cfg(feature = "convert")]
pub use crate::stdlib::convert::*;
#[cfg(feature = "matrix_horzcat")]
pub use crate::stdlib::horzcat::*;
#[cfg(feature = "table")]
pub use crate::stdlib::table_ops::*;
#[cfg(feature = "matrix_vertcat")]
pub use crate::stdlib::vertcat::*;
#[cfg(feature = "combinatorics")]
pub use mech_combinatorics::*;
#[cfg(feature = "compare")]
pub use mech_compare::*;
#[cfg(feature = "logic")]
pub use mech_logic::*;
#[cfg(feature = "math")]
pub use mech_math::*;
#[cfg(feature = "matrix")]
pub use mech_matrix::*;
#[cfg(feature = "set")]
pub use mech_set::*;
#[cfg(feature = "stats")]
pub use mech_stats::*;

pub fn load_stdkinds(kinds: &mut KindTable) {
  #[cfg(feature = "u8")]
  kinds.insert(hash_str("u8"), ValueKind::U8);
  #[cfg(feature = "u16")]
  kinds.insert(hash_str("u16"), ValueKind::U16);
  #[cfg(feature = "u32")]
  kinds.insert(hash_str("u32"), ValueKind::U32);
  #[cfg(feature = "u64")]
  kinds.insert(hash_str("u64"), ValueKind::U64);
  #[cfg(feature = "u128")]
  kinds.insert(hash_str("u128"), ValueKind::U128);
  #[cfg(feature = "i8")]
  kinds.insert(hash_str("i8"), ValueKind::I8);
  #[cfg(feature = "i16")]
  kinds.insert(hash_str("i16"), ValueKind::I16);
  #[cfg(feature = "i32")]
  kinds.insert(hash_str("i32"), ValueKind::I32);
  #[cfg(feature = "i64")]
  kinds.insert(hash_str("i64"), ValueKind::I64);
  #[cfg(feature = "i128")]
  kinds.insert(hash_str("i128"), ValueKind::I128);
  #[cfg(feature = "f32")]
  kinds.insert(hash_str("f32"), ValueKind::F32);
  #[cfg(feature = "f64")]
  kinds.insert(hash_str("f64"), ValueKind::F64);
  #[cfg(feature = "c64")]
  kinds.insert(hash_str("c64"), ValueKind::C64);
  #[cfg(feature = "r64")]
  kinds.insert(hash_str("r64"), ValueKind::R64);
  #[cfg(feature = "string")]
  kinds.insert(hash_str("string"), ValueKind::String);
  #[cfg(feature = "bool")]
  kinds.insert(hash_str("bool"), ValueKind::Bool);
}

#[cfg(feature = "functions")]
pub fn load_stdlib(fxns: &mut Functions) {
  for fxn_desc in inventory::iter::<FunctionDescriptor> {
    fxns.insert_function(fxn_desc.clone());
  }

  for fxn_comp in inventory::iter::<FunctionCompilerDescriptor> {
    fxns.function_compilers
      .insert(hash_str(fxn_comp.name), fxn_comp.ptr);
  }
}

fn format_duration(d: Duration) -> String {
  let ns = d.as_nanos();
  if ns < 1_000 {
    format!("{}ns", ns)
  } else if ns < 1_000_000 {
    format!("{:.2}µs", ns as f64 / 1_000.0)
  } else if ns < 1_000_000_000 {
    format!("{:.2}ms", ns as f64 / 1_000_000.0)
  } else {
    format!("{:.2}s", ns as f64 / 1_000_000_000.0)
  }
}

fn print_histogram(total_durations: &[Duration]) {
  let max_duration = total_durations
    .iter()
    .cloned()
    .max()
    .unwrap_or(Duration::ZERO);
  let max_bar_len = 50; // max characters for the bar

  println!("{:>5}  {:>10}  {}", "#", "Time", "Histogram");
  println!("-----------------------------------------------");

  for (idx, dur) in total_durations.iter().enumerate() {
    let bar_len = if max_duration.as_nanos() == 0 {
        0
    } else {
        ((dur.as_nanos() * max_bar_len as u128) / max_duration.as_nanos()) as usize
    };
    let bar = std::iter::repeat('░').take(bar_len).collect::<String>();

    println!("{:>5}  {:>10}  {}", idx, format_duration(*dur), bar);
  }
}
