#![cfg_attr(feature = "no-std", no_std)]
#![cfg_attr(feature = "no-std", alloc)]
#![allow(dead_code)]
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
use crate::function::*;
#[cfg(feature = "access")]
use crate::intrinsics::access::*;
#[cfg(feature = "assign")]
use crate::intrinsics::assign::*;
#[cfg(feature = "convert")]
use crate::intrinsics::convert::*;
#[cfg(feature = "matrix_horzcat")]
use crate::intrinsics::horzcat::*;
#[cfg(feature = "table")]
use crate::intrinsics::table_ops::*;
#[cfg(feature = "matrix_vertcat")]
use crate::intrinsics::vertcat::*;
#[cfg(any(feature = "map", feature = "table", feature = "record"))]
use indexmap::map::IndexMap;
#[cfg(feature = "set")]
use indexmap::set::IndexSet;
#[cfg(feature = "complex")]
use mech_core::C64;
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
#[cfg(feature = "rational")]
use mech_core::R64;
use mech_core::kind::Kind;
#[cfg(feature = "matrix")]
use mech_core::matrix::{Matrix, ToMatrix};
use mech_core::*;
use mech_core::{Dictionary, LegacyValue, Ref, ToValue, ValRef, ValueKind};
use mech_core::{MResult, hash_str, nodes::Kind as NodeKind, nodes::Matrix as Mat, nodes::*};
#[cfg(feature = "matrix")]
use na::DMatrix;
use std::time::Duration;

#[cfg(all(feature = "source", feature = "functions", feature = "symbol_table"))]
pub mod activation;
#[cfg(feature = "resident-ekf")]
mod efficacy;
#[cfg(feature = "source")]
pub mod expressions;
#[cfg(feature = "functions")]
pub mod function;
#[cfg(all(feature = "program", feature = "invariant_define"))]
pub mod integrity;
pub mod interpreter;
pub mod intrinsics;
#[cfg(feature = "source")]
pub mod literals;
#[cfg(feature = "source")]
pub mod mechdown;
#[cfg(feature = "source")]
pub mod patterns;
pub mod program;
#[cfg(feature = "resident-ekf")]
mod resident;
#[cfg(feature = "resident-ekf")]
#[doc(hidden)]
pub mod __gate_b_resident {
    #[cfg(feature = "runtime_bench_probes")]
    pub use crate::resident::bench::ResidentTurnProbe;
    pub use crate::resident::bench::{
        PreparedResidentTurn, ResidentEkfBatch, ResidentEkfState, ResidentTurnSummary,
    };
    pub use crate::resident::{
        FULL_WRITE_ELEMENTS, PreparedResidentFullWrite, ResidentExecutionError, ResidentFullWrite,
    };
}
#[cfg(feature = "resident-artifact")]
#[doc(hidden)]
pub mod __resident {
    pub use crate::efficacy::ekf::catalog::frozen_ekf_compiler_catalog;
    pub use crate::efficacy::ekf::closure::{
        FrozenEkfArtifactClosure, FrozenEkfArtifactClosureError, FrozenEkfCompilation,
        FrozenEkfCompilationServices, FrozenEkfConstantClosure, FrozenEkfConstraint,
        FrozenEkfInputClosure, FrozenEkfKernelNode, FrozenEkfOutputClosure, FrozenEkfPredicateNode,
        FrozenEkfStateUpdate, FrozenLiveBinding, compile_frozen_ekf_source,
    };
    pub use crate::efficacy::ekf::operation::{EkfKernel, EkfPredicate};
    pub use crate::resident::general::{
        ActivatedConstraint, ActivatedInput, ActivatedNode, ActivatedNodeIndex, ActivatedOutput,
        ActivatedPlan, ActivationFacts, CapturedSignalInput, DependencyTopology,
        PreparedResidentTurn, ReactiveInstance, ResidentActivationError, ResidentArenaSizes,
        ResidentExecutionError, ResidentReadLocation, ResidentRegion, ResidentStorageClass,
        ResidentStructuralProbe, ResidentTurnSummary, ResidentValueBorrow, ResidentWriteLocation,
        ResolvedSlot, StateArena, StateMigrationPolicy, TurnWorkspace, TypedResidentArena,
        activate,
    };
}
#[cfg(all(feature = "source", feature = "state_machines"))]
pub mod state_machines;
#[cfg(feature = "source")]
pub mod statements;
#[cfg(feature = "source")]
pub mod structures;
#[cfg(test)]
#[path = "../tests/support/mod.rs"]
pub(crate) mod test_support;
pub mod tracing;

#[doc(hidden)]
#[cfg(feature = "native-link")]
pub mod __mech_native {
    #[cfg(all(feature = "access", feature = "matrix"))]
    pub use crate::intrinsics::access::matrix::__mech_native::*;
    #[cfg(all(feature = "access", feature = "tuple"))]
    pub use crate::intrinsics::access::tuple::install_tuple_access_element;
    #[cfg(feature = "access")]
    pub use crate::intrinsics::access::{
        install_record_access_field, install_record_access_swizzle, install_table_access_swizzle,
    };
    #[cfg(feature = "assign")]
    pub use crate::intrinsics::assign::catalog::__mech_native::*;
    #[cfg(feature = "invariant_define")]
    pub use crate::intrinsics::catalog::install_integrity_constraint_marker;
    #[cfg(feature = "matrix_comprehensions")]
    pub use crate::intrinsics::catalog::install_matrix_comprehension;
    #[cfg(feature = "set_comprehensions")]
    pub use crate::intrinsics::catalog::install_set_comprehension;
    #[cfg(feature = "set")]
    pub use crate::intrinsics::catalog::install_set_define;
    #[cfg(feature = "convert")]
    pub use crate::intrinsics::convert::scalar::__mech_native::*;
    #[cfg(feature = "variable_define")]
    pub use crate::intrinsics::define::__mech_native::*;
    #[cfg(all(feature = "variable_define", feature = "matrix"))]
    pub use crate::intrinsics::define::__mech_native_matrix::*;
    #[cfg(all(feature = "f64", feature = "variable_define"))]
    pub use crate::intrinsics::define::install_variable_define_f64;
    #[cfg(feature = "matrix_horzcat")]
    pub use crate::intrinsics::horzcat::__mech_native::*;
    #[cfg(feature = "matrix_vertcat")]
    pub use crate::intrinsics::vertcat::__mech_native::*;
}

pub use mech_core::*;

#[cfg(feature = "source")]
pub use crate::expressions::*;
#[cfg(feature = "functions")]
pub use crate::function::*;
#[cfg(all(feature = "program", feature = "invariant_define"))]
pub use crate::integrity::*;
pub use crate::interpreter::*;
#[cfg(feature = "source")]
pub use crate::literals::*;
#[cfg(feature = "source")]
pub use crate::mechdown::*;
#[cfg(feature = "source")]
pub use crate::patterns::*;
pub use crate::program::*;
#[cfg(all(feature = "source", feature = "state_machines"))]
pub use crate::state_machines::*;
#[cfg(feature = "source")]
pub use crate::statements::*;
#[cfg(feature = "source")]
pub use crate::structures::*;
pub use crate::tracing::*;

#[cfg(feature = "access")]
pub use crate::intrinsics::access::*;
#[cfg(feature = "assign")]
pub use crate::intrinsics::assign::*;
#[cfg(feature = "convert")]
pub use crate::intrinsics::convert::*;
#[cfg(feature = "matrix_horzcat")]
pub use crate::intrinsics::horzcat::*;
#[cfg(feature = "table")]
pub use crate::intrinsics::table_ops::*;
#[cfg(feature = "matrix_vertcat")]
pub use crate::intrinsics::vertcat::*;
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

#[macro_export]
macro_rules! print_tree {
    ($tree:expr) => {
        #[cfg(feature = "pretty_print")]
        println!("{}", $tree.pretty_print());
        #[cfg(not(feature = "pretty_print"))]
        println!("{:#?}", $tree);
    };
}

#[macro_export]
macro_rules! print_symbols {
    ($intrp:expr) => {
        #[cfg(feature = "pretty_print")]
        println!("{}", $intrp.pretty_print_symbols());
        #[cfg(not(feature = "pretty_print"))]
        println!("{:#?}", $intrp.symbols());
    };
}

#[macro_export]
macro_rules! print_plan {
    ($intrp:expr) => {
        #[cfg(feature = "pretty_print")]
        println!("{}", $intrp.plan().pretty_print());
        #[cfg(not(feature = "pretty_print"))]
        println!("{:#?}", $intrp.plan());
    };
}

pub mod artifact;
pub use crate::artifact::*;
