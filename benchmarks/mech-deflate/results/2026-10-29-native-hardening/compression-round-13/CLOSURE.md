# Completed private mixed-result slice

The compiler now supports a private flat numeric-leaf result containing one
current unsigned-array owner and supported scalar fields. Unchanged
`huff-assign-symbols` executes this path beneath its ordinary `huff-assign-lengths`
parent. **247 scoped Rust tests pass**, with twelve static contracts, five late
applicable checks, and twelve fresh paired scalar timing/allocation controls.

The unchanged eight-byte level-12 comparison is **253.03× → 250.01× Roc**
(Roc = 1×), **1.19% less elapsed time**. This is a small observed gain. The large
component reductions do not establish a large complete-program improvement.
Only three primary observations per side were taken; no statistical confidence
or larger-input throughput claim is made. The parity objective remains open.

## Revisions and preserved application

| Role | Exact revision |
| --- | --- |
| Round base / previously published checkpoint | `1087e7b8160894f542b81d46b887eb32a450125c` |
| Current-head regression and source requirement | `e04ea4d6287c25ed8ecef9581b39315e0e41b75e` |
| Measured production compiler | `1ec498a67fc862bac39188fdf5040eaacf6e954c` |
| Paired timing and separate profile committed | `f8d8bcf07022c9ce9409d7ba3547b43053b5ba7c` |
| Completed qualification and real-buffer ownership evidence | `4410cf95730c69f897eda24aa46c9bb6c756075a` |
| Previous measured write compiler | `d105f884bc79d464a205b3022da4abdc00da41aa` |
| PR parent | `eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b` |

The patch is on draft [#973](https://github.com/mech-lang/mech/pull/973), stacked
#934 → #935 → #939 → #973. Subsequent qualification/documentation commits change
no production code after the measured compiler. The final published review head
is recorded in the PR and publication response; no self-referential commit hash
is embedded in this file.

The three pending write-specific checks in the brief were already completed in
round 12 at `e7767697e1b1bdc6e8013a499a1cecbb80319a22`. They were retained and
regression-checked here, rather than reopened as separate work packages.

The 25-component assembly remains 160992 bytes, SHA-256
`345324ce551aaa2485dcd0fe4262da96e660d6b31f5f772bff89b034e9b1997f`.
The timing caller remains 160857 bytes, SHA-256
`331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`.
Its 4138627-byte canonical artifact remains
`a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`.
No fixture source changed. The real 262144-entry U16 tree, 1500513-word U32 cache,
depth 300, nice length 258, EOF rules, tables, allocation sites, six Huffman
initializers and timing boundaries are unchanged. Mech performs compression.

Default build receipts have identical Core, Engine, Syntax, Stdlib, serde,
serde_json and miniz_oxide library hashes to the prior compiler. Only GPU differs.
The canonical producer is unchanged; the round-12 source reproduction is retained,
not claimed as a fresh producer execution here. Original/decoded execution and
fresh assembly-byte preservation are covered in this round.

## Result recipe and execution

`prepare/numeric_results.rs` derives an explicit `Scalar`, `Array`, or `Frame`
recipe using the existing selected factories, exact schemas and connected private
frame facts. A Frame names its exact terminal value, managed-field ordinal and
carrier-array root. Its scalar lanes and array-field map come from the existing
`FrameWords` layout. There is exactly one managed result field, with a current
unsigned-array root. Records, multiple returned arrays, nested/escaping composites,
growing producers and child native regions retain the maintained path. Selection
uses no source names, table sizes or values.

`numeric_versions.rs` checks the returned managed field's functional version on
every reaching path. Root provenance alone does not authorize mutation or return.
Stale reads/returns, discarded updates, swaps and unknown forwarding remain
excluded. Simultaneous next-state copies still read all old scalar lanes first.

The shared emitter retains owners separately from scalar SSA state and emits
native typed loads/stores, checked operations and source control. Normal exit
spills the actual terminal frame's scalar lanes as well as the carrier lanes.
The terminal values can differ from the carrier: the generic witness returns
`i + 17`. Scalars are synchronized to already admitted frame lanes after private
cells freeze. The current owner is installed only then; no additional entry pin
can force the first consuming write to copy an exclusive backing.

Core's unchanged fixed-shape owner and existing first-change boundary retain
ownership authority. Scalar/array results preserve their previous publication
protocol. A private Frame needs no canonical aggregate or new publication ticket
at native exit. Later ordinary source operations keep their own fallible bounds
and admission at their lexical positions. Frame construction bounds and exact
shape relations remain checked; unsuitable contexts decline native entry.
Failure preserves published outputs, aliases, cleanup and retry.

IR118/work policy93 commits this recipe and terminal-field synchronization. Its
bounded field work is conservatively prepaid on every traversal, including the
terminal one. The original first-change settlement/reopen uses the same account;
it does not refill a paid traversal. Canonical bytecode14 and ABI35 are unchanged.
Restored images independently rederive result identities, owner links, scalar
lanes, factory associations, boundaries and prices from the validated plan.

## Complete paired result

| Subject | Median seconds | Multiple of fresh Roc |
| --- | ---: | ---: |
| Pinned reference, 20000 observations | 0.000290 | 1× |
| Retained budget-correct round-12 compiler | 0.073378958 | 253.0309× |
| Mixed-result compiler | 0.072503125 | 250.0108× |

After/before is 0.9880642486. This is one independent cohort; comparison against
round 12's earlier 251.38× denominator is not another paired speedup. All five
calls per side preserve exact raw bytes, independent inflation, capacity 13 and
terminal zero managed bytes/nodes. Warmup and the extra observation are excluded.
The same mimalloc 2.1.7 object and evaluate_into-only timer are used. Compilation,
reserve13, clear/rebind, verification and drop retain their existing boundaries.
Builds and tests do not overlap protected primary or scalar measurements.

Every call reports `same_backing=false` on both compilers. Capacity retention
therefore does not establish physical output reuse; those public-boundary copies
remain timed. Logical work changes 530905237224 → 527732164684 under IR118/policy93.
It is accounting work, not executed instructions or measured memory traffic.
The diagnostic NoLimits policy is unchanged.

- Before executable: `390370ca3419a4fa03646979a3ff121db07fc85cbca83e7226cf89db38d409a5`.
- After executable: `59c15cf5021fa2947779334c0e4f8c17f1b393c3ca5d5e3dc955444f38e89bae`.
- Reference executable: `3941ef59b2c9343d0493cc6b6d14ca4803e6bd27acfe520b1fb59d9b63026324`.
- Allocator object: `38c6ec82a41e90a96bb9f1ae2c89058a77909da3fbb69c16e542ae5c858bf874`.

`primary/` retains commands, library hashes, raw observations and the separate
56-repetition profile. No larger input or cancelled campaign was restarted.

## Qualification and scope

Nightly `2026-03-03`, aarch64-apple-darwin, warnings as errors, locked/offline
Cargo, one compiler job and `CARGO_INCREMENTAL=0` were used. The existing default
native-profile consumer selects five math operations with the established Core /
Engine compiler configuration. Unit and linked qualification use the existing
`unit-qualification` feature profile and actual runtime source-routing consumer.
The default profile was restored before timing; its rebuild took 249.30 seconds.
The initial GPU unit-profile rebuild took 71.99 seconds.

| Executed check | Passed | Command / receipt owner |
| --- | ---: | --- |
| Core snapshot units, retained executable with unchanged Core sources | 87 | `validation/round13-core-snapshot-cohort.*` |
| Existing 129 components plus 15 mixed-result cases | 144 | `default-qualification/component-v2-run.json` |
| Native scalar correctness | 8 | `primary/selected-native-run.json` |
| Bit-operation correctness | 3 | `primary/selected-bits-run.json` |
| Actual runtime fuel / prepaid publication units | 4 | `units-v3/unit-run3.json` |
| Actual generated runtime-only linked retry application | 1 | `qualified-unit-components/linked-execution.json` and run log |
| **Unique scoped Rust total** | **247** | No remaining failed test in the executed scope |
| Architecture, catalog, inventory, safety, warnings and formatting | 12 | `validation/round13-static.*` |
| Applicable checks after late test-only changes | 5 | `validation/round13-late-checks.*` |
| Fresh protected scalar latency/allocation controls | 12 pairs | `primary/scalar-results.json` |

Exact executable arguments and full feature/library metadata are in the receipts.
The maintained integration root is adapted only to absolute module paths. The
linked module is excluded from the default cohort and retained for the actual
linked qualification. The unit build compiles the actual GPU lib root with
`--test`, opt-level1 and `-D warnings`, using the unit-profile dependency closure.
The Core binary is retained with its original build receipts; no Core source
changed. Targeted late reruns cover the two expanded test bodies, not new counts.

The cases cover original/decoded artifacts; actual terminal scalars; every
supported scalar field kind, negative zero and NaN payload bits; a managed field
at ordinal 2; changed/no-op/zero-visit paths and input identity; same-shape fresh
owners and changing parent arguments; actual shared aliases; 2×3/3×2 geometry;
short guarded reads; stale, multiple, record, nested and escaping returns;
assertion/load/arithmetic precedence in both orders; overflow after mutation;
selected frame-output ceilings; exact/one-short finite work; prior publication
identity, cleanup and retry. Existing component expectations and allowances,
including private write allocation and unchanged-store output bounds, are intact.

Sixteen observed mixed-result fuel exits freeze actual prefixes of
64/63/61/57/49/33/17/1 changed cells across the two forms. Current scalar fields,
released loans, prior publication and final bytes/nodes are checked, followed by
successful retry and terminal `(array,81)` verification. The existing observer
also records ten genuine read-only exits and sixteen original write exits.
Prepaid publication adversaries still reject foreign arenas, stale generations,
wrong types/accounts and oversized bounds, then publish successfully at full
admitted memory.

The real linked application executes **3 → 0 → 2**, with the private mixed child
actually selected beneath its ordinary parent. Rejection follows a changed write,
preserves assertion-before-division order and the previous output, cleans up,
and recovers. Its binary hash is
`32067a42d7e205fa9e884d9988da6664ddc804611fe0c8529d41033f11a063bb`.
Generated Object/descriptor/source pins and actual Cargo compilation units are
retained. Runtime features are canonical-native-program-runtime, Engine native
artifact, and the required Core bool/matrixd/convert/serde/tuple/u32/u64 subset.
The deployed executable excludes parser, source compiler and Cranelift. This is
feature qualification, not a full deployed-compressor timing result.

All twelve fresh scalar pairs preserve exact allocation requests/bytes and the
zero loop allocation slope. Maximum direct latency ratio is **1.0103051**, below
the unchanged 1.05 ceiling. The historical round-10 decoded-U64 observation
**1.0526036** remains unresolved and is not replaced by this passing cohort.

## Allocation, Object and profile evidence

System/probe component observations are separate from mimalloc complete timing:

| 256-step witness | Before requests / bytes / ordinary visits | After |
| --- | --- | --- |
| Generic two-call parent | 9251 / 646577 / 9811 | 1619 / 172265 / 51 |
| Unchanged assign-lengths parent | 4918 / 303186 / 9651 | 340 / 52138 / 108 |

Positive iteration counts are flat. The private real-buffer test adds no payload
copy for the result pin: large-minus-288 requested bytes are exactly
**523712** for U16 and **6000900** for U32 at zero, one and eight updates, in both
forms. The real cache requests 6043774 bytes at initialization and 6045358 at
one/eight updates; positive cases have 296 requests and 23 ordinary visits.
Those fixed costs include the ordinary output and terminal assertion.

The complete Object selects **149** numeric regions: 45 read-only / 72 load sites,
and 104 writes / 341 loads / 130 stores. All read regions and 101 write regions
are parented. The **13 new mixed-result regions are all parented**. The shared
emitter audits the actual funded, already-private CFGs for zero explicit calls;
first-change, fuel, failure and exit boundaries remain distinct.

All 320 first-recorded candidate dispositions are retained with their origins:

| First disposition | Count |
| --- | ---: |
| Selected | 149 |
| Child loop or activation boundary | 76 |
| Stale returned functional version | 41 |
| Multiple managed result fields | 25 |
| Changing/swapped/unknown array forwarding | 18 |
| No exact private result frame | 6 |
| Unsupported scalar/array/tuple result kind | 5 |

These are compiled sites, not dynamic invocation counts. The result guard
precedes other checks; removing one guard does not qualify every candidate.
The full Object is 7748568 bytes, SHA-256
`eef36a03e58b00c5a85c3edff1cb0c67135bf908ec666690d395bb2ee6a488f2`.
Passive inspection of its 1936962-line disassembly records direct byte loads /
stores and exact hashes/excerpts in `object-audit/machine-audit.json`. Whole-Object
opcode counts do not identify individual source functions or prove runtime
frequencies. The unchanged reference binary's earlier typed-memory excerpts
remain relevant; no new source mapping for its hashed procedure names is claimed.
Large binary/disassembly artifacts remain private.

The separate after profile contains 1347 sampled stacks. Overlapping inclusive
ancestry includes helper 1041, frame handling 472, frame-word admission 331,
frame-input admission 329, ordinary visits 213 and selected calls 196. These counts
are not additive time shares or predicted speedups. Ordinary frame admission
remains prominent despite the successful component transformation.

## Disposition and next boundary

Initial and diagnostic attempts remain in their own directories. The preliminary
conditional prefix/cumulative witness correctly reproduced the result restriction,
but the brief narrowed the first acceptance target to direct assign-symbols.
Its transport observations remain separate. No predicate-sensitive proof was
added to force cumulative selection.

The first mixed fuel fixture lacked the exact private-use frame required by the
class; its combined structural projection remains on the maintained path and is
now an explicit regression case. A normal outer tuple consumer qualifies. The
next attempt withheld from complete work and rejected later public packing.
The observer now captures actual native exit work and withholds traversal fuel
there. This changes test observation, not production limits or accounting.

The first new broad cohort passed 13/15. One fixture replaced a dimension with
None, producing an invalid partial library policy; it now preserves the existing
NoLimits policy except for its deliberate finite ceiling. Another called the
resident oracle with two 64-step parent visits, requiring 65888 comparisons against
its inherited 65536 allowance. That allowance is unchanged; the established
independent model checks every output word and terminal scalar for the large
retry case. Both corrected tests pass. All existing expected values and
allowances remain unchanged. These were new fixture errors, not introduced
production regressions. No remaining introduced failure is known in the executed
scope. The historical scalar timing failure remains unresolved.

Skipped coverage is explicit: the selected catalog omits the all-primitives
not/xor case; no full all-unit/platform sweep, fresh larger complete input,
complete deployed-compressor timing or cancelled full-corpus campaign was run.
CI is cancelled by user request and is not claimed green. The PR stays draft;
fresh review is requested at publication and no merge is attempted.

The next bounded candidate is **predicate correlation for current-version facts**.
The exact blocking operation is unchanged `huff-cumulative` at lines 63–71 of the
pinned Huffman source: conditional write, merge, then another branch on the same
`more` value. Its actual post-change rejection remains
`leaf returns an older array version`. Source execution returns the old carrier
only on the no-write path; the current must-analysis loses that correlation.
The retained independent conditional prefix is the minimal witness. A follow-on
must preserve stale-alias rejection, condition redefinition, source error order,
simultaneous updates, finite fuel and restoration, prove selection from exact
current definitions, and repeat this same paired timing. Multiple arrays, growth
and child native regions remain separate. No such semantic change is included
in this completed slice.

`closure-inventory.json` covers the completed packet and verifies the immutable
prior timing/qualification inventories. The original timing report and pending
list remain historical; this closure supersedes their status.
