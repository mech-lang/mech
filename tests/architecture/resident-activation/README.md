# Resident activation architecture gate

D0 froze the boundary between the final `ProgramArtifact` and resident
activation without changing production behavior. D1 now implements the exact
ordinary-EKF vertical slice behind the `resident-ekf-artifact` efficacy
feature; general production routing remains unchanged.

The Gate B executor remains an explicitly named efficacy control, but no
private resident `ProgramArtifact` authority remains. The frozen ordinary EKF
source compiles through the normal parser and `MechProgram`, produces equivalent
source and bytecode-v1 artifacts, activates typed resident storage, and executes
the complete 4,096-turn trace with zero steady-state allocation.

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

The checked files in this directory include the frozen D0 contract and the
mechanical D1 artifact, activation, and execution projections. Run:

```text
python3 scripts/generate-resident-activation-contract.py --check
python3 scripts/check-resident-activation-contract.py
python3 scripts/generate-d1-contract.py --check
python3 scripts/check-d1-contract.py --contract-only
```

The D1 generator executes the source- and bytecode-derived artifacts in five
fresh processes and pins deterministic projections. The phase checker fails
closed on opaque or unclassified nodes, duplicate artifact authority, legacy
resident dependencies, pointer-derived identity, per-turn semantic lookup,
production routing, stale evidence, or global migration overclaims.

The publication boundary freezes the complete `reserve → begin → execute →
validate → summary → prepare → publish → append` order as one exact
`ordered_steps` list, in addition to its publication safety predicates.

The EKF workload contains fifteen resident-kernel nodes and three pure
integrity-predicate nodes. Each predicate produces one Boolean output. A
separate `integrity/assert` declaration reads that Boolean and has zero outputs.
`ekf/candidate-finite` consumes both corrected state and symmetrized covariance,
preserving Gate B finiteness coverage without changing artifact lowering.

The frozen D0 inventory exception is the exact `resident_activation_contract`
compilation unit rooted at and reaching only
`src/engine/tests/resident_activation_contract.rs`, one addition of that path
to the Rust-file inventory, and the `mech` 912→913 plus `mech-engine` 143→144
file-count changes. All other inventory content, including every legacy
occurrence and count, remains byte-for-byte fixed by blob
`5b5fd877143cba1d7945d850405a45975930e6f4`.

D1 records only vertical-slice progress: one admitted artifact and two migrated
state slots. Both global D migration targets remain incomplete, no legacy target
is removed, and none of the 488 inventoried legacy occurrences is claimed as
migrated.
