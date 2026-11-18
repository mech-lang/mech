# Batch parser page index

## Complete build

The one-shot parser now directly indexes completed event-page groups.
The standalone paired median is **4.08% faster**. Eight complete build
orders on Apple M1 use the same source, Cranelift options, fixed runtimes
and link adapters with the two prebuilt compilers.

| Matched source through final link | Time | Encoder text |
| --- | ---: | ---: |
| Mech control | 3.87335 s | 71,768 B |
| Mech grouped event pages | 3.81385 s | 71,768 B |
| Roc | 3.01609 s | 57,696 B |
| libdeflate | 0.48388 s | 37,080 B |

The paired reduction is **1.23%** in **7/8**
faster pairs. Parsing moves from **1.98773 to 1.92024 s** in the
complete builds, which take **1.265× Roc** and **7.882×
libdeflate** time. The bytecode, object, metadata, link outputs and timed
worker reproduce byte for byte. The fixed worker passes 162 exact checks.
The archived full runtime result for that worker is **81/81 against Roc and
61/81 against libdeflate in both orders**.
[Matched raw builds](matched/timing.json.gz) · [Phases, memory and artifacts](matched/summary.json) · [Earlier full orders](../bound-71768-runtime/README.md)

## Standalone parser

These standalone Apple M1 trials use the syntax at `b72b7419b39b22765fe1eed6b5c636e63964b905`,
the 243,209-byte encoder source, pinned Rust opt-level 3, and the recorded
mimalloc probe. The candidate hosts use the same parser options and allocator
as their control. Each trial reproduced the same source and syntax-tree
digests, 718,353 emitted events and 3,967,302 parser steps.

| Candidate | Matched control | Candidate parse | Median paired ratio | Faster pairs | Median peak RSS |
| --- | ---: | ---: | ---: | ---: | ---: |
| Direct page index | 1.960629 s | 1.880181 s | 0.959155× | 6/6 | 113,975,296 B |
| Index with bounded page capture | 1.963204 s | 1.939697 s | 0.988189× | 8/8 | 113,999,872 B |
| Inline stale-token accounting | 1.960629 s | 1.964259 s | 0.996245× | 4/6 | 115,793,920 B |

The direct index cut paired parse time by **4.08%** and peak RSS by **1,843,200
bytes**. The earlier combined trial included both grouping and bounded page
capture. Its 1.18% reduction fell below the prior standalone screen. The
stale-token candidate had mixed pair results and a near-flat median. These timings stop at parser completion.
[Standalone summary](standalone/report.json) · [Combined plan](standalone/combined-plan.json)
· [Combined raw trials](standalone/combined-trials.json.gz)
· [Independent plan](standalone/independent-plan.json)
· [Independent raw trials](standalone/independent-trials.json.gz)

## Storage and work bounds

One-shot document parsing groups 32 completed event pages in each retained
index entry and indexes into recent pages directly. The current event page
retains its existing bounded layout. A snapshot holds shared roots for the
completed prefix and frontier. Mutation copies at most one 32-page handle
group, the necessary index path, and the affected event page when shared.
Rewinds preserve the old roots and existing snapshot visibility. Streaming
and diagnostic journals keep the original retained-page policy. The shared
grammar retains the same event tape, stable IDs, and checked work grants.
[Direct-index patch](patches/direct-index.patch.gz)

The separate capture experiment added a FIFO of up to eight versioned page
handles per medium request. The direct index retains the existing short-event
copy and long retained-snapshot policies. It omits that additional queue.
[Combined patch](patches/grouped-with-page-capture.patch.gz)

The stale-token candidate only applies after the existing allowance proof
admits a discarded request. Each event still costs one unit. Each token is
validated before its byte length and completion unit are charged and its ID
is advanced. The instrumented variant counted 708,002 gated stale requests
visiting 5,462,516 events. It saw 2,633,220 one-byte tokens, so inline
accounting removed 5,266,440 resumable-token loop entries while preserving
all associated charges. The timing comparison used the uninstrumented
candidate. [Stale-token patch](patches/stale-tokens.patch.gz)
· [Census counts](census/counts.json) · [Census build and probe](census/build.json)

## Correctness and reproduction

Each candidate passed **535 syntax unit tests** and **60 differential
snapshots**. The fixtures cover malformed source, Unicode, segmented source,
and finite fuel and event limits. Snapshots include the tree and IDs,
node and restart indexes, diagnostics, and parser stats. The direct index
also exercises mutation and rewind across page groups while an immutable
snapshot remains live. The stale-token tests cover exact allowance
thresholds, invalid token kinds, UTF-8 range failures, and empty tokens.
[Direct-index tests](validation/direct-index.json)
· [Capture tests](validation/grouped-with-page-capture.json)
· [Stale-token tests](validation/stale-tokens.json)
· [Matched snapshot digests](validation/matched-snapshots.txt)

Start from the revision above and decompress a [source patch](patches/) before applying it. The
census instrumentation applies on top of the stale-token patch. The
[build receipts](build/) contain the argument arrays for the pinned rustc,
matched dependencies, parser probe, and exact source/work checks. Use the
[pinned parser probe](probes/parser.rs) and
[differential snapshot probe](probes/snapshots.rs). Remap the recorded checkout,
output, Cargo dependency, and allocator paths to your workspace while keeping
the recorded flags and toolchain. The Rust probe's allocator path points to
the checkout's `src/compiler_allocator.rs`.

The [executed scripts](as-run/) record the original Apple M1 workspace paths.
Run their `build` and `validate` modes after preparing the corresponding
private source copy. The combined screen uses `standalone.py measure`. The
independent screen uses `compare-candidates.py` with one excluded warmup per
actor and all six actor permutations. The timing plans bind the exact probes
and source used for the raw trials.
