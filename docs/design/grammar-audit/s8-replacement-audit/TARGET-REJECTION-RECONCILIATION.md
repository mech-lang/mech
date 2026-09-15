# Configured target admission: actual rejection boundary

The frozen audit does **not** establish that pure `ProgramCompiler::compile_document`
accepting a semantic program, followed by a clean configured-resident activation
rejection, is an early-admission defect. Those are two distinct boundaries in the
current API. The observed C32/power/matrix-product/layout failures alone cannot establish a
compiler rejection-stage defect. They still establish finite unavailable
capabilities. No accepted S8 exclusion for C32 arithmetic or general computed-mask
admission was established by this audit. Current rejection safety does not close
the positive milestone capability.

## Pure artifact production

`ProgramCompiler::compile_document` delegates to the view (`src/runtime/src/runtime/program/compiler.rs:182`),
which obtains a canonical artifact and wraps it in `ProgramCompilationProduct`
(`compiler.rs:688`). `ProgramCompilationProduct::from_canonical_artifact` encodes
the artifact and retains its semantic graph/bytecode; its native instruction
sidecars are deliberately empty (`src/engine/src/program/compiler_planning.rs:96`).
Neither returned type contains an `ActivatedPlan` or promises successful physical
kernel binding to every configured resident target.

The shared validation helper explicitly skips temporary execution when there are
no writes and no planning values (`compiler.rs:884`), and also when a pure plan
still has open live inputs (`compiler.rs:900`). This is intentional source policy,
not a missing generic call hidden behind successful test counts. The pure audit
witnesses compile first, then call `activate` using the same catalog in a separate
step (`src/runtime/tests/s8_replacement_gap_audit.rs:63`).

Resident activation owns physical binding: `activate` returns
`Result<ReactiveInstance, ResidentActivationError>`
(`src/engine/src/resident/general/mod.rs:1640`). Its failure type explicitly has
`MissingResidentFactory` and `KernelBind` variants (`general/mod.rs:1601`), and
kernel selection maps unavailable factories and rejected layouts into those
errors (`general/mod.rs:3931`). The retained contract distinguishes semantic
`ProgramArtifact` authority from physical `ActivatedPlan` authority
(`docs/design/program-artifact-resident-activation.md:22`).

## Resource planning and effectful compiler paths

For writes or closed planning-value evaluation, the same helper creates a
temporary artifact, activates it, prepares a candidate, preflights provider
payloads, and aborts it (`compiler.rs:904`). A binding failure propagates from
that temporary activation before candidate preparation or payload preflight.
`preflight_canonical_effect_payloads` invokes the planning hook
`resources.plan_write` (`compiler.rs:2742`); it does not publish a runtime turn.
Static evaluation and detached initializer methods separately execute their
required closed projections and can therefore reject during compilation.

The existing five-route regression
`canonical_ordinary_and_interactive_compilation_preflight_payloads_without_live_effects`
(`src/runtime/src/runtime/program/query_tests/source.rs:292`) checks ordinary,
interactive, rooted, interactive-rooted, and supplied-input compilation. It
permits effect-free provider planning, rejects the disallowed payload, and
asserts zero live provider prepares and deliveries. It is evidence for that
planning contract, not a new all-overload physical-admission test.

## Finite accounting correction

1. Record a semantically valid but unsupported pure physical operation as a
   configured-target activation rejection, with the exact `KernelBind` or
   `MissingResidentFactory` result. Do not label compilation success itself a
   confirmed defect or require rejection before `activate` without a stronger
   product API contract.
2. Keep supported target cells as positive execution obligations. An independent
   maintained profile/operation contract is necessary to classify an absent
   kernel as a defect; factory absence alone neither creates that promise nor
   permits silently narrowing the language.
3. Keep effectful compilation and closed initialization/static evaluation as
   separate negative obligations: unsupported binding must fail before live
   preparation, publication or delivery. The existing pure observations do not
   demonstrate a violation of this boundary. Effect-free planning hooks are
   allowed and must not be mislabeled host execution effects.

Accordingly, G02 is not demonstrated as an early-admission bug by these pure
witnesses. It remains an identifier for current target rejection contracts and
unimplemented positive capabilities until scope acceptance assigns completion
or an explicit exclusion. A new compiler-stage rejection is not a substitute for
implementing a promised capability. This review adds no production changes or
Rust tests and runs no Cargo commands.


## Two independent acceptance axes

`target-rejection-cells.tsv` records both current-target behavior and positive
milestone capability. Its 25 G02 pure fixtures group into 11 C32 basic/reduction/
selected-update manifestations, 10 power manifestations, three matrix-product
manifestations, and one f32 binary-broadcast manifestation. The exact positive
fixture outputs are preserved; corrected current-target errors cannot erase them.
The owners are the corresponding math/stats/matrix physical kernels and resident
binders. Kernel absence is evidence of unavailable implementation, not evidence
that S8 accepted the omission. Scope acceptance remains pending.

All 25 currently reject at `activation` with `KernelBind`: 24 report
`UnsupportedLayout`, and `numeric-c32-mul` reports `UnsupportedContract`.
`target-rejection-oracle-updates.json` supplies these 25 exact frozen observations
plus the G17 and G18 observations: 27 current rejection oracles, each alongside
its preserved positive expected values and its unimplemented capability group.
The published-contract rerun checks and matches all 27 exact current rejections
for both direct and decoded artifacts. These matches must not be reported as
completed positive capabilities. Eight named supported numeric/
layout controls and exact C32 live transport are separate evidence.

Effectful, supplied-value, static-initializer and open-input unsupported-target
cells are explicit unrun responsibilities. The existing five-route effect-free
preflight regression remains a positive policy control, not evidence that those
unsupported-operation cases were executed.

## G17 computed-mask facts

`logical-read` is `x := [1 2 3]; mask := x > 1; x[mask]`. Its positive result is
independently `[2,3]` with shape1x2. The current probe compiles it and calls
`activate` with empty facts, receiving `UnresolvedShape { slot: CellSlotId(1) }`.

`ActivationFacts` contains selected slot shapes (`general/mod.rs:1186`). The
resident derivation recognizes constant logical selectors and concatenations of
known selectors (`general/mod.rs:2663,2693`), but does not evaluate arbitrary
computed masks to derive population. Q09 explicitly requires a fact for its live
mask and validates population changes atomically. The production loader supplies
default facts for pure programs (`loading.rs:399`) and derives provider-owned
observation shapes for external programs (`loading.rs:650`). Its physical
preflight can reject `SemanticUnsupported` before installing a program.

This explains correct current rejection and its actual owner. It does **not**
establish an accepted S8 exclusion for computed masks, nor prove that the
positive source-to-production capability is complete. General computed-mask
admission remains a finite unimplemented shape-derivation/handoff capability
(CAP-MASK), with a preserved positive `[2,3]` witness and scope acceptance
pending. The ledger separately names current empty-fact rejection, same-artifact
supplied-population success, the existing Q09 fixed-population control, and the
production loader's rejection path. The new supplied-population and production-loader routes are not credited
as executed merely because low-level activation returned an error.


## G18 downstream collection layout

The control owner's `CONTROL-TARGET-RECONCILIATION.md` establishes both axes.
`dynamic-concat` is `xs := [1 2]; y := [x | x <- xs]; [y y]`. Its preserved
positive result is an F64 matrix of shape1x4 and values `[1,2,1,2]` on both turns.
The current exact error is `KernelBind { node: NodeId(1), error: UnsupportedLayout }`
at activation. The producer is accepted; the downstream fixed-layout constructor
cannot bind its resulting layout. That is a finite unavailable consumer capability,
not a demonstrated late-rejection bug and not an accepted milestone exclusion.

The ledger now has 46 responsibility cells. G18 contributes the current closed
concat witness and two explicit unrun changing-cardinality capabilities: concat
producing shapes1x4/1x2/1x0, and transpose producing2x1/1x1/0x1 for the three inputs
`[1,2]`, `[-1,2]`, `[-1,-2]`. These preserve the control review's exact independent
positive oracles. The missing owner is resident downstream layout/shape handling;
G07's nested control representation remains separate.

Current-target oracles for `logical-read` and `dynamic-concat` match the exact
direct and decoded rejections in `s8-audit-published-contracts.log`. Their CAP-MASK
and CAP-DOWNSTREAM positive gates remain failing until implemented or an explicit
scope decision accepts a precise exclusion. The normative source boundary
permits resident availability to remain an activation concern
(`canonical-source-boundary.mec:178-182`), while the S8 rehearsal prohibits silently
turning an uncovered row into unsupported scope
(`syntax-s8-execution-rehearsal.mec:275-278`). Both constraints apply together.

To exercise positive capability separately from current rejection correctness,
set both `MECH_AUDIT_REQUIRE_PASS=1` and `MECH_AUDIT_REQUIRE_CAPABILITY=1` with the
same `MECH_AUDIT_CASE` selector. A matching rejection can never satisfy that gate.
