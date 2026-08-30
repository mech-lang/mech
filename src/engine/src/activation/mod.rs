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
    create_capture_slot_for_schema, generation,
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
mod tests {
    use super::{Finalize, GuardFinalize, Matcher, Select, UnmatchedFinalize};
    use crate::{CompiledPattern, FunctionValueRepresentation, MechFunctionImpl, ValueCell};

    fn contains_representation(
        ports: &[mech_core::FunctionStatePort<'_>],
        expected: FunctionValueRepresentation,
    ) -> bool {
        ports.iter().any(|port| port.representation() == expected)
    }

    #[test]
    fn activation_dispatch_nodes_checkpoint_every_hidden_canonical_state_cell() {
        let matched = ValueCell::from_exact(false).unwrap();
        let matcher = Matcher {
            pattern: CompiledPattern::Wildcard,
            trigger: ValueCell::unit(),
            expression_values: Vec::new(),
            captures: Vec::new(),
            matched: matched.clone(),
            out: ValueCell::from_exact(1_usize).unwrap(),
        };
        let matcher_ports = matcher.transaction_state_ports().unwrap().unwrap();
        assert_eq!(matcher_ports.len(), 2);
        assert!(contains_representation(
            &matcher_ports,
            FunctionValueRepresentation::Bool
        ));

        let eligible = ValueCell::from_exact(false).unwrap();
        let finalize = Finalize {
            matched: matched.clone(),
            eligible: eligible.clone(),
            out: ValueCell::from_exact(1_usize).unwrap(),
        };
        let finalize_ports = finalize.transaction_state_ports().unwrap().unwrap();
        assert_eq!(finalize_ports.len(), 2);
        assert!(contains_representation(
            &finalize_ports,
            FunctionValueRepresentation::Bool
        ));

        let unmatched = UnmatchedFinalize {
            matched,
            eligible: ValueCell::from_exact(false).unwrap(),
            out: ValueCell::from_exact(1_usize).unwrap(),
        };
        assert_eq!(
            unmatched.transaction_state_ports().unwrap().unwrap().len(),
            2
        );

        let guard = GuardFinalize {
            guard: ValueCell::from_exact(false).unwrap(),
            eligible: ValueCell::from_exact(false).unwrap(),
            out: ValueCell::from_exact(1_usize).unwrap(),
        };
        assert_eq!(guard.transaction_state_ports().unwrap().unwrap().len(), 2);

        let select = Select {
            eligible: vec![eligible],
            selected: ValueCell::from_exact(usize::MAX).unwrap(),
            out: ValueCell::from_exact(1_usize).unwrap(),
        };
        let select_ports = select.transaction_state_ports().unwrap().unwrap();
        assert_eq!(select_ports.len(), 2);
        assert!(contains_representation(
            &select_ports,
            FunctionValueRepresentation::Index
        ));
    }
}
