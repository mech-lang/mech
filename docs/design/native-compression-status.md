# Native compression: current status

Maintained code/tests and compact hashes stay in source. Raw observations,
profiles, failed attempts and replay scripts are in the [external evidence
index](../../benchmarks/mech-deflate/EXPERIMENTS.md).

## Current measured checkpoint: R89

Complete pinned level-12 compression of `abcdefgh` improves **9.616266× →
9.080397× Roc**, or **5.572531% less elapsed**, against the retained R87 compiler.
The target remains 1×; larger-input throughput is unestablished.

R89 base/prior published head: `21e3266e89d36d1224be050a4530625cf08f2ea2`.
Actual measured compiler source: **`460c9cb748fc377c1cbcfd57565a5373de05de14`**.
Baseline compiler: `c2b334feed4326a66ce1f69ca4bc99b0cbe41bb2`.
Separate CI inventory: `71c02cc89`; formatting-only correction: `64ed902f9`.
The latter equals formatting the pinned candidate source; it changes no Rust
behavior. Stack base: `eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`.
[PR #973](https://github.com/mech-lang/mech/pull/973) stays draft pending green CI
and clean review. Publication CI is cancelled by instruction, not counted as
passing.

| Metric | R87, freshly remeasured | R89 |
|---|---:|---:|
| Matched elapsed |2.7983335 ms|**2.6423955 ms (−5.572531%)**|
| Elapsed / fresh 0.291 ms reference |9.616266×|**9.080397×**|
| Instructions, separate serial 32-call cohort |30,535,598|**29,287,785.5 (−4.086419%)**|
| Cycles, same separate cohort |8,897,304|**8,424,351.5 (−5.315683%)**|
| Requests including reallocations |6,399|6,399|
| Requested bytes |98,074,788|98,074,788|
| Incremental peak live allocator bytes |20,473,824|**17,851,960 (−12.805932%)**|
| Generated text bytes |14,920,564|15,032,784 (+112,220)|
| Entry stack bytes |133,984|134,816 (+832)|

Four prepared pairs contain 3,968 primary/3,984 total observations. All four pairs
and all 64 descriptive blocks improve; no significance claim. Use this round's
matched baseline, not ratios from another reference-clock cohort. Fresh reference
hardware medians are 1,364,372 instructions/925,595.5 cycles; its allocator has 95
requests/9,021,923 requested bytes/9,257,328 incremental peak live bytes. These are
separate cohorts, not exact per-function instruction accounts or semantic minima.

The smaller complete-call live heap peak does not mean fewer allocation requests,
less requested capacity, or less copied data. Large-request histograms and 162
reallocations are unchanged. Whole-executable RSS is slightly adverse (+180,224
bytes); physical footprint differs by −32,768 bytes. There is no broad RSS win.

## What changed

A constant named field returning managed data previously forced a parent native
piece through ordinary field access. The compiler now uses the existing exact,
unambiguous selector proof and prepared library factory to retain that current
owner as a private alias. Structural aliases and scalar specialization remain.
No source-site names or compression-specific rules enter production code.

Every entry still validates current source schemas/accounts/availability, alias
types and empty output slots, funding and reached retirement. New library aliases
retain explicit finite call limits and declared output cardinality/byte limits.
A sufficient bound uses current admitted source capacity. A tight/refused bound
silently declines before spending, so the ordinary call checks its exact current
footprint at the lexical site. No untaken check rejects an invocation.

Real escapes retain ordinary `arena.put`; only internal aliases avoid canonical
installation. Existing current owners are pinned before ordered retirement and
publication, then cleaned on failure and retry. Every child boundary stays
ordinary: no parent loan/view survives a call. Original/decoded/restored images
rederive selector, layout, current-owner and exit-use proofs.

IR/work/private ABI/MBC are **136/97/38/14**, scalar ABI1. Policy97 explicitly
prices the representation that omits physical name lookup and intermediate alias
transactions. It does not claim identical old physical-resource error prefixes.
Actual region admission remains finite; source errors and declared output limits
stay checked. Work changes **45,853,669,270 → 42,869,883,099**; comparison work is
**23,004 → 23,774**, an adverse change caused by some scalar calls returning to
ordinary handling. Neither counter is a hardware instruction count. No allowance,
output golden, unsafe allowlist, warning or architecture policy is weakened.

Four isolated full-call censuses agree for each subject:

| Executed mechanism | Before | After |
|---|---:|---:|
| Named managed field callbacks |349|87|
| All host helper callbacks |5,188|4,959|
| Ordinary library callbacks |1,009|792|
| Source admission callbacks |6,319|6,034|
| Canonical arena puts |2,178|2,047|
| Successful native pieces |810|803|

Thus 262 managed projections avoid their callback; 131 arena installations
also disappear. This is less than the earlier 323-visit candidate ceiling.
Scalar field callbacks rise 25→112, so all column callbacks fall only 374→199.
Measured complete-call instructions, not candidate counts, establish the win.
Static emitted pieces increase 384→387; more metadata alone is not acceptance.

## Fresh profiling and next bounded target

Fresh serial Mech profiles contain 695 complete calls each, with 1,456/1,310
exclusive sample weights. Frame-action ancestry falls 231→161; native-entry
body weights are 203/184. Entry preview, source admission and cleanup remain
visible. These are CPU samples, not retired-instruction fractions; ancestry
unions overlap. A fresh 20,000-call reference profile has 1,520 weights, dominated
by one opaque native symbol (1,432); no unsupported source-name attribution.

A concrete optimization conflict remains: **97 fully proved pieces are skipped
because their entry already belongs to an ordinary scalar admission group**.
They contain **264 currently executed library callbacks**. In parent25, the
31-site piece beginning2366 is skipped; its old scalar-only pieces at2383/2394
were emitted before the managed projection expansion. It accounts for136 current
library callbacks. The source gate and serialized admission layout establish
this refusal; it is not a limit or failed numeric/ownership proof.

Next establish a minimal changing-buffer/child reproducer for that overlap, then
resolve or split it through the existing selection/admission machinery. Preserve
the efficient scalar fallback, same account, error order and finite recovery.
Do not remove the gate without settling the scalar admission, add another ledger,
or reopen ordinary debit-function tuning. No next compiler change is included
in this checkpoint and no speedup is forecast from the 264 callbacks.

## Fidelity and qualification

The [25-component assembly](../../hosts/gpu/tests/fixtures/native/deflate_assembly.rs)
performs all compression as one compiled computation. Source SHA-256
`331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`;
MBC SHA-256
`a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`.
Algorithm/tables/source allocation sites/depth300/nice258/near-EOF checks,
tree262144 U16/cache1500513 U32/path305257 U64, input aliases, pinned allocator,
capacity13 clear/rebind and evaluate_into-only timer remain unchanged. Search
and skip stay separate; skip has no cache/recording path. Bytes
`[75,76,74,78,73,77,75,207,0,0]` and independent inflation agree. Final managed
bytes/nodes are zero. Physical public-output backing still changes.

Fresh overlapping passing selections: runtime345; work/memory/values33;
native CI inventory and execution51; image1; unchanged component goldens and
allowances236; connected8; path1; existing linked/AOT original+decoded16;
new named-projection linked/AOT14; scalar12 (maximum1.042987915 against unchanged
1.05, pure-scalar allocation equality and zero loop allocation slope); frame8;
workflow Python132; architecture/type/memory15. GPU-only release and optimized
unit cfg use the existing coherent selected-operation dependency graph. The
runtime-only GPU cfg also compiles warnings-as-errors against that graph.
Core/Budget/Engine integration remains inherited; no math or fresh minimal/no_std
rebuild is claimed.

Four new maintained tests cover native selection, changing lengths/arguments,
recursion, aliases, original/decoded/restored execution, success/failure/retry,
unchanged prior publication and exact cleanup. They cover assertion-before-math
and math-before-assertion, assertion versus empty load, unselected division,
missing/tight output policies and six finite resource boundaries. Architecture protections remain intact; no unrelated package is reopened.

An initial typed-ID build error was fixed before the source commit. A formatting
check then found one reflow; a separate style commit fixes it and all 15 checks
pass. An invalid whitespace-diff equivalence predicate in the correction driver
was replaced by formatting the exact pinned source. Failed commands and the
reconstructed failed patch remain external. No required local failure remains.

The first hardware/control pass overlapped the tail of static checks. It is
preserved, audited, and excluded from selected counter/control claims; explicit
serial repeats pass. Both scalar passes remain below1.05. The primary paired
elapsed cohort ran separately. Larger/full-corpus timing, giant complete
knucleotide, hosted green CI and a fresh minimal/no_std graph remain skipped.

R87's current-capacity proof, prior structural work, and all useful retained
changes remain. R88's 673-fewer-payment experiment stays rejected/reverted
because elapsed and instructions were adverse. All historical failed/adverse
observations remain externally retrievable; source keeps one status, compact
index and maintained harness, with no history rewrite.

Independent retrieval verifies all **29 packets /2,306 file versions /1,264 paths**,
including49 paths with preserved multiple versions. All five numerical reports
reproduce byte-identically without compiling or compressing.
