//! Patterned activation elaboration.
//!
//! This module is deliberately kept separate from `mechdown`: activation
//! scheduling is interpreter infrastructure rather than syntax dispatch.
//! Fixed-body scopes continue to use the established registration path while
//! patterned scopes are elaborated here as the activation graph grows.

use crate::*;

/// Describes the internal generation cells used by a patterned activation.
///
/// Keeping this descriptor independent of syntax is important: the reactive
/// plan owns these cells for the lifetime of the loaded program, so a dispatch
/// never needs to create or rebuild plan nodes.
#[derive(Clone, Debug, Default)]
pub(crate) struct PatternActivationGenerations {
    pub scope: Option<ReactiveCellId>,
    pub selection: Option<ReactiveCellId>,
    pub arms: Vec<PatternActivationArmGenerations>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct PatternActivationArmGenerations {
    pub matched: Option<ReactiveCellId>,
    pub guard_start: Option<ReactiveCellId>,
    pub complete: Option<ReactiveCellId>,
    pub pulse: Option<ReactiveCellId>,
}
