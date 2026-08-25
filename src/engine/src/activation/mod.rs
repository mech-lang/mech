#![cfg(all(feature = "functions", feature = "symbol_table"))]
#![forbid(unsafe_code)]
//! Statically elaborated structural dispatch for patterned activation scopes.
mod arms;
mod captures;
mod dispatch;
mod guards;
mod registers;

use captures::{
    ActivationPatternCapture, ReactiveBindingSink, commit_proposed_captures,
    create_capture_slot_for_kind, generation,
};
use dispatch::{Finalize, MatchGate, Matcher, ScopePulse, Select, UnmatchedFinalize};
use guards::{GuardFinalize, elaborate_patterned_arm_guard};

mod errors;
mod registration;
mod validation;

pub(crate) use errors::{
    ActivationPatternArmsNonExhaustive, ActivationPatternBodyDependencyInvariant,
    ActivationPatternCaptureKindUnsupported, ActivationPatternContextEffectUnsupported,
    ActivationPatternDefinitionUnsupported, ActivationPatternGuardDependencyInvariant,
    ActivationPatternGuardMustBePure, ActivationPatternRegisterWriteUnsupported,
    ActivationPatternTriggerInvariant, ActivationPatternWildcardMustBeLast,
    ActivationScopeTriggerWriteUnsupported,
};
pub(crate) use registration::{activation_scope_entry_cells, elaborate_patterned_activation};

#[cfg(test)]
mod tests;
