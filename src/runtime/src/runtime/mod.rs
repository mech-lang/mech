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
#[cfg(any(test, feature = "runtime_bench_probes"))]
pub(crate) mod cost_probe;
#[path = "../effect_journal.rs"]
pub(crate) mod effect_journal;
mod errors;
mod events;
mod execution_session;
pub(crate) mod extension;
mod host;
mod id;
mod lifecycle;
mod limits;
#[cfg(feature = "source")]
mod module;
mod object;
mod operation_context;
pub mod program;
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
#[cfg(feature = "runtime_bench_probes")]
#[doc(hidden)]
pub use self::cost_probe::{RuntimeCostSnapshot, reset_runtime_costs, runtime_cost_snapshot};
pub use self::errors::*;
#[cfg(feature = "resident-external")]
pub use self::program::*;
pub use self::state::MechRuntime;
pub use self::transaction::{RuntimeHealth, RuntimePoisonRecord};
