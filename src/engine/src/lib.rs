#![cfg_attr(all(feature = "no_std", not(feature = "std")), no_std)]
#![feature(where_clause_attrs)]

#[cfg(feature = "matrix")]
extern crate nalgebra as na;

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

#[cfg(feature = "semantic-compiler")]
pub use mech_core::{
    CompileCtx, CompiledBytecode, CompiledInstructionRole, CompiledIntegrityConstraint,
    CompiledNodeKind, CompiledSymbolDefinition,
};

#[cfg(all(
    feature = "semantic-compiler",
    feature = "functions",
    feature = "symbol_table"
))]
pub mod activation;
#[cfg(feature = "resident-ekf")]
mod efficacy;
#[cfg(feature = "semantic-compiler")]
pub mod expressions;
#[cfg(feature = "functions")]
pub mod function;
#[cfg(all(feature = "semantic-compiler", feature = "invariant_define"))]
pub mod integrity;
#[cfg(feature = "semantic-compiler")]
mod interpreter;
#[cfg(all(feature = "semantic-compiler", feature = "invariant_define"))]
pub(crate) use interpreter::InterpreterRef;
#[cfg(feature = "semantic-compiler")]
pub(crate) use interpreter::{Interpreter, InterpreterExecution, RuntimeContextBinding};
pub mod intrinsics;
#[cfg(feature = "semantic-compiler")]
pub mod literals;
#[cfg(feature = "semantic-compiler")]
pub mod mechdown;
#[cfg(feature = "semantic-compiler")]
pub mod patterns;
#[cfg(any(
    all(feature = "subscript_formula", feature = "semantic-compiler"),
    feature = "resident-artifact"
))]
mod portable_index;
pub mod program;
#[cfg(all(feature = "resident-ekf", not(feature = "resident-artifact")))]
mod resident;
#[cfg(feature = "resident-artifact")]
pub mod resident;
#[cfg(feature = "resident-artifact")]
mod resident_value_adapter;
#[cfg(feature = "resident-ekf")]
#[doc(hidden)]
pub mod __gate_b_resident {
    pub use crate::resident::ResidentCandidateExecutionError as ResidentExecutionError;
    #[cfg(feature = "runtime_bench_probes")]
    pub use crate::resident::bench::ResidentTurnProbe;
    pub use crate::resident::bench::{
        PreparedResidentTurn, ResidentEkfBatch, ResidentEkfState, ResidentTurnSummary,
    };
    pub use crate::resident::{FULL_WRITE_ELEMENTS, PreparedResidentFullWrite, ResidentFullWrite};
}
#[cfg(feature = "resident-artifact")]
#[doc(hidden)]
pub mod __resident {
    #[cfg(feature = "semantic-compiler")]
    pub use crate::efficacy::ekf::catalog::frozen_ekf_compiler_catalog;
    #[cfg(feature = "semantic-compiler")]
    pub use crate::efficacy::ekf::closure::{
        FrozenEkfArtifactClosure, FrozenEkfArtifactClosureError, FrozenEkfCompilation,
        FrozenEkfCompilationServices, FrozenEkfConstantClosure, FrozenEkfConstraint,
        FrozenEkfInputClosure, FrozenEkfKernelNode, FrozenEkfOutputClosure, FrozenEkfPredicateNode,
        FrozenEkfStateUpdate, FrozenLiveBinding, compile_frozen_ekf_source,
    };
    pub use crate::efficacy::ekf::operation::{EkfKernel, EkfPredicate};
    pub use crate::resident::general::{
        ActivatedConstraint, ActivatedExternalNode, ActivatedInput, ActivatedInputSource,
        ActivatedKernelNode, ActivatedNodeIndex, ActivatedOutput, ActivatedPlan, ActivatedTurnStep,
        ActivationFacts, CapturedSignalInput, CapturedValueInput, DependencyTopology,
        PreparedResidentTurn, ReactiveInstance, ResidentActivationError, ResidentActivationOptions,
        ResidentArenaSizes, ResidentEffectIntent, ResidentEffectIntentIter,
        ResidentEffectIntentView, ResidentExecutionError, ResidentExternalAdmission,
        ResidentExternalPublicationAuthority, ResidentIntegrityMode, ResidentReadLocation,
        ResidentRegion, ResidentStorageClass, ResidentStructuralProbe, ResidentTurnSummary,
        ResidentValueBorrow, ResidentWriteLocation, ResolvedSlot, StateArena,
        StateMigrationMapping, StateMigrationPolicy, TurnWorkspace, TypedResidentArena, activate,
        activate_external, activate_with_options,
    };
}
#[cfg(all(feature = "semantic-compiler", feature = "state_machines"))]
pub mod state_machines;
#[cfg(feature = "semantic-compiler")]
pub mod statements;
#[cfg(feature = "semantic-compiler")]
pub mod structures;
#[cfg(all(test, feature = "semantic-compiler", feature = "functions"))]
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
    #[cfg(all(feature = "access", feature = "native-plan"))]
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
    #[cfg(feature = "matrix_horzcat")]
    pub use crate::intrinsics::catalog::install_value_horizontal_concatenation;
    #[cfg(feature = "matrix_vertcat")]
    pub use crate::intrinsics::catalog::install_value_vertical_concatenation;
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
    #[cfg(feature = "table")]
    pub use crate::intrinsics::table_ops::__mech_native::*;
    #[cfg(feature = "matrix_vertcat")]
    pub use crate::intrinsics::vertcat::__mech_native::*;
}

pub use mech_core::*;

#[cfg(feature = "semantic-compiler")]
pub use crate::expressions::*;
#[cfg(feature = "functions")]
pub use crate::function::*;
#[cfg(all(feature = "semantic-compiler", feature = "invariant_define"))]
pub use crate::integrity::*;
#[cfg(feature = "semantic-compiler")]
pub use crate::literals::*;
#[cfg(feature = "semantic-compiler")]
pub use crate::mechdown::*;
#[cfg(feature = "semantic-compiler")]
pub use crate::patterns::*;
pub use crate::program::*;
#[cfg(all(feature = "semantic-compiler", feature = "state_machines"))]
pub use crate::state_machines::*;
#[cfg(feature = "semantic-compiler")]
pub use crate::statements::*;
#[cfg(feature = "semantic-compiler")]
pub use crate::structures::*;
#[cfg(any(feature = "trace", feature = "state_machines"))]
pub use crate::tracing::*;

#[cfg(all(feature = "access", feature = "map"))]
pub use crate::intrinsics::access::map::*;
#[cfg(all(feature = "access", feature = "matrix"))]
pub use crate::intrinsics::access::matrix::*;
#[cfg(all(feature = "access", feature = "record"))]
pub use crate::intrinsics::access::record::*;
#[cfg(all(feature = "access", feature = "string", feature = "semantic-compiler"))]
pub use crate::intrinsics::access::string::*;
#[cfg(all(feature = "access", feature = "table"))]
pub use crate::intrinsics::access::table::*;
#[cfg(all(feature = "access", feature = "tuple"))]
pub use crate::intrinsics::access::tuple::*;
#[cfg(feature = "access")]
pub use crate::intrinsics::access::{AccessColumn, AccessRange, AccessScalar, AccessSwizzle};
#[cfg(feature = "assign")]
pub use crate::intrinsics::assign::EmptyAssignmentNotBytecodeCompilable;
#[cfg(all(feature = "assign", feature = "map"))]
pub use crate::intrinsics::assign::map::*;
#[cfg(all(feature = "assign", feature = "matrix"))]
pub use crate::intrinsics::assign::matrix::*;
#[cfg(all(feature = "assign", feature = "record"))]
pub use crate::intrinsics::assign::record::*;
#[cfg(all(feature = "assign", feature = "table"))]
pub use crate::intrinsics::assign::table::*;
#[cfg(all(feature = "assign", feature = "tuple"))]
pub use crate::intrinsics::assign::tuple::*;
#[cfg(all(feature = "assign", feature = "semantic-compiler"))]
pub use crate::intrinsics::assign::{AddAssignValue, AssignColumn, AssignValue};
#[cfg(feature = "matrix_horzcat")]
pub use crate::intrinsics::horzcat::{HorizontalConcatenateDimensionMismatchError, MatrixHorzCat};
#[cfg(feature = "table")]
pub use crate::intrinsics::table_ops::{
    TableFullOuterJoin, TableInnerJoin, TableLeftAntiJoin, TableLeftOuterJoin, TableLeftSemiJoin,
    TableRightOuterJoin,
};
#[cfg(feature = "matrix_vertcat")]
pub use crate::intrinsics::vertcat::{MatrixVertCat, VerticalConcatenateDimensionMismatch};
#[cfg(feature = "semantic-compiler")]
pub fn load_stdkinds(kinds: &mut NamedSchemaTable) {
    // `ix` is the canonical spelling used by value formatting; `index` is the
    // long-form alias. Both names denote the same one-based scalar schema.
    kinds.insert(hash_str("ix"), SchemaBody::Index);
    kinds.insert(hash_str("index"), SchemaBody::Index);
    #[cfg(feature = "u8")]
    kinds.insert(
        hash_str("u8"),
        SchemaBody::UnsignedInteger(IntegerWidth::W8),
    );
    #[cfg(feature = "u16")]
    kinds.insert(
        hash_str("u16"),
        SchemaBody::UnsignedInteger(IntegerWidth::W16),
    );
    #[cfg(feature = "u32")]
    kinds.insert(
        hash_str("u32"),
        SchemaBody::UnsignedInteger(IntegerWidth::W32),
    );
    #[cfg(feature = "u64")]
    kinds.insert(
        hash_str("u64"),
        SchemaBody::UnsignedInteger(IntegerWidth::W64),
    );
    #[cfg(feature = "u128")]
    kinds.insert(
        hash_str("u128"),
        SchemaBody::UnsignedInteger(IntegerWidth::W128),
    );
    #[cfg(feature = "i8")]
    kinds.insert(hash_str("i8"), SchemaBody::SignedInteger(IntegerWidth::W8));
    #[cfg(feature = "i16")]
    kinds.insert(
        hash_str("i16"),
        SchemaBody::SignedInteger(IntegerWidth::W16),
    );
    #[cfg(feature = "i32")]
    kinds.insert(
        hash_str("i32"),
        SchemaBody::SignedInteger(IntegerWidth::W32),
    );
    #[cfg(feature = "i64")]
    kinds.insert(
        hash_str("i64"),
        SchemaBody::SignedInteger(IntegerWidth::W64),
    );
    #[cfg(feature = "i128")]
    kinds.insert(
        hash_str("i128"),
        SchemaBody::SignedInteger(IntegerWidth::W128),
    );
    #[cfg(feature = "f32")]
    kinds.insert(hash_str("f32"), SchemaBody::FloatingPoint(FloatWidth::W32));
    #[cfg(feature = "f64")]
    kinds.insert(hash_str("f64"), SchemaBody::FloatingPoint(FloatWidth::W64));
    #[cfg(feature = "c64")]
    kinds.insert(hash_str("c64"), SchemaBody::Complex(FloatWidth::W64));
    #[cfg(feature = "r64")]
    kinds.insert(hash_str("r64"), SchemaBody::Rational64);
    #[cfg(feature = "string")]
    kinds.insert(hash_str("string"), SchemaBody::String);
    #[cfg(feature = "bool")]
    kinds.insert(hash_str("bool"), SchemaBody::Bool);
}

#[cfg(all(test, feature = "semantic-compiler"))]
mod standard_kind_tests {
    use super::*;

    #[test]
    fn ix_and_index_are_aliases() {
        let mut kinds = NamedSchemaTable::default();
        load_stdkinds(&mut kinds);
        assert_eq!(kinds.get(&hash_str("ix")), Some(&SchemaBody::Index));
        assert_eq!(kinds.get(&hash_str("index")), Some(&SchemaBody::Index));
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

#[cfg(any(
    feature = "artifact-codec",
    feature = "resident-artifact",
    feature = "semantic-compiler"
))]
pub mod artifact;
#[cfg(any(
    feature = "artifact-codec",
    feature = "resident-artifact",
    feature = "semantic-compiler"
))]
pub use crate::artifact::*;
