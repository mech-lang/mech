# Platform filter proposal v1

This directory is an additive, unexecuted candidate. No frontend checkout, Site,
saved source, report or sample has changed. Tests, pin checks and QA require the
common `/tmp/mech-clbg-timing.lock` and a root-reviewed integration packet.

`traits.mjs` is a separate copy of the current frontend module, adding only the
platform dimension and qualified measured execution override from `platform.mjs`.
The existing app renders dimensions generically and already implements OR within
one category and AND across categories. Manual choices and release year remain
constraints; empty platform selection includes all. Existing year/history APIs,
family controls and the Mech view reset/disable logic need no separate platform
implementation. `chartDomain` and the domain cache key remain unchanged, so axes
continue to use the admitted cohort independent of platform/year/manual display.

Options are JVM, BEAM, .NET, Native executable, JavaScript and Unclassified. .NET
includes JIT and NativeAOT; the existing Execution filter distinguishes them.
Detailed per-program labels retain `.NET NativeAOT` and `GraalVM NativeAOT`.
GraalVM NativeAOT belongs to Native; .NET NativeAOT stays in .NET. Neither is
classified as JVM. Every timed profile in a language
entry must have a qualified backend, report/source pins, saved command and saved
runtime receipt. One missing/contradictory profile or a mixed backend aggregate
keeps the entry Unclassified. Names/extensions/upstream IDs provide no fallback.

`test_platform.mjs` contains synthetic, meaningful fixtures for evidence failure,
literal True qualification, stale Java execution repair, native GraalVM exclusion
from JVM, .NET JIT/NativeAOT grouping, OR/AND/empty selection, manual/year
intersection and fixed axes using the existing comparison/math implementation.
Proposed later locked command: `node test_platform.mjs` from this directory, plus
the existing explorer trait/math tests and visual QA after approved integration.
No command has run yet. The node executable must be the pinned workspace runtime.

`java_evidence.jq` reads existing metadata only. Its output binds all 17 current
Java endpoints to the exact three saved reports and records displayed source
pins, actual commands, compilation commands, execution labels and sample arrays.
Those records prove local OpenJDK JVM JIT rather than the upstream GraalVM AOT
submission setting. `java-admitted-metadata.json` is unqualified until an actual
locked hash/identity check. Historical runtime versions come from the unchanged
report or pinned original report; today's executable version cannot establish
which runtime produced a historical sample.

After locked reconciliation, root's producer should add `measured_backend` to
each admitted program and actual receipt pins to `data.provenance`; put Java in
the JIT execution group, remove it from AOT, and update the stale Java AOT prose
in the producer protocol/README. This is an additive metadata correction. Keep
all original source/report/sample bytes and language release year unchanged.
Set the qualified program's `runtime_policy` to its detailed measured backend
label so the existing source inspector shows JVM/JIT and explicit NativeAOT
details without new controls or implementation details in the selection flow.
The candidate also repairs the displayed Java execution trait only when its
complete per-program evidence is qualified, protecting against stale group data.

`platform-evidence-plan.json` describes exact evidence routes for existing
Erlang/.NET and six requested new languages. New planned runtimes never become
verified before source-work and full-run reconciliation. This proposal does not
admit V or alter any publication language scope.

`existing_platform_evidence.jq` and `existing-admitted-metadata.json` provide the
same exact admitted per-program metadata route for all displayed Erlang and four
.NET entries. The receipt includes saved actual commands, native/module/source
pins and sample arrays, plus preserved BEAM and .NET build/runtime receipts.
This is a read-only metadata extraction, not a hash verification or fixture run;
all records remain `verified:false` until locked reconciliation.
