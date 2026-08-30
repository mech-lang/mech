# Resident activation architecture contract

This directory freezes the permanent boundary between the final
`ProgramArtifact` and resident activation. Its generated contract names the
exact semantic targets and current activation owners; it deliberately contains
no migration status or source-occurrence counts.

The Gate B executor remains an explicitly named efficacy control, but no
private resident `ProgramArtifact` authority remains. The frozen ordinary EKF
source compiles through the normal parser and `ProgramCompiler`, produces equivalent
source and bytecode-v1 artifacts, activates typed resident storage, and executes
the complete 4,096-turn trace with zero steady-state allocation.

The semantic workload and committed source bytes are authoritative. The
fixture uses the current parser's hanging-call form: no whitespace immediately
after `(` or immediately before `)`, while line breaks are permitted after
commas. An independently pinned digest freezes the exact bytes; this contract
does not broaden function-call whitespace grammar.

There is no bytecode v2 before launch. Bytecode v1 evolves only if the static
`ProgramArtifact` format requires additional pre-launch fields.

The checked files in this directory include the permanent structural contract
and the frozen artifact, activation, and execution evidence. Run:

```text
python3 scripts/generate-resident-activation-contract.py --check
python3 scripts/check-resident-activation-contract.py
python3 scripts/generate-d1-contract.py --check
python3 scripts/check-d1-contract.py --contract-only
```

The D1 generator executes the source- and bytecode-derived artifacts in five
fresh processes and pins deterministic projections. The checker fails closed
on opaque or unclassified nodes, duplicate artifact authority, forbidden
resident dependencies, pointer-derived identity, per-turn semantic lookup, missing
permanent owners, obsolete executor owners, production routing, or stale evidence.

The publication boundary freezes the complete `reserve → begin → execute →
validate → summary → prepare → publish → append` order as one exact
`ordered_steps` list, in addition to its publication safety predicates.

The EKF workload contains fifteen resident-kernel nodes and three pure
integrity-predicate nodes. Each predicate produces one Boolean output. A
separate `integrity/assert` declaration reads that Boolean and has zero outputs.
`ekf/candidate-finite` consumes both corrected state and symmetrized covariance,
preserving Gate B finiteness coverage without changing artifact lowering.

The frozen D0 ancestry proof remains in `d0-boundary.json` as historical audit
evidence. The permanent generated contract is self-contained: it names the
canonical cell-binding and uninitialized-slot rules directly and has no
dependency on the retired value-system inventory.
