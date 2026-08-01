#![cfg(all(feature = "functions", feature = "symbol_table"))]
#![forbid(unsafe_code)]
//! Statically elaborated structural dispatch for patterned activation scopes.
mod arms;
mod captures;
mod dispatch;
mod guards;
mod registers;

use captures::{
    ActivationPatternCapture, ReactiveBindingSink, commit_capture_slot, commit_proposed_captures,
    create_capture_slot_for_kind, detached, generation,
};
use dispatch::{Finalize, MatchGate, Matcher, ScopePulse, Select, UnmatchedFinalize};
use guards::{GuardFinalize, elaborate_patterned_arm_guard};
use registers::Gate;

mod errors;
mod registration;
mod validation;

use arms::{PreflightActivationArm, PreflightPatternedActivation};
pub(crate) use errors::{
    ActivationPatternArmsNonExhaustive, ActivationPatternBodyDependencyInvariant,
    ActivationPatternCaptureKindUnsupported, ActivationPatternContextEffectUnsupported,
    ActivationPatternDefinitionUnsupported, ActivationPatternGuardDependencyInvariant,
    ActivationPatternGuardMustBePure, ActivationPatternRegisterWriteUnsupported,
    ActivationPatternTransactionBoolStateUnsupported, ActivationPatternTriggerInvariant,
    ActivationPatternWildcardMustBeLast, ActivationScopeTriggerWriteUnsupported,
};
pub(crate) use registration::{activation_scope_entry_cells, elaborate_patterned_activation};
use validation::{
    preflight_patterned_activation, validate_patterned_arm_body,
    validate_patterned_guard_expression,
};

#[cfg(test)]
mod tests;
