//! Managed realization of deterministic R5 memory plans.
//!
//! This module owns process-local allocation identity, reservations, leases,
//! publication records, and reclamation. It consumes R5 plan records through
//! a borrowed [`RuntimePlanView`] and never derives a second physical policy.

mod access;
mod allocation;
mod domain;
mod error;
mod identity;
mod payload;
mod transaction;

pub use self::access::*;
pub use self::domain::*;
pub use self::error::*;
pub use self::identity::*;
pub use self::payload::*;
pub use self::transaction::*;

pub(crate) use self::allocation::*;
