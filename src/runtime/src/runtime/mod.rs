//! Runtime shell for Mech.
//!
//! `MechRuntime` is the host-facing runtime object. It wraps the current
//! program/interpreter layer and owns the system-level components:
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
mod external;
#[cfg(any(test, feature = "runtime_bench_probes"))]
pub(crate) mod gate_a_probe;
mod host;
mod id;
mod lifecycle;
mod limits;
mod live_state;
#[cfg(feature = "source")]
mod module;
mod object;
mod operation_context;
#[cfg(feature = "resident-external")]
#[path = "../resident_external/mod.rs"]
pub mod resident_external;
mod resources;
mod schedule;
mod state;
mod task;
mod transaction;

#[cfg(test)]
mod input_tests;

#[cfg(test)]
pub(crate) mod test_support;

pub use self::builder::RuntimeBuilder;
pub use self::errors::*;
pub use self::external::{
    ExternalOperationArityMismatch, ExternalRequirementCanonicalizationOverflow,
    ExternalRequirementCatalog, ExternalRequirementDigestCollision, hidden_external_operation_name,
};
#[cfg(feature = "runtime_bench_probes")]
#[doc(hidden)]
pub use self::gate_a_probe::{GateACostSnapshot, gate_a_cost_snapshot, reset_gate_a_costs};
pub use self::resources::{RuntimeResourceBinding, RuntimeResourceBindingError};
pub use self::state::{MechRuntime, RuntimeExecutionMode};
pub use self::transaction::{RuntimeHealth, RuntimePoisonRecord};
