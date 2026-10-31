# Checked private-frame physical cleanup — round 49

**Retain.** Complete eight-byte level-12 compression improves **14.306172× → 14.259172× Roc**, using one shared fresh 0.293 ms reference: **4.1917085 → 4.1779375 ms (−0.32853%)**. All four newly prepared pairs improve (−0.359%, −0.316%, −0.339%, −0.535%); 40/64 descriptive blocks improve. There are 3,968 primary calls / 1,984 per version, plus 16 warmup/profile calls. This is a small gain. Do not compare the multiple directly with round 48's different reference clock or multiply independent cohorts.

## Exact checkpoint and mechanism

Published parent: `01a71ae184c8e65a12833b74c503f60c5046cffe`. Stack base: `eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`. Branch: `codex/level12-compression-consolidated`; worktree `/private/tmp/mech-deflate-linear-stack`.

Tests first: `09288f43df3913cee1568bb530a128d07f09ea68`. Production change: `51345d3feea1fe1d147d3ce50c3c68d1255ff71f`. Separate CI inventory change: `8329e06957adf1c26ba8adfe5bf24cc81a185c72`. Exact optimized GPU source: `1897c74630ac57515a60d8934db61718c58fb7c7`. Core, Engine, Stdlib, Compute and all common dependencies remain the immutable round-48 after graph. Only GPU needs rebuilding; the exact retained opt3/feature/WAE flags took **22.514 seconds**. The new test counters compile out of production. IR/work/ABI/bytecode remain **131/95/37/14**.

The real compressor carries buffer-containing records through collector, search/skip, parser and Huffman tail loops. The unchanged source's optimal-collect-block is at line 603, called from complete compression at 1851. Ordinary lexical cleanup included both frame and canonical-arena retirement for every local. The existing checked frame recipe already identifies private containers. Their projected managed fields remain canonical and must keep arena cleanup.

On the fully funded path, retire a selected container in its frame bank and consume its original **32-unit arena price** on the same mutable noncloneable paid loan. Omit the empty physical arena effect. The frame method returns whether an existing recipe was actually applied, avoiding another recipe lookup. No new executor, ledger, metadata allocation, permission cache, dirty-slot ownership model or source-ID specialization is added.

A pure preview still checks every original slot/price. Recipe-bearing slots must have frame backing and an **empty canonical slot**. Unexpected canonical presence declines the entire batch. Canonical fields retain normal generation/retirement checks. Insufficient funding, active/nested/foreign scope, worker budget or uncertain proof follows the original ordered path. The original scan price, logical total, first error, spent prefix and partial cleanup remain.

## Proof boundary

The selector checks all definitions and reads, closes copies/composite projections, and rejects inputs, constants, unknown producers and escaping containers. Selected Library, Structural and Move execution stores these containers in the frame bank; successful native composition exports there, and declined composition uses the same selected ordinary actions. Owned-image restoration rederives the selection/recipes from plan facts.

This is a **guarded** storage-domain proof. Worker capture can install canonical values, so the implementation does not assert universal no-arena storage: paid batches explicitly reject worker budgets, and the current empty-slot guard conservatively handles unexpected state. Projected arrays do not inherit their parent's container domain.

Arena retirement of an empty slot does not advance generation or release ownership. A focused maximal-generation test establishes this, while the existing populated-generation overflow test retains failure and the live alias. Preview validates slot/ledger/memory identity; the synchronous loan exclusively borrows the meter, and the caller exclusively borrows both banks. Clearing frozen canonical owners cannot execute user callbacks or populate the arena. Thus the checked empty condition and context cannot change before the omitted effect. No physical slot reuse/epoch coloring is introduced; future compaction must preserve this boundary.

## Actual mechanism census

A separate copy of GPU source adds six counters only to retirement.rs. The main checkout is not modified. That overlay builds its **own matching GPU library, emitted Object/image and linked worker**; every instrumented elapsed observation is excluded from performance claims. Eight steady complete calls consistently observe:

| Counter | Per call |
| --- | ---: |
| Fully paid batches | 590 |
| Original slot attempts | 26,565 |
| Omitted empty arena effects | **13,857** |
| Remaining paid arena effects | 12,708 |
| Ordered fallback batches in this input | 0 |
| Unexpected canonical previews in this input | 0 |

This removes **52.16% of those arena effects**, not 52% of program time. Frame clearing, owner release, scans and original charges remain. Logical work stays **60,260,219,686**, comparison work **47,612**, capacity 13, exact bytes/inflate, input aliases/content and zero cleanup all agree. The two before-mechanism tests fail on the parent (three physical arena calls instead of one, and unexpected canonical presence incorrectly remains batched); ten existing charge/scope/generation controls already pass there. All twelve pass afterward.

## Qualification and failure disposition

**593 fresh distinct Rust tests pass**: 318 maintained runtime, 32 work/memory/value, 234 component, eight connected and one path mechanism. These include the three new tests; the focused 12-case run overlaps this count. The unchanged Core library retains its previous 413-test qualification (same exact hash), giving **1,006 covered distinct tests across these identified cohorts**, not 1,006 newly executed tests.

All 12 linked original/decoded assertion, arithmetic/load precedence, publication, alias, failure/cleanup/retry executions pass. Twelve scalar controls retain their original ≤1.05 latency gate and exact scalar allocation equality; eight frame-allocation cases retain zero loop slope and no request/byte/work increase. Source/component expected values and allowances remain unchanged. The expanded CI selection lists and executes **21** tests, retaining the equality guard. All 132 workflow/inventory Python tests and ten architecture/catalog/warning/format checks pass. No cleanup-domain or CI finding remains unresolved.

The only intentional failures are the two tests committed before implementation. All actual release builds, eight-byte computations, instrumented mechanism runs and final qualification pass. A redundant poll after the qualification handle had completed returned Unknown process id; successful recorded results establish terminal completion, and no job was restarted. No resource denial occurred during the measured workload.

The one inherited exclusion remains `complete_unchanged_knucleotide_body_uses_the_owned_emitted_map`, because the selected catalog lacks its stats import. No local all-features or hosted-CI pass is claimed. PR #973 remains draft until green CI and clean review; publication CI is cancelled by user direction. No unrelated repairs or new language semantics are bundled.

## Fidelity and reproduction

Source SHA-256 `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`; MBC `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`. Algorithms, tables, actual 262144-entry U16 tree / 1500513-word U32 cache, depth 300, nice length 258, original near-EOF limits, allocation sites, pinned mimalloc 2.1.7, output capacity-13 clear/reuse and evaluate_into timer are unchanged. Mech compresses itself; independent inflate remains outside the timer. Raw output is `[75,76,74,78,73,77,75,207,0,0]`.

The generated compressor Object remains `3f0c0e1954afe1083254d928a8ff66977ee9203e64bfd281326c75f9ff5f09e9`, text **13,885,696 bytes**, machine stack **108,400 bytes**. The actual identical Object/image is archived once in round 46. Real host disassembly is added here: retire_tail_locals grows **616 → 676 static sites** including fallback/error paths. Dynamic instructions still fall; static size is not the mechanism claim.

Same Apple M1 / 8 GiB / macOS 15.6.1 (24G90), Rust nightly-2026-03-03 ec818fda361ca216eb186f5cf45131bd9c776bb4 / LLVM 22.1.0. Pinned reference binary SHA `3941ef59b2c9343d0493cc6b6d14ca4803e6bd27acfe520b1fb59d9b63026324`, compiler d99c7b838fb05bb1047eb0c18d4d06d0f86f345d, package 671077457ff7ef3a3d44caf715a3fc595addf379. Only complete eight-byte compression is timed; no larger-input or parity claim.

Each timing, hardware, allocation, sample and mechanism cohort is serialized and committed immediately with exact commands, profiles, revisions, stdout/stderr and hashes. `analyze.py` and `profile_analysis.py` reproduce the scorecard and sample counts from raw receipts. [Fresh numerical profile](PROFILE.md) records the adverse diagnostic cycle reading and memory limitations. Child-exit emptiness, shorter/prepriced inventories, storage allocation, shared native bodies and numeric facts remain separate candidates.

Independent numerical replay verifies **43 archives, 405 file occurrences and 351 unique paths**, reproducing scorecard JSON/CSV and profile-number JSON byte-for-byte. Exact invocation: `python3 reconstruct.py /private/tmp/new-empty-replay-directory`.
