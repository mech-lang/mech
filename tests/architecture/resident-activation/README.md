# Resident activation architecture gate

D0 changes no production behavior. It freezes the boundary between the final
`ProgramArtifact` and future resident activation.

The private Gate B executor remains an efficacy control. D1 must replace the
private EKF artifact with activation from the final `ProgramArtifact`. The
ordinary EKF source is frozen here, and D1 must produce zero `LegacyOpaque`
contracts. D1 must not route general production execution.

The semantic workload and committed source bytes are authoritative. The
fixture uses the current parser's hanging-call form: no whitespace immediately
after `(` or immediately before `)`, while line breaks are permitted after
commas. The exact bytes become frozen in D0 commit 3. D0 does not broaden
function-call whitespace grammar.

D2 generalizes storage and shapes. D3 adds observations, effects, and
transactional participants. D4 routes supported production programs. D5
closes legacy runtime storage. Final cutover deletes dead legacy types only.

There is no bytecode v2 before launch. D evolves bytecode v1 only if the
static `ProgramArtifact` format requires additional pre-launch fields. D0
itself changes no bytecode.

The checked files in this directory are architecture and workload contracts,
not claims that resident activation already exists. Run:

```text
python3 scripts/generate-resident-activation-contract.py --check
python3 scripts/check-resident-activation-contract.py
```

The generator derives source integrity and the Phase-D migration projection.
The checker fails closed on production changes, duplicate artifact authority,
new legacy dependencies, pointer-derived identity, per-turn semantic lookup,
stale Gate B evidence, or premature migration claims.

The publication boundary freezes the complete `reserve → begin → execute →
validate → summary → prepare → publish → append` order as one exact
`ordered_steps` list, in addition to its publication safety predicates.

The EKF workload contains fifteen resident-kernel nodes and three pure
integrity-predicate nodes. Each predicate produces one Boolean output. A
separate `integrity/assert` declaration reads that Boolean and has zero outputs.
`ekf/candidate-finite` consumes both corrected state and symmetrized covariance,
preserving Gate B finiteness coverage without changing artifact lowering.

The only C0 inventory exception is the exact `resident_activation_contract`
compilation unit rooted at and reaching only
`src/engine/tests/resident_activation_contract.rs`, one addition of that path
to the Rust-file inventory, and the `mech` 912→913 plus `mech-engine` 143→144
file-count changes. All other inventory content, including every legacy
occurrence and count, remains byte-for-byte fixed by blob
`5b5fd877143cba1d7945d850405a45975930e6f4`.
