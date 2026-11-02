# Resident activation architecture contract

This directory freezes the permanent boundary between the final
`ProgramArtifact` and resident activation. Its generated contract names the
exact semantic targets and current activation owners; it deliberately contains
no migration status or source-occurrence counts.

The frozen ordinary EKF source compiles through the normal parser and
`ProgramCompiler`, produces equivalent source and bytecode-v1 artifacts,
activates typed resident storage, and executes the complete 4,096-turn trace
with zero steady-state allocation.

The semantic workload and committed source bytes are authoritative. The
fixture uses the current parser's hanging-call form: no whitespace immediately
after `(` or immediately before `)`, while line breaks are permitted after
commas. An independently pinned digest freezes the exact bytes; this contract
does not broaden function-call whitespace grammar.

There is no bytecode v2 before launch. Bytecode v1 evolves only if the static
`ProgramArtifact` format requires additional pre-launch fields.

The checked files in this directory contain source fixtures used by the engine
integration tests. Run:

```text
cargo test -p mech-engine --all-features --test resident_activation_contract
cargo test -p mech-engine --all-features --test resident_ekf_program_activation
cargo test -p mech-engine --all-features --test resident_ekf_program_execution
```

The resident integration tests execute source- and bytecode-derived artifacts
directly. They fail on opaque or unclassified nodes, duplicate artifact
authority, pointer-derived identity, per-turn semantic lookup, or obsolete
executor ownership without depending on a historical branch, commit sequence,
or generated milestone projection.

The publication boundary freezes the complete `reserve → begin → execute →
validate → summary → prepare → publish → append` order as one exact
`ordered_steps` list, in addition to its publication safety predicates.

The EKF workload contains fifteen resident-kernel nodes and three pure
integrity-predicate nodes. Each predicate produces one Boolean output. A
separate `integrity/assert` declaration reads that Boolean and has zero outputs.
`ekf/candidate-finite` consumes both corrected state and symmetrized covariance,
preserving finiteness coverage without changing artifact lowering.
