//! Runtime shell for Mech.
//!
//! `MechRuntime` is the host-facing runtime object. It owns resident program
//! activation and execution together with the system-level components:
//!
//! - ID generator
//! - store
//! - capability kernel
//! - source resolver
//! - host registry
//! - host call policy
//! - scheduler
//! - runtime config
//!
//! RuntimeContext is used as the per-operation execution envelope. It carries
//! subject/task/actor/module/transaction identity, resource budget, capabilities,
//! and accumulated events.

mod actor;
mod builder;
mod components;
#[path = "../effect_journal.rs"]
pub(crate) mod effect_journal;
mod errors;
mod events;
mod execution;
mod execution_session;
pub(crate) mod extension;
#[cfg(any(test, feature = "runtime_bench_probes"))]
pub(crate) mod gate_a_probe;
mod host;
mod id;
mod lifecycle;
mod limits;
#[cfg(feature = "source")]
mod module;
mod object;
mod operation_context;
#[cfg(feature = "resident-routing-source")]
mod program;
#[cfg(feature = "resident-external")]
#[path = "../resident_external/mod.rs"]
pub mod resident_external;
#[cfg(feature = "resident-routing")]
mod resident_program;
mod resources;
mod schedule;
mod state;
mod task;
mod transaction;

#[cfg(all(test, feature = "source"))]
mod input_tests;

#[cfg(test)]
pub(crate) mod test_support;

pub use self::builder::RuntimeBuilder;
pub use self::errors::*;
#[cfg(feature = "runtime_bench_probes")]
#[doc(hidden)]
pub use self::gate_a_probe::{GateACostSnapshot, gate_a_cost_snapshot, reset_gate_a_costs};
#[cfg(feature = "resident-routing-source")]
pub use self::program::{CompilerImportValueUnsupported, ProgramCompiler};
#[cfg(feature = "resident-routing")]
pub use self::resident_program::*;
pub use self::resources::{RuntimeResourceBinding, RuntimeResourceBindingError};
pub use self::state::MechRuntime;
pub use self::transaction::{RuntimeHealth, RuntimePoisonRecord};
