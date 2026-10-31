# One-hour compiler audit — 2026-10-31

> Correction: pointer-role counts in this historical report are superseded by the [R62 audit correction](../compression-round-62/pointer-role-correction.md). The original counter missed one ARM64 immediate form. Whole-Object totals, measured hardware counters, allocations and timings remain unchanged.

The complete unchanged eight-byte level-12 workload remains about thirteen times the reference. The final direct R56→R61 comparison uses one fresh reference and gives **13.656178×→13.553403×**, an observed **0.752587% elapsed reduction**. Three of four prepared pairs and 49/64 alternating blocks improve; pair 3 is **1.724041% slower**. This is a modest measured gain, not parity or a larger-input result. Percentages from separate rounds are not multiplied.

| Retained change | Matched elapsed change | Separate retired-instruction change | Allocation requests |
|---|---:|---:|---:|
| R59: existing interned schema key, from the supplied brief | −0.445361% | −0.325168% | 8,773→8,173 |
| R60: admitted vector geometry | +0.322840% | −0.968622% | 8,173→8,173 |
| R61: omit unread first-change local snapshot | −1.395305% | −0.437626% | 8,173→8,173 |

R59 retains its adverse +11.006495% cycle cohort. R60 retains larger text/stack and adverse elapsed/cycles. R61 retains +1.155690% diagnostic elapsed and its physical-footprint increase; diagnostic elapsed is excluded from primary. Each round has independent counters, allocation census, CPU samples and exact binary bindings. Incremental peak heap remains unchanged. No hardware-counter percentages are inferred from CPU-time sample weights.

## Newly identified mechanisms

**Register allocation is excessively reloading long-lived bookkeeping pointers.** R60 removes 1,724 static division instructions, yet text grows 759,684 bytes. Actual disassembly attributes much of that growth to 62,620 additional locals-pointer reloads and 15,732 argument-pointer reloads. Both pointer slots were already above 100 KiB; this was not a newly crossed immediate-offset boundary. A matched source block loses pointer reuse between two reads. The pinned allocator has a two-split fallback, but whether that exact fallback triggered remains unproved. R61 still contains 165,897 static locals-pointer reloads. See the [binary investigation](../compression-round-60/pointer-reload-addendum.md).

**First-change materialization saves an unread snapshot.** Current scalar fields, index, incoming value and fuel already travel in explicit arguments; other live values stay in SSA. Removing one call deletes exactly 28,774 original-local stores at 312 static bridges. Another 7,226 stack stores and 35,017 stack reloads disappear through backend allocation changes. Algorithm loads remain unchanged. The new original/decoded regression covers delayed changes to two arrays, current scalar/Boolean state, actual aliases, callback rejection, clone denial, unchanged publication and retry. The callback-capacity case does not claim to test the separate generated pre-call capacity guard.

**The constructor metadata family contains smaller ownership mistakes.** The [construction audit](construction/audit-findings.txt) attributes 574 requests to disjoint observed families, including an already-owned refinement call cloned again (149 requests), binding cloned before discovering there is no seal (72), and validation through temporary descriptor construction (30). These are candidate removals, not measured savings. They overlap the broader prepared-plan proposal and must not be added to its ceiling. Three unapplied patches and acceptance plans are retained.

**309 imported constants have no semantic reads.** The [inventory and design](ownership/dead-constant-design.md) identify a candidate 618 requests, but eliminating imports must preserve current setup admission, retained-node accounting and registry/alias visibility. Zero use alone is not permission to bypass those obligations.

**Budget operations repeat bookkeeping.** The [scoped-ledger audit](ownership/scoped-ledger-audit.md) identifies duplicate thread-local lookups within a single operation. Two sinks also build full rejection collections while needing only the first result. The existing ordered first-violation API offers a bounded instruction target; successful calls do not allocate there, so this is not an allocation-count claim.

## Review checkpoint and next bounded step

Production commits: R59 `7658ace6877bc48bc7445ce967ca48994149c5d3`; R60 `172a4e9ce`; R61/build source `6f412525daf0f7284d4ef6bc2bd8e21f95d22e96`. The source and bytecode hashes remain `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9` / `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`. Algorithm, tables, real scratch capacities, allocation sites, allocator, output reuse and evaluate_into timing are unchanged.

Local qualification passes: 328 runtime tests, 236 selected components, eight connected cases, the path witness, 12 linked cases, 12 scalar controls under the original ceiling, eight frame controls, 34 selected CI tests, 132 Python tests and all architecture checks after correcting evidence-file layout. These groups overlap. Initial test/format/evidence failures are preserved with their corrections. One inherited reduced-catalog knucleotide exclusion remains; full hosted CI, all-features, Miri and larger inputs are not claimed. A separate source review found no introduced correctness issue; it does not replace external review.

The next compiler experiment should trace only the locals and argument-pointer values during register allocation, recording splits, spills and minimal-bundle fallback events outside performance timing. Establish the trigger before altering limits or register policy. A pinned-register experiment has an explicit callee-save obligation and must not be implemented by merely enabling a flag. The existing scalar, error, alias, finite-budget and decoded-artifact controls remain required.

All proposals beyond R61 remain unapplied. PR973 stays draft pending hosted CI and external review. The fresh final comparison and each adverse result are retained in the [R61 review packet](../compression-round-61/REPORT.md).
