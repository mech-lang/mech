# Statement evaluation topology

`mech-interpreter` keeps statement dispatch in a small façade and assigns each
statement family to a named module. The subsystem forbids unsafe code and keeps
cross-module helpers private to the statement implementation.

## Ownership

- `statements/mod.rs` owns the `statement` dispatcher, module declarations,
  and deliberate compatibility re-exports.
- `context.rs` owns interpreter-local context declarations and context aliases.
- `variable_define.rs` owns variable definitions, initial-value detachment,
  and definition-function registration.
- `variable_assign.rs` owns whole-variable and subscript assignment lowering.
- `op_assign.rs` owns additive, subtractive, multiplicative, and divisive
  assignment lowering.
- `destructure.rs` owns tuple destructuring.
- `integrity.rs` owns invariant declarations and invariant-function
  registration.
- `kinds.rs` owns kind definitions.
- `enums.rs` owns enum definitions and enum-variant value validation.
- `state_machines.rs` owns finite-state-machine declarations.
- `errors.rs` owns statement-specific structured errors.
- `tests/` owns the behavior-grouped private statement tests described in the
  [core and interpreter test topology](../testing/core-interpreter-test-topology.md).

## Compatibility boundaries

Existing public statement functions and errors remain available through
`mech_interpreter::statements::<name>` and the crate-root re-exports. Moving an
implementation into a named leaf module does not add that module's private
helpers to the public API.

Production modules import their dependencies explicitly. Shared implementation
details use the narrowest sibling visibility that permits coordination:
variable detachment is shared with integrity and state-machine declarations,
and enum matching is shared with variable definitions.

## Safety boundary

The statement directory declares `#![forbid(unsafe_code)]`. Every statement
leaf is covered by that directory-level prohibition; the split does not add an
unsafe-boundary allowlist entry.
