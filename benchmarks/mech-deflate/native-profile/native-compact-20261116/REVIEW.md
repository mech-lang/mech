# Review and continuation handoff

Review **mech-lang/mech PR #993**, branch
`codex/native-compact-source-handoff`. It is stacked on #973's
`codex/level12-compression-consolidated` branch. The compiler changes, Mech source,
build recipe, runtime source, and evidence are committed here. Another agent can
fetch this branch and use it as the basis for compiler work.

```sh
git fetch origin codex/native-compact-source-handoff
git switch -c codex/review-native-compact origin/codex/native-compact-source-handoff
```

The retained runtime floors are **81/81 Roc and 61/81 libdeflate**. The next
milestones are 62, 71, and 81 libdeflate wins with repeatable qualification.
Continue reducing emitted code and complete build time toward
1× libdeflate. Preserve checked errors, origins, ownership, and finite admission;
keep the ordinary resource-accounting limitations explicit. The rebuild recipe
uses no external LLVM executable. Do not substitute old M3/LLVM measurements for
the current Apple M1 comparisons.

Reusing the verifier's graph and proving identity-route reachability cuts
matched native time by **1.71%** and the complete build by
**0.50%**. The Apple M1 build takes **3.83915 s** through the
final link, **1.285× Roc** and **8.016× libdeflate**. It reproduces
the exact timed worker and emits **71,768 B**, or **1.244× Roc** and
**1.935× libdeflate**. Both full runtime orders preserve all **81/81 Roc**
and **61/81 libdeflate** wins.
[Compiler comparison and reproduction](evidence/native-graph-work/README.md)

The compiler now skips unused general SSA for eligible refused private
numeric phases. Its measured source-through-link build is **3.73777 s**,
a **0.87%** paired reduction with the identical 71,768-byte worker.
[Compiler source, checked cases and raw timings](evidence/refused-general-ssa/README.md)

For new runtime campaigns, **policy v2** reports slowdowns and bases the
runtime gate on preserved individual peer wins and both absolute floors.
[Qualification policy v2](QUALIFICATION.md#acceptance-policy-v2)

The 71,288-byte owner-exchange worker beats Roc on all 81 cases in both
complete v2 orders. Both Mech workers score 61/81 against libdeflate in
the forward order and 60/81 in reverse. In reverse, the candidate loses
the control's random-1024 win and gains 320-overlap. The runtime gate
rejects the candidate on the individual loss. The 71,768-byte worker remains
the reference. [Complete v2 orders and replay](evidence/owner-exchange-71288-v2/README.md)
· [Earlier v1 focused orders](evidence/owner-exchange-71288/README.md)
· [Match-cache source sizes](evidence/match-cache-offset-slot/README.md)
· [Cached terminal parser experiment](evidence/cached-terminal-failures/README.md)

Direct indexing of completed event pages brings the matched Apple M1
source-through-link build to **3.81385 s**, a paired reduction of
**1.23%**. The 71,768-byte worker reproduces exactly.
[Parser source, finite-limit checks and matched builds](evidence/batch-page-index/README.md)
· [Independent matcher source screens](evidence/mech-source-matcher/README.md)

## Latest matched build and runtime

The current opt-in **71,768-byte** Cranelift encoder uses
**1.244× Roc** and **1.935× libdeflate** text. Single-piece source
access brings the matched Apple M1 build through the final link to
**3.93889 s**, or **1.295× Roc** and
**8.115× libdeflate**. Both fresh full runtime orders
record **81/81 Roc and 61/81 libdeflate**, with every individual win
preserved and one duplicate-control repeatability case open.
[Matched build and parser checks](evidence/contiguous-syntax-source/README.md)
· [Bound runtime orders](evidence/bound-71768-runtime/README.md)

## What is retained

The baseline recipe remains unchanged: **72,520 bytes of encoder text**, or
**1.25693× Roc and 1.95577× libdeflate**. Historical full runtime runs in both
orders record 81/81 and 61/81. Rebuilt objects and metadata match those artifacts
exactly; freshly linked worker executables passed correctness checks but have
not received a new full peer timing qualification.

The frontend allocator change is published in `21757538`. Its matched comparison
improved complete build time by 8.83%, with all ten Mech outputs byte-identical.
The parser now reads 32-entry journal pages, hashes token bytes in contiguous
spans with unchanged work limits, and skips an unused map when caching subtrees.
The parser benchmark improves by 16.82% with identical trees and parser steps.
The matched complete build improves from **6.30298 to 5.65845 seconds**
(10.23% less time), reaching **1.89458× Roc and 11.61221× libdeflate**.
The retained encoder object, complete metadata, and pinned benchmark worker
rebuild byte for byte. All 531 syntax unit tests pass. Review the three source
files under `src/syntax/src/document/parser` and the
[page-size, parser, and complete build evidence](evidence/parser-journal-pages/report.json).
All build comparisons include final linking with compilers, fixed runtimes, and
adapters prebuilt. See the README for phase timings and exact rebuild commands.

The native cleanup change batches ordered block parameter removal in Cranelift's
constant-phi pass and Mech's local-phi and dead-value passes. All 207 Cranelift
unit tests pass. With the same frontend, the native-only median moves from
2.22625 to 2.21648 seconds, and the complete build moves from 5.65167 to
5.64176 seconds. The retained benchmark worker matches byte for byte.
[Source-to-executable trials and checks](evidence/batched-block-params/report.json)

## AArch64 checked-root zero branches

The AArch64 checked-root branch recipe emits **71,768 B** of encoder text,
**1.24390× Roc** and **1.93549× libdeflate**. It saves 192 bytes and
reduces measured peak native stack to 5,280 bytes. Its matched Apple M1
source-through-link build is **4.00611 s**. It stays opt-in for runtime
review. [Rebuild, both full timing orders, and per-case bounds](evidence/branch-on-zero/README.md)

## Ordered scoring address experiment

Carrying the vector addresses through the ordered scoring loop cut 164,742
instructions on 8 KiB natural text and added 43,020 stack reloads. The
runtime was flat against the Mech control and about 1.04× libdeflate.
The encoder grew by 64 bytes and the empty-input regression triggered the
validator's Reject decision in the two focused orders.
[Patch, replay, and exact object reproduction](evidence/ordered-vector-cursors/README.md)

## Shared AArch64 frame copies

The opt-in source recipe emits **71,192 B**, or **1.234× Roc** and
**1.920× libdeflate**. Its matched source-to-final-link build takes **3.82835 s**.
Both 17-case orders preserve every individual win at **17/17 Roc and 13/17
libdeflate**. The 4 KiB random case remains unresolved because the reverse
order conflicts with the bounded duplicate-control follow-up.
[Source reproduction, matched sizes and runtime](evidence/shared-copy-tails/README.md)

The two later full orders record **81/81 Roc** for both actors. The
71,192-byte candidate records **57/81 and 61/81 libdeflate** versus
**57/81 and 60/81** from the unchanged control. No individual control
wins are lost. The full v1 decision is **Reject**, with a **10.468%**
paired slowdown on `prefix-periodic-16384`. The archive retains both raw
orders, identifies the cases behind the same-worker win-count drift, and
reproduces the decision. The 71,768-byte worker remains the reference.
[Full 71,192-byte replay and control drift](evidence/frame-copies-71192-runtime/README.md)

The separate batched timing study resolved duplicate-control precision on
seven-stored-prefix for its diagnostic link. It measured the 71,768-byte
object slower than the 71,960-byte control, with other short cases still
unresolved. The original paired-worker outcome is preserved.
[Clock calibration and exact replay](evidence/batched-clock-calibration/README.md)

The 71,704-byte scorer prototype makes natural text faster in both orders
and is rejected on two short inputs.
[Scorer source experiment](evidence/ordered-scorer-addresses/README.md)

## Related-post architecture check

The architecture job stops on the historical after-4 comparison host's
same-Object restore. The producer and linker consume separate inputs
there. The checked generator already binds them for managed launch.
The review records the live callers and the migration boundary that
preserves the benchmark's timing and input contract.
[Hosted check and exact sources](evidence/related-post-linked-binding/README.md)

## Cold error-origin tables

An exact Mach-O prototype packs 291 cold origins and emits **71,256 bytes**
of encoder text, or **1.92168× libdeflate**. Both focused links preserve
all measured peer wins; empty input triggers rejection at the standard
placement and remains unresolved at equal stub and constant section
addresses. Exact reproduction, checked errors, and both timing receipts
are recorded. [Prototype and intended backend boundary](evidence/cold-origin-tables/README.md)

A follow-up removes one repeated reference-count assignment in the
terminal append helper and independently combines it with the 71,256-byte
packed origin prototype. Both preserve individual focused wins. Empty
input is 1.146% slower with the helper change alone and 2.375% slower with
packing, so policy v1 rejects both variants.
[Focused experiments and raw replay](evidence/terminal-owner-publication/README.md)

## Single-process source compilation

The maintained rebuild accepts `--direct-source`. The existing diagnostic
driver now hands its in-memory Program artifact and function catalog to
Cranelift. The canonical bytecode receipt remains available at
`frontend/program.mecb`. The external bytecode emitter still validates input
canonicality when invoked on a file.

The 71,960-byte recipe completes in **3.99930 s**, a **2.13%** reduction
from the split control in six matched orders. Peak compiler RSS rises from
382,443,520 B to 401,408,000 B. Both exact benchmark workers keep the
recorded 81/81 and 61/81 scores in each full order. Parsing still takes
2.108 s and native emission 1.518 s.
[Exact inputs, profiles and output identities](evidence/direct-source/README.md)

## Reusable parser cache frames

Each parser keeps the frame allocation for its active subtree sink and
reuses it after that request completes. On the benchmark source, frame
stack growth drops from 706,474 to one. Matched Apple M1 source through
final link improves from **4.01608 to 3.95160 seconds** and reproduces
the exact **71,768-byte** encoder and linked timing worker. Both full
runtime orders on that worker recorded **81/81 Roc and 61/81 libdeflate**.
[Checks, build trials, and reproduction](evidence/parser-cache-frames/README.md)

## Optimized compiler builds

On this M1 the same 71,960-byte encoder builds in **4.76479 s**
at Rust opt-level 3, down from **5.64395 s**.
That is **1.596× Roc** and
**9.893× libdeflate** through
final linking. The compiler reproduces both pinned benchmark workers and their
existing runtime receipts exactly. The profile flag is optional and the default
rebuild stays at level 1. The compiler's C allocator archive is byte-identical.
[Matched trials and raw profiles](evidence/host-opt-levels/report.json)

Parsing still takes **2.618 s** of the retained build. Native
emission takes **1.765 s**. Inspect the 14 built vector
alternatives, of which five are selected, before adding compilation concurrency.
The raw marker spans cover groups of work. Instrument an individual compile or
cleanup before assigning its cost to a specific rewrite.
The [linked-size census](evidence/linked-size-census/report.json) attributes
237,384 bytes of retained executable text to Rust standard libraries. With
the common deployment CLI and shared strip settings, Mech is **575,632 B**,
Roc **191,920 B**, and libdeflate **155,760 B**. The same generated Mech
encoder contributes 71,768 B, and Rust standard libraries still contribute
199,224 B of the remaining linked machine code.
[Deployment sources, section census and raw verification](evidence/minimal-deployment/README.md)

The standalone fresh-owner diagnostic ran eight fresh-process epochs using
one executable file and separate byte-identical copies, with two batch floors.
The table counts intervals within the existing 0.5% resolution.

| Case | Duplicate control | Candidate |
| --- | ---: | ---: |
| `prefix-natural-16384` | 7/8 | 8/8 |
| `random-64` | 0/8 | 8/8 |
| `reymont-prefix-32769` | 8/8 | 7/8 |

All earlier observations remain in the qualification review. The diagnostic
preserves the unresolved decision. [Predeclared plan and raw observations](evidence/fresh-owner-repeatability/report.json)

In the eight-pair root inlining ablation, complete build time moves from
**4.77169 to 4.67504 s**.
Native emission moves from **1.76630 to
1.67275 s**. Encoder text moves from
**72,520 to 72,600 B** and the root
frame goes from **3,744 to 3,536 B**.
The changed encoder passes 162 exact linked calls. Full runtime and peak-stack
qualification remain open. The pinned recipe keeps its existing inlining.
[Ablation and raw trials](evidence/root-inlining-opt3/timing.json)

## Equivalent vector alternatives

The native phase compiler can reuse a completed general body when its entire
prepared function matches the vector variant. The comparison covers effects,
source locations, external names, and layout hints. The vector keeps its own
guard and cleanup metadata. Declarations are replayed in the original order,
and disassembly requests still compile their requested body.

On both encoders, seven of nine unused alternatives have identical prepared
functions. The worker pool submits **23 → 16 jobs**, with two cleanup jobs in
both runs. The other two unused alternatives have different prepared IR and
still go through the existing backend comparison. The bytecode, complete
objects and metadata, and both historical benchmark workers reproduce exactly.
The historical runtime receipts record **81/81 Roc and 61/81 libdeflate** in
both full orders.

On Apple M1, eight matched native-only pairs move from
**1.76341 to 1.76057 s**.
Six balanced full-build orders move the smaller encoder from **4.76513 to
4.76469 s**, or **1.592× Roc** and
**9.909× libdeflate**. This is a measured
reduction in backend work with essentially flat complete compilation time.
Use `rebuild.py --compiler-opt-level 3 --reuse-equivalent-vector-ir` to reproduce
the opt-in path. [Raw trials, controls, and rebuild receipts](evidence/equivalent-vector-ir/report.json)

## Cache accounting for one-shot parsing

The baseline parser's cache handles **1,414,476** requests for the pinned
source. By the time **708,002** are processed, their markers can no longer
receive a tree. The one-shot compiler counts the original work and reserves
the original IDs for these requests without constructing their trees. The
existing grant must cover the entire accounting pass. Both modes traverse the
same event tape and use the same cache lookups. Streaming retains its original
materialization and allocation accounting.

In six balanced parser-only pairs, parsing takes **2.59947 →
2.44044 s**, a **6.12%** reduction.
The full tree and token identities, 3,967,302 parser steps, and 718,353 events
match exactly. Three smaller source fixtures also match for tree, event, and ID
counts. All **533** syntax unit tests pass, including finite grants, rewinds,
multiple UTF-8 source pieces, and exact diagnostics for malformed input.

| Matched Apple M1 build | Source through executable link | Encoder text |
| --- | ---: | ---: |
| Retained Mech before | 4.76028 s | 72,520 B |
| Retained Mech with cache accounting | 4.59466 s | 72,520 B |
| Smaller Mech before | 4.76323 s | 71,960 B |
| Smaller Mech with cache accounting | 4.60240 s | 71,960 B |
| Roc | 2.99970 s | 57,696 B |
| libdeflate | 0.48119 s | 37,080 B |

The smaller build takes **1.534× Roc** and
**9.565× libdeflate**. Both historical
benchmark workers reproduce byte for byte and carry **81/81 Roc and 61/81
libdeflate** receipts from both full orders. The existing rebuild command uses
the one-shot parser for the exact source. [Census, build trials, and controls](evidence/stale-cache-accounting/report.json)

## Canonical parser storage

Literal continuations reuse a per-parser frame stack and scanner slot. The
one-shot parser copies short cache request ranges into a queue so they release
their journal pages. On Apple M1, the 71,960-byte encoder's matched complete
build goes from **4.53898 to 4.18694 s**
(7.76% faster), with parsing at **2.45876 to
2.10957 s**. The exact bytecode, object, metadata and timing
workers match the recorded recipes with **81 Roc and 61 libdeflate wins**
in both complete orders. [Measurements and proof](evidence/compact-parser-storage/README.md)

## Parser stage profile

In six balanced Apple M1 parser-only pairs, the coarse instrumented run
spends **2.565 s** inside canonical grammar processing,
including in-parse cache materialization. Syntax indexing takes
**35.3 ms**, and restart indexing takes
**2.22 ms**. All trials reproduce the same
tree and identities, 3,967,302 parser steps, and 718,353 events.

The separate nested profile observes **499,404**
cache advances in **0.507 s** and
**1,414,476** cache requests queued in
**37.2 ms** during grammar processing.
The nested instrumentation adds wall time. These measurements guide work on
cache scheduling and materialization. Measure which requests publish and
reuse their completed nodes before changing event or finite-work accounting.
[Raw profiles and instrumentation](evidence/parser-stage-profile/report.json)

## Native verification and ownership search

Cranelift compares ordered CFG edge iterators before collecting mismatch
diagnostics. The compact ownership analysis reuses its query storage and the
consumption decision already established at each buffer operation. The
71,960-byte encoder's native emission falls from **1.70433
to 1.59661 s** and its complete build falls from
**4.18568 to 4.07374 s**. The new emitter reproduces
the fixed benchmark workers byte for byte. [Builds and checks](evidence/native-pass-work-reuse/README.md)

## Preselected root inlining

The opt-in AArch64 policy selects private single-call helpers from optimized
IR. It verifies the inlined root and skips the first machine compilation
of the projection policy. A missing inlining candidate follows the existing
path. Both pinned benchmark workers reproduce byte for byte.

On Apple M1, source through final link takes **4.58958 →
4.52728 s** in eight matched orders. Native emission falls from
**1.76414 to 1.69787 s**. The smaller encoder emits
**71,960 B**, or **1.24723× Roc and 1.94067× libdeflate**. It builds in
**1.518× Roc** and
**9.427× libdeflate**. The 81/81 Roc
and 61/81 libdeflate receipts remain bound to both exact historical workers.
The rebuilt adapter also passes 162 exact linked calls in each mode.
[Build trials, profiles, memory and size metrics](evidence/native-root-inline-plan/report.json)

The [cached-join SSA screen](evidence/native-ssa-timing/report.json) avoids
17,270 temporary values and leaves native emission time effectively flat.
The implementation remains in the experiment archive.

## Smaller reviewed configuration

Use `rebuild.py --variant fresh-owner-counts-cold-tails` to review the
**71,960 B** encoder. It is **1.24723× Roc and 1.94067× libdeflate**,
with **81/81 Roc and 61/81 libdeflate** in both full orders. Peak stack is
5,296 bytes. The default recipe keeps the pinned 72,520-byte encoder while
three short cases have repeatability outside the 0.5% resolution.

With six direct continuation input words, the same passes reach 71,720 B and
lose two libdeflate cases in both focused orders. Use `size_screens.py --list`
to find the nine screen cases and `replay.py` to recompute peer wins from the
recorded observations.
[Continuation budgets and linked qualification](evidence/continuation-widths/report.json)

## Fresh owner reference counts

The new pass folds 20 reference-count loads from direct, checked allocations
that have known nonempty storage. It preserves the writes and checked paths.
Review `hosts/gpu/src/native_artifact/program/compact/owner_counts/mod.rs` and its
tests. The five native test groups cover aliasing stores, unknown calls, reused
backings, loop changes, allocation failures, and joins.

Its encoder is **72,344 B (1.25388× Roc, 1.95102× libdeflate)** with unchanged
5,296-byte peak stack. Both full peer orders record **81/81 Roc and 61/81
libdeflate** for candidate and control. The full matched runs bound 77 cases at
0.5% resolution. The [report and raw observations](evidence/fresh-owner-counts/report.json)
include longer runs for the remaining four cases. The duplicate controls vary
beyond 0.5% on three cases, leaving the pass opt-in. The matched build takes
6.24649 seconds, effectively flat against 6.25214 seconds for the control and
12.96053× libdeflate. Use `rebuild.py --variant fresh-owner-counts` to reproduce
the candidate with exact object and metadata checks.

The 72,024-byte cleanup combination still has two sustained slowdowns in the
focused screen. Its exact rebuild variant is `fresh-owner-counts-cleanup`.

## Integration follow-up

The GitHub run for `ddc26e24` passed distribution, browser compute, and the
warning-policy step. The browser standard canary exposed an asynchronous
native fullscreen exit. The corrected check waits for native exit and docked
mode before testing resize. Both modes pass the targeted Chrome reproduction,
and the [hosted browser standard canary passed](https://github.com/mech-lang/mech/actions/runs/44022578666/job/133835834951).
The static audit now has 11 additional exact-file contracts. Its complete-tree
gate still fails on the historical runtime and the inventory includes remaining
violations inherited from #973. The full gate remains required.
[Hosted failures, replay, and audit inventory](evidence/ci-44018345099/README.md)

The all-thread allocator probe now drives the tracked allocator with
owned, layout-checked blocks. Historical probe hashes still verify.
The full-tree unsafe boundary gate now reaches the inherited same-object
binding in the generated comparison host.
[Probe checks, provenance, and the remaining audit failure](evidence/allocator-probe-audit/README.md)

## Dead spill-store screen

A diagnostic AArch64 liveness pass reduced the smaller encoder from 71,960 to
71,944 bytes, or 1.94024× libdeflate. The candidate passed 162 exact linked calls
and Cranelift's register allocation check. The 16-byte saving did not justify
keeping the pass. The pinned workers and their original runtime receipts remain
the basis for comparison. [Source patch and raw checks](evidence/dead-spill-stores/report.json)

## Register spill pairs and function placement

Consecutive ARM64 integer spills and reloads can share one instruction.
This experiment preserves the pinned matcher block order and removes
24 instructions from its leaf, reducing encoder text to **71,864 B**.
The 1 MiB random matcher executes **8.41 million fewer instructions**.
The source-through-link median is 4.52721 s, with a 4.52776 s control.

Both complete timing orders record **81/81 Roc and 61/81 libdeflate** for
candidate and control. The per-case decision is Reject. The two established
slowdowns are 0.576% on `prefix-natural-65535` and 0.599% on
`prefix-natural-262144`.

The 71,896-byte placement diagnostic retains the control's address alignment
modulo 64 for all 40 native function symbols. Both natural cases meet the
0.5% resolution in its two focused orders. One short case remains uncertain.
The source patch, replay entry point, checked calls, raw full runs, and focused
placement follow-ups are in the [experiment](evidence/register-spill-pairs/README.md).

The compiler now reproduces that 71,896-byte object, its complete metadata
and its linked benchmark worker byte for byte. The recipe exposes it as
`fresh-owner-counts-cold-tails-paired-spills`. New full orders each preserve
the same **81 Roc and 61 libdeflate wins**, including every individual win.
The v1 replay rejects promotion because `empty` and `seven-stored-prefix`
slow by **1.704%** and **1.085%**, with both lower bounds above 0.5%.
The two previous large natural-input regressions are bounded. Keep the
retained recipe. The [source-emitted variant and measurements](evidence/aligned-register-spills/README.md)
include both full runs, the exact linked identities and complete build times.

## Earlier experiments

`evidence/size-screens` contains all 24 static screens: ten additional shared-region
families, eight individual compiler policies, and six combinations. These use
the compiler source already published in this branch. No production source or
retained recipe was changed for this follow-up.

| Configuration | Encoder text | × libdeflate | Root frame | Runtime qualification |
| --- | ---: | ---: | ---: | --- |
| Retained baseline | 72,520 B | 1.95577 | 3,744 B | Historical full 81/61 |
| Range-derived owner proofs | 72,392 B | 1.95232 | 3,744 B | Full 81 Roc; C 61/60; not promoted |
| Owner proofs, inline only helper 4879 | 72,168 B | 1.94628 | 3,680 B | Rejected after two focused orders |
| Owner proofs and final cleanup | 72,136 B | 1.94542 | 3,792 B | Full 81/61 in both orders; rejected on five cases |
| Owner proofs, final cleanup, cold tails | 71,768 B | 1.93549 | 3,792 B | Pending |

The narrower proof removes 128 bytes, passes correctness and checked-failure
controls, and keeps the same peer wins as its simultaneous control. Both control
and candidate have 61 then 60 C wins in the full orders; the close case wins in
both longer follow-ups. Tiny-input repeatability remains unresolved, so the
candidate is not promoted. Its matched complete build is 6.26348 seconds versus
6.24346 for the control, with no demonstrated build-time improvement.

The configuration that removes 352 bytes and reduces the root frame by 64 bytes
loses C wins and consistently slows several short cases. It is rejected despite
passing correctness. The smallest static candidate removes 752 bytes but
increases the root frame by 48 bytes; it has not been linked or runtime-qualified.
Do not attach the baseline's win counts to these candidates. The root frame is
not complete stack high-water, and metadata calls are not final relocated calls.
See [the complete follow-up](evidence/owner-proof-runtime/report.json) for raw
paired data, repeated same-worker controls, and every adverse observation.

The owner-proof and final cleanup combination keeps 81 Roc and 61 libdeflate
wins in both full orders. Five cases show a slowdown above the 0.5% resolution
and peak stack grows from 5,296 to 5,344 bytes, so the result is rejected. See
[the full report](evidence/owner-cleanup-full/report.json) and
[the earlier focused report](evidence/owner-cleanup-focus/report.json) for raw
measurements and the exact candidate identity. Its static reproduction case is
`combinations--owners-cleanup`. `replay.py` recomputes wins from both sets of runs.

All ten additional shared-region screens were at least as large as the baseline.
The stack-pair census found only 156 bytes potentially removable by pairing
adjacent stack operations. Root IR, region inventories, negative results, raw
emission logs, complete metadata, configuration differences, and hashes are
included so the next agent can inspect these results without recreating local
scratch directories. Raw receipts preserve original paths as provenance.

After running `rebuild.py` as described in the README, reproduce the static
screens from this checkout with:

```sh
python3 benchmarks/mech-deflate/native-profile/native-compact-20261116/size_screens.py \
  /tmp/native-size-screens \
  --emitter /tmp/native-baseline/compiler/release/emit_compact_size
```

Use `--list` to list configurations or `--case combinations--owners-inline-4879`
to select one. The script checks the exact object and metadata hashes, text size,
root frame, and metadata call count for every selected case. It does not qualify
runtime performance. The recorded single-emission durations are not matched
source-to-executable benchmarks.

## Review priorities

1. Start with the [maintained qualification entry point](QUALIFICATION.md).
   Replay the published experiments, then pin a complete campaign plan before
   starting a new comparison. Promotion requires exact artifact and worker
   identities, both complete orders, individual wins, and per-case bounds.
2. Review the native compiler changes against #973, particularly checked-region
   boundaries, live arguments, owner proofs, direct runtime declarations in
   compilation workers, and the deferred publication order. Start in
   `hosts/gpu/src/native_artifact/program/compact`, including `compile_jobs.rs`.
3. Review the vendored ARM64 backend's branch, relocation, call, trap, and unwind
   updates for cold-tail sharing. The pass remains opt-in. Its existing 15-case
   results include adverse tiny-input estimates; it is not a new full 81/61 result.
4. Inspect `evidence/integration` and `evidence/compiler-allocator` before reusing
   performance claims. Keep object identity, linked-worker identity, correctness,
   and runtime qualification distinct.
5. Review the [integration repairs and open gates](evidence/integration-repairs/report.json).
   The warning scanner passes locally. The distribution snapshots reflect two
   intended compiler allocator packages, public CLI selection includes the sixth
   unit, and both generated Linux launcher feature sets pass with warnings denied.
   Browser execution and the full unsafe-boundary and retired-value gates
   remain open. The PR is a draft.
6. Before promoting a smaller candidate, run correctness and checked-error
   controls, both complete peer timing orders, and per-case uncertainty checks
   with sufficient measurement duration for tiny inputs. Preserve the 81/61
   floors and investigate individual regressions, even if total wins hold.

The optional cold-tail candidate in the main README and the 24 new static screens
are separate experiments. All smaller candidates remain opt-in pending the
required qualification.
