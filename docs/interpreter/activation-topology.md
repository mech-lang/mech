# Activation coordination topology

Patterned activation is a statically elaborated interpreter subsystem. Its
public façade remains `interpreter::activation`, while implementation files
separate registration-time validation and plan construction from runtime arm
dispatch.

## Ownership

- `mod.rs` is the stable façade. It declares the subsystem, retains the
  existing crate-visible entry points and errors, and deliberately re-exports
  only the private items required by sibling implementation and test modules.
- `registration.rs` owns activation registration scopes and static reactive
  plan construction. It joins preflight arm metadata, capture proposals,
  guards, dispatch nodes, register gates, and arm bodies into one plan.
- `validation.rs` owns arm order, exhaustiveness, wildcard placement, pattern
  compilation, guard purity preflight, and validation of allowed body forms.
- `arms.rs` owns the preflight arm metadata that connects compiled patterns to
  their arm-local capture cells.
- `captures.rs` owns proposed and committed capture cells, capture-kind
  allocation, sampled composite values, pulse-generation transaction state,
  and atomic capture commits.
- `guards.rs` owns guard expression elaboration, sampled capture
  dependencies, purity enforcement, and guard finalization.
- `dispatch.rs` owns runtime pattern matching, eligibility finalization,
  source-order selection, and selected-arm pulse behavior.
- `registers.rs` owns staged register-write validation, arm-body node
  registration, sampled capture dependencies, and the selected-arm gate that
  commits capture proposals before pulsing the body.
- `errors.rs` owns activation-specific structured error kinds.
- `tests/` groups private unit tests by registration, dispatch,
  exhaustiveness, guard, capture, register, and rollback behavior.

## Boundaries

The existing `activation_scope_entry_cells` and
`elaborate_patterned_activation` entry points remain available at their
original `crate::activation` paths. Activation-specific errors also retain
their original qualified paths. The split introduces no coordinator object or
`ActivationEngine`; registration stays a set of narrow functions over the
existing interpreter and reactive plan.

Generic structural pattern compilation and matching remain in `patterns.rs`.
Generic reactive-plan registration, scheduling, transaction state, and
register commit mechanics remain in `mech-core`. Statement lowering remains in
the statement subsystem; activation owns only patterned-arm coordination after
lowering.

Every production activation module forbids unsafe code through the façade and
uses explicit imports. New behavior belongs in the narrowest owning module;
cross-module access should remain private unless it is part of the existing
crate-visible activation façade.

For test placement and the distinction between statement lowering and runtime
activation dispatch, see the
[core and interpreter test topology](../testing/core-interpreter-test-topology.md).
