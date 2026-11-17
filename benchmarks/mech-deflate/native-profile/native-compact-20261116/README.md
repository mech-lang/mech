# Native compact compiler review handoff

This PR contains compiler and parser source, a pinned Mech program, native build
configuration, measured runtime source, and raw benchmark evidence. It is stacked
on #973 at `9087b1f07727a9bfcb7c09059f48edaca6dc2301`.

Start with [the current scorecard](CURRENT.md) and [the review and continuation handoff](REVIEW.md).
It identifies the retained baseline, review priorities, unresolved CI, and all
24 static size screens and their subsequent runtime follow-ups, with
checkout-relative reproduction commands. No smaller candidate has been promoted.

The parser now replays admitted one-shot variable definitions with fresh
identities. Matched Mech deployment builds take **3.13566 s**
through the final link. The generated 71,768-byte encoder, metadata and
timed benchmark worker reproduce exactly.
[Parser source, finite-limit checks and complete builds](evidence/scoped-definition-replay/README.md)
The [qualification policy and validator](QUALIFICATION.md) run on new campaigns
and replay the archived controls against the same fixed 81-case contract.

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

The opt-in frame-copy variant emits **71,192 B**, or **1.234× Roc** and
**1.920× libdeflate**. Its two full orders preserve every individual
control win. They record **81/81 Roc** in both orders and **57/81 and
61/81 libdeflate**. The validator rejects this campaign on the 16 KiB
periodic case. The exact 71,768-byte control also varies from its previous
full orders, recording 57 and 60 libdeflate wins in this comparison.
[Complete orders and replay](evidence/frame-copies-71192-runtime/README.md)

The current opt-in **71,768-byte** Cranelift encoder uses
**1.244× Roc** and **1.935× libdeflate** text. Single-piece source
access brings the matched Apple M1 build through the final link to
**3.93889 s**, or **1.295× Roc** and
**8.115× libdeflate**. Both fresh full runtime orders
record **81/81 Roc and 61/81 libdeflate**, with every individual win
preserved and one duplicate-control repeatability case open.
[Matched build and parser checks](evidence/contiguous-syntax-source/README.md)
· [Bound runtime orders](evidence/bound-71768-runtime/README.md)

Reusing the parser's subtree frame allocation cuts the matched Apple M1
source-through-link build from **4.01608 to 3.95160 seconds**. The exact
71,768-byte object, complete metadata, and linked benchmark worker reproduce
with the recorded 81/81 Roc and 61/81 libdeflate results. The complete build
is **1.323× Roc** and **8.223× libdeflate**.
[Parser checks and matched builds](evidence/parser-cache-frames/README.md)

The AArch64 checked-root branch recipe emits **71,768 B** of encoder text,
**1.24390× Roc** and **1.93549× libdeflate**. It saves 192 bytes and
reduces measured peak native stack to 5,280 bytes. Its matched Apple M1
source-through-link build is **4.00611 s**. It stays opt-in for runtime
review. [Rebuild, both full timing orders, and per-case bounds](evidence/branch-on-zero/README.md)

The ordered-scoring vector address experiment grew the encoder by 64 bytes
and was rejected after the empty-input slowdown. Its patch, measured stack
traffic, raw observations, and object reproduction are available for review.
[Ordered scoring experiment](evidence/ordered-vector-cursors/README.md)

The opt-in direct source rebuild compiles the 71,960-byte encoder through
the final link in **3.99930 s** on Apple M1, down from **4.08628 s**
in the matched split build. It takes **1.339× Roc** and
**8.292× libdeflate** time. Both paths reproduce the pinned workers.
The direct compiler uses about 19 MB more peak RSS.
[Rebuild command, matched trials, and exact worker checks](evidence/direct-source/README.md)

The preceding native emitter study builds the 71,960-byte recipe from source through
final link in **4.07374 s** on Apple M1, with identical encoder
objects and benchmark workers. The complete build takes **1.366× Roc**
and **8.478× libdeflate** time.
[Current native compiler results](evidence/native-pass-work-reuse/README.md)

The canonical parser storage update reduces the smaller Mech encoder's matched
source-through-link build to **4.18694 s** on Apple M1. Its
complete build takes **1.402× Roc** and
**8.718× libdeflate** time. The pinned benchmark workers
match byte for byte. [Current parser results](evidence/compact-parser-storage/README.md)

The accepted runtime floors are **81/81 against Roc and 61/81 against libdeflate**.
62 is an improvement target. The older raw reports retain the former 62 floor;
that wording is historical. Historical evidence has not been edited.

| Apple M1 evidence | Mech code | × Roc code | × libdeflate code | Runtime qualification |
| --- | ---: | ---: | ---: | --- |
| Retained native baseline | 72,520 B | 1.25693 | 1.95577 | 81/81 and 61/81, both full orders |
| Fresh owner counts and cold-tail sharing | 71,960 B | 1.24723 | 1.94067 | 81/81 and 61/81, both full orders |
| Fresh owner reference counts | 72,344 B | 1.25388 | 1.95102 | 81/81 and 61/81, both full orders |
| Optional cold-tail sharing | 72,200 B | 1.25139 | 1.94714 | 162 exact encoder calls; 15-case screen only |
| [Aligned register spill pairs](evidence/aligned-register-spills/README.md) | 71,896 B | 1.24612 | 1.93894 | 81/81 and 61/81 in both full orders, rejected on two cases |

The code denominators are 57,696 B for Roc and **37,080 B for libdeflate**.
Historical receipts used a 41,112-byte C object for their size ratios.

## Compiler optimization profile

Rebuild either encoder with `rebuild.py --compiler-opt-level 3 --target-dir PATH`
in a separate Cargo directory. Level 1 remains the pinned default. Both host
profiles use the same mimalloc C archive. The Cranelift backend, native flags,
benchmark allocator, fixed runtime, adapters, and final link are held constant.

| Apple M1 build | Source to executable | × Roc time | × libdeflate time | Encoder text |
| --- | ---: | ---: | ---: | ---: |
| Retained Mech, level 1 | 5.64000 s | 1.88910 | 11.71036 | 72,520 B |
| Retained Mech, level 3 | 4.75709 s | 1.59337 | 9.87717 | 72,520 B |
| 71,960-byte Mech, level 1 | 5.64395 s | 1.89042 | 11.71857 | 71,960 B |
| 71,960-byte Mech, level 3 | 4.76479 s | 1.59595 | 9.89315 | 71,960 B |
| Roc | 2.98555 s | 1.00000 | 6.19891 | 57,696 B |
| libdeflate | 0.48162 s | 0.16132 | 1.00000 | 37,080 B |

The smaller encoder builds **15.58% faster** at level 3. Six balanced
orders include fresh source parsing, code generation, and final linking.
The medians exclude warmups and separate profiled trials. Bytecode, encoder
objects, complete metadata, and linked executables are byte-identical within
each encoder configuration. Both historical benchmark workers also reproduce
exactly, carrying their measured **81/81 Roc and 61/81 libdeflate** scores in
both orders. Both configurations pass 162 exact checks at each compiler level.

At level 3 the retained frontend takes **2.950 s** including
**2.618 s parsing**. Native emission takes **1.765 s**
and the final link takes **0.042 s**. The profiled frontend and
emitter peak RSS are **382,222,336 B**
and **217,825,280 B**.
The compiler built 14 vector alternatives and selected five. The native marker
spans include all intervening work and can guide narrower pass measurements.

Compiler construction is separate. The clean level-3 trial initially optimized
the allocator too and took 1300.53 seconds.
The allocator-pinned correction rebuilt affected units in
5.98 seconds from that cache.
The level-1 control reused its existing compiler cache. A matched cold
construction measurement remains open.
[Build timings, full sizes, profiles, and identity proofs](evidence/host-opt-levels/report.json)

The link maps reproduce all four current timed executables byte for byte.
Mech's retained linked text includes **237,384 B** from Rust standard libraries,
**52,156 B** from the benchmark adapter objects, **29,928 B** from JSON and
serialization libraries, and **14,044 B** from inflation support. The maps
attribute input symbols and include compiler-generated instances in the object
that publishes them. [Benchmark link maps and file-byte census](evidence/linked-size-census/report.json)

The common raw-DEFLATE deployment CLI measures **575,632 B Mech**,
**191,920 B Roc** and **155,760 B libdeflate** after identical
dead stripping and symbol stripping. All six links produce the exact raw
streams for all 81 cases and inflate independently. Mech's minimal link
still includes 199,224 B of linked Rust standard-library code.
[Deployment source, links and verification](evidence/minimal-deployment/README.md)

A bounded Mech host uses Rust core and alloc with the same encoder and
shared deployment link settings. It measures **292,576 B stripped** and
passes all 81 exact deployment checks and six compiled-provider failure
cases. A new matched compile-to-deployment comparison gives **3.73555 s**
Mech, 2.99608 s Roc and 0.48811 s libdeflate.
[Smaller host, independent timing and reproduction](evidence/alloc-only-deployment/README.md)

## Preselected root inlining

Use the opt-in AArch64 diagnostic build to skip the first machine compilation
of the checked root while keeping the existing IR optimizer and verifier.

```sh
python3 rebuild.py /tmp/mech-native-preselected \
  --compiler-opt-level 3 --variant fresh-owner-counts-cold-tails \
  --preselect-root-inlining
```

Run this command from this benchmark packet directory. The rebuild verifies
the pinned object and metadata. Supply `--mimalloc-object` with the fixed
allocator object to link the worker and check all 81 named fixtures.
[Matched build trials and exact worker identities](evidence/native-root-inline-plan/report.json)

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

## Batched block parameter cleanup

Constant-phi, local-phi, and dead-value cleanup now reindex surviving block
parameters in one pass. The existing proofs, alias order, and incoming
argument order are preserved. The constant-phi solver also joins incoming
facts in place. The earlier M3 trial on #973 batched constant-phi removal
alone and measured a 0.19% increase in complete build time.
[Earlier trial](../../../../docs/design/native-batch-phi-params-compile-trial.md)

| Matched Apple M1 build | Source to executable | Encoder text |
| --- | ---: | ---: |
| Previous Mech emitter | 5.65167 s | 72,520 B |
| Mech with batched cleanup | 5.64176 s | 72,520 B |
| Roc | 2.98782 s | 57,696 B |
| libdeflate | 0.48359 s | 37,080 B |

The matched complete build improves by **0.18%**, reaching **1.88825× Roc**
and **11.66649× libdeflate**. Eight balanced native-only pairs move the
median from **2.22625 to 2.21648 seconds**. Separate instrumented runs move
the interval from initial CFG cleanup through entry into vector guard
analysis from 0.44633 to 0.42784 seconds.

All twelve complete Mech builds produced identical bytecode, encoder objects,
metadata, and executables. The 207 Cranelift unit tests pass, including parameter
order, reindexing, attachment, and aliasing coverage. The compiler reproduces the
original pinned worker byte for byte and the fresh-owner object and metadata
exactly. Both linked rebuilds pass 162 correctness checks.
[Build trials, profiles, and rebuilds](evidence/batched-block-params/report.json)

## Smaller continuation configurations

The nine screens vary the continuation input budget and combine it with fresh
owner counts and cold-tail sharing. The 71,960-byte configuration enables those
two passes with the retained input budget. It cuts **560 bytes (0.77%)** of
encoder text and keeps **81/81 Roc and 61/81 libdeflate** in both full orders.
Each order checks 982,656 timed calls. The configuration also passes six
checked-failure cases, 14 shape controls, and 162 exact linked calls. Peak stack
remains 5,296 bytes.

In four balanced Apple M1 build orders, this configuration takes **5.65399 s**
through final linking, or **1.88967× Roc** and **11.76562× libdeflate**.
The simultaneous Mech control takes 5.63752 s, a 0.29% difference.

Paired intervals bound 77 cases at 0.5% resolution. Longer batches with duplicate
controls resolve `prefix-repeated-4096`. Control variation exceeds the resolution
on `one-stored`, `seven-stored-prefix`, and `random-64`, so the 71,960-byte
configuration is opt-in as `rebuild.py --variant fresh-owner-counts-cold-tails`.

The 71,720-byte configuration also limits direct continuation inputs to six
words. It loses `320-overlap` and `prefix-periodic-1024` against libdeflate in
both focused orders. Its reproduction case is
`size_screens.py --case continuation-widths--width-6-owners-cold`.
[Static screens, paired runs, and exact rebuilds](evidence/continuation-widths/report.json)

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

## Parser journal pages

The parser groups events into 32-entry pages and borrows each page during tree
construction. The sink hashes contiguous token bytes while charging the original
work allowance per byte. Cached subtrees skip an event map that only the final
diagnostic sink needs. A boundary test resumes within multibyte characters across
source pieces and verifies the same work, token hash, and node identities.

Six balanced parser-only pairs on the pinned 2,719-line source move the median
from **3.51713 to 2.92567 seconds**, a **16.82% reduction**. Peak process RSS
falls from 130,539,520 to 115,974,144 bytes. The complete syntax tree and IDs,
3,967,302 parser steps, and 718,353 events match in all twelve trials. All **531
syntax unit tests** pass, including the existing streaming and limit tests.
The earlier page-size screen compares four, eight, sixteen, and thirty-two
entries with the same batched implementation.

| Matched Apple M1 builds | Source to executable | Encoder text |
| --- | ---: | ---: |
| Previous Mech frontend | 6.30298 s | 72,520 B |
| Mech with journal pages | 5.65845 s | 72,520 B |
| Roc | 2.98664 s | 57,696 B |
| libdeflate | 0.48728 s | 37,080 B |

The complete Mech build takes **10.23% less time**, **1.89458× Roc** and
**11.61221× libdeflate**. Parsing accounts for 2.930 s of the new build, with
3.383 s for the whole frontend, 2.232 s for native emission, and 0.040 s for
final linking. Each actor occupies every position of four timed orders, with
compilers, runtimes, and adapters prebuilt. Warmups and separate instrumented
builds are excluded from these medians.

All twelve Mech builds produce the same bytecode, object, metadata, and
executable, including the warmups and profiles. Both build variants pass 162
exact-output, alias, and inflation checks. The default and fresh-owner rebuild
commands also reproduce their respective objects and metadata exactly and pass
162 checks each.

The retained worker and encoder object are reproduced byte for byte. That keeps
the earlier 81/81 Roc and 61/81 libdeflate results attached to the same benchmark
binary. [Parser and build trials, test results, and exact rebuilds](evidence/parser-journal-pages/report.json)
are included for review.

The native policy workflow keeps twelve minutes of scheduling headroom after
adding the fresh owner test. The focused workflow contract passes locally.

## Fresh owner reference counts

The optional compiler pass folds 20 reference-count loads in the compact root.
A successful allocation with nonzero capacity and stride initializes its count
to zero. The pass follows direct allocation results and exact count stores, and
requires agreement at joins. It stops at potentially aliasing stores, unrecognized
calls, and cycles. It preserves the allocation, count writes, and checked failure
paths. Five native test groups exercise 2,592 calls across the proof boundaries.

The standalone pass reduces emitted text by **176 bytes** to **72,344 B**. Both
full peer benchmark orders record **81/81 Roc and 61/81 libdeflate** for this
candidate and the control. Its peak stack remains 5,296 bytes. It passes 162
exact-output checks, six checked-failure cases, and 14 shape controls. The full
orders verify 1,965,312 timed calls. The initial paired analysis bounds 77 cases
at 0.5% resolution and has four inconclusive cases. The report includes longer
measurements with a duplicate control for those cases. `heldout-window-32767`
is bounded in the follow-up. The duplicate controls vary beyond the resolution
on `random-64`, `reymont-prefix-32769`, and `prefix-natural-16384`.

The pass remains opt-in. The default configuration reproduces the retained
object, metadata, and historical benchmark worker byte for byte. Both the
standalone pass and its cleanup combination reproduce their objects and metadata
through `rebuild.py` and pass 162 checks each. The combination reaches 72,024 B
and consistently slows `16384-distribution-change` and `heldout-mixed-131073`
in both focused orders. The standalone pass avoids those focused regressions.

| Matched Apple M1 builds | Source to executable | Encoder text | Executable file |
| --- | ---: | ---: | ---: |
| Retained Mech control | 6.25214 s | 72,520 B | 956,056 B |
| Mech with fresh owner counts | 6.24649 s | 72,344 B | 956,056 B |
| Roc | 2.98377 s | 57,696 B | 230,600 B |
| libdeflate | 0.48196 s | 37,080 B | 210,168 B |

The candidate build takes 0.99910× the control, 2.09349× Roc, and 12.96053×
libdeflate. The timing is effectively flat. Final linking is included, with the
compilers, runtimes, and host adapters prebuilt. Four balanced orders follow one
excluded warmup for each actor. Bytecode is identical across both Mech variants.
Each variant reproduces its object, metadata, and executable in all five builds
and passes 162 correctness checks on its newly linked worker.

The candidate medians include 3.523 s parsing, 3.973 s for the whole frontend,
2.233 s for native emission, and 0.041 s for final linking. A separate instrumented
run shows 0.777 s between the native root cleanup and machine code stages and
0.221 s for root machine code. Roc's instrumented build spends 1.651 s on
specialization and 0.811 s on LLVM optimization and emission. Clang reports
0.085 s in the frontend and 0.441 s in the backend for the profiled C build.
The instrumented runs are excluded from the build medians.

See [the fresh owner report](evidence/fresh-owner-counts/report.json) for the
measurements, qualification status, and exact artifact identities.

```sh
python3 benchmarks/mech-deflate/native-profile/native-compact-20261116/rebuild.py /tmp/native-fresh-owners --variant fresh-owner-counts
cargo +nightly-2026-03-03 test --locked --manifest-path benchmarks/mech-deflate/native-profile/Cargo.toml --release --test fresh_owner_counts
```

## Retained parser change

The retained parser change measured **7.59864 → 7.21202 seconds** from the same
source through a runnable executable (5.09% faster). Matched Roc and libdeflate
source-to-executable medians were 2.98588 and 0.48001 seconds. Compilers, runtimes,
and adapters were prebuilt. The Mech compiler was Rust opt-level 1 without PGO.
The parser change passed 530 tests and preserved bytecode, object, complete
metadata, and executable byte for byte. These are M1 measurements, separate from
#973's M3/LLVM result.

## Compiler allocator comparison

The standalone frontend now imports the same existing compiler allocator as
`src/bin/mech.rs` and the compact native emitter. In a matched Apple M1 comparison,
this reduces complete source-to-executable time from **6.84496 to 6.24081 seconds
(8.83%)**, with identical compiler optimization settings and dependencies.
Parsing falls from 4.06316 to 3.52152 seconds. There is no encoder size change.

| Same-run complete build | Median seconds | Mech / peer |
| --- | ---: | ---: |
| Mech with the compiler allocator | 6.24081 | 1.00000 |
| Roc | 2.97982 | 2.09436 |
| libdeflate | 0.47988 | 13.00485 |

All four actors include the final executable link, with their compilers, runtimes,
and host adapters prebuilt. One warmup per actor is excluded; four balanced orders
place every actor in every position once. All ten Mech invocations (including
warmups) produce identical bytecode, encoder objects, complete metadata and linked
executables. The worker passes 162 exact-output/alias/inflation checks. These
checks establish unchanged output from the allocator correction, not newly
measured peer runtime scores. The historical 81/61 evidence remains tied to its
original benchmark worker.

`evidence/compiler-allocator` contains every timed command, phase log, individual
run receipt, source/worker hashes, and the as-run measurement scripts. Those
scripts preserve original machine paths as provenance; `rebuild.py` remains the
checkout-relative reproduction entry. The 5.67-second cached frontend rebuild
in the build log is excluded and is not a cold full-compiler build measurement.

## Owner-proof runtime follow-up

Two smaller configurations have now been linked and measured. **Neither replaces
the retained 72,520-byte recipe.** All raw results, including adverse observations,
are in [the qualification report](evidence/owner-proof-runtime/report.json).

The 72,168-byte configuration with broader owner proofs and only helper 4879
inlined passed 162 correctness calls and reduced measured peak stack use from
5,296 to 5,232 bytes. It nevertheless lost libdeflate wins in both focused orders
and consistently slowed several cases, including `320-overlap` by 3.73% and
5.08%. It is rejected.

The narrower `ranges` proof produces **72,392 bytes**, or **1.25471× Roc and
1.95232× libdeflate**, with unchanged 5,296-byte peak stack use. Both complete
orders retained 81 Roc wins. Libdeflate counts were **61 then 60 for both control
and candidate**, with no candidate-only lost wins. A longer `320-overlap` follow-up
won against C in both orders, but does not replace the original full-run counts.
The initial per-case uncertainty analysis bounded 74 cases at the declared 0.5%
resolution; seven required follow-up. Additional same-worker measurements still
leave tiny-input repeatability unresolved. The first one-byte follow-up's adverse
estimate is retained alongside its better repeat. This is not a newly qualified
81/61 result.

| Latest matched complete build | Median seconds | Encoder text |
| --- | ---: | ---: |
| Retained Mech control | 6.24346 | 72,520 B |
| Mech `owner-ranges`, not promoted | 6.26348 | 72,392 B |
| Roc | 2.99064 | 57,696 B |
| libdeflate | 0.49022 | 37,080 B |

This repeats the same complete source-to-executable boundary, with one excluded
warmup and four balanced orders. The candidate takes 1.00321× its control,
2.09436× Roc, and 12.77679× libdeflate. The retained control takes 12.73596×
libdeflate. The smaller object did not demonstrate a build-time improvement.
The linked text shrinks by 128 bytes, while the complete executable file remains
956,056 bytes. All five builds of each Mech variant reproduce their own bytecode,
object, metadata, and executable exactly; both linked variants pass 324 combined
correctness calls. Freshly linked workers do not inherit the pinned workers'
runtime scores.

For review, `rebuild.py --variant owner-ranges` reproduces that experimental
object and complete metadata exactly and can link/check it with the same
`--mimalloc-object` option. This path has been exercised with 162 correctness
calls. All three rebuild variants now check complete metadata identity as well
as object identity. The default remains the retained baseline. Raw runtime
measurements, repeatability studies, build commands and phase logs, integrated
rebuild receipts, and as-run analysis scripts are in `evidence/owner-proof-runtime`.
The as-run scripts retain original machine paths; the advertised rebuild command
uses paths relative to this checkout.

## Owner-proof and cleanup experiment

This experiment combines owner proofs with final control-flow
cleanup, using the compiler source already on this branch. It produces
**72,136 bytes of encoder text**, 384 bytes below the retained baseline, or
**1.25028× Roc and 1.94542× libdeflate**. The root frame grows from 3,744 to
3,792 bytes; this is not a measurement of complete stack high-water.

It passes 162 exact-output/alias/inflation checks. Both 12-case timing orders
record 12 Roc wins and eight libdeflate wins for both control and candidate,
covering 1,428,480 checked timed calls. The candidate/control geometric means
are 0.99775× and 0.99628×. The first order's one-byte case is 0.675% slower,
while the reverse order is 1.981% faster. Both observations are retained; this
screen does not establish per-case nonregression at the declared 0.5% resolution.

Both subsequent full orders reach 81/81 against Roc and 61/81 against libdeflate
for both actors. The per-case analysis finds five slowdowns above 0.5%, including
a 2.84% estimate for `16384-distribution-change`. Peak stack grows to 5,344 bytes.
The benchmark therefore rejects this optimization. See the
[initial screen](evidence/owner-cleanup-focus/report.json) and the
[full qualification](evidence/owner-cleanup-full/report.json) for raw results,
checked failures, and stack controls. `replay.py` checks both sets of wins.
Use `size_screens.py --case combinations--owners-cleanup` with the output and
emitter arguments documented in `REVIEW.md` to reproduce the exact object and
metadata. The single emission duration in its receipt is not a complete build
benchmark; no compile-time improvement is claimed for this experiment.

## What to review

- Compiler changes under `hosts/gpu/src/native_artifact/program`: checked regions,
  narrower live packets, direct/register outputs, shared array growth, owner
  proofs, and cleanup passes. They remain opt-in diagnostic policies.
- Vendored native backend: spill forwarding, measured leaf layout, and optional
  sharing of identical cold machine-code tails. Scalar operations keep direct
  native instructions. No external LLVM executable is used by this recipe.
- `src/syntax/src/document/parser/journal/mod.rs`: retain the current journal page
  directly while preserving shared snapshots.
- The maintained allocation policy lives in `compact_runtime/zero_pages.rs`.
  The exact historical runtime remains at the revision pinned in the recipe.
  [Recorded runtime source](https://github.com/mech-lang/mech/tree/3ff13b6ab4834770aa422fa26887c7ff8cef6b0e/benchmarks/mech-deflate/native-profile/native-compact-20261116/recorded-runtime)

Cold-tail sharing is **not promoted as a new 81/61 result**. Its two focused
orders preserve the screen's peer wins, but tiny inputs have adverse estimates.
Those observations and all raw paired blocks are included. The maintained native
tests pass 10,240 calls and check call/relocation/trap/unwind metadata; warm-only,
debug-location, disassembly, and embedded-literal controls stay unchanged.

## Rebuild and replay

On Apple ARM64 with Rust `nightly-2026-03-03`, from a checkout of this PR:

```sh
python3 benchmarks/mech-deflate/native-profile/native-compact-20261116/replay.py > /tmp/native-replay.json
python3 benchmarks/mech-deflate/native-profile/native-compact-20261116/rebuild.py /tmp/native-baseline
cargo +nightly-2026-03-03 test --manifest-path benchmarks/mech-deflate/native-profile/Cargo.toml --release --test native_compact_cold_tails -- --nocapture
```

The portable backend checks have a maintained owner in the Linux
`canonical-policy` CI job. It runs `isa::aarch64::cold_` with the ARM64 backend
and requires exactly **12 passed, zero failed, zero ignored**: eight epilogue
checks plus four cold-tail checks. They cover signed branch limits, independently
reachable interior labels, distinct origins, relocation/call/trap remapping,
recorded deferred traps, unsupported relocations, and hint reset. The same
command passed locally; the source hashes and log are in `evidence/integration`.

The last command must execute **two tests** on Apple ARM64. The cold-tail test is
also registered in the `mech-gpu` library's native test module. Other hosts do not
qualify execution of Apple ARM64 machine code.

`rebuild.py --variant cold-tails` enables the new optional pass. The default
regenerates the retained baseline. Compiler construction and source-to-object
timings are separate. No timing result from this command is a new runtime score.
An optional `--mimalloc-object PATH` links the recorded worker and runs 162 exact
calls using the included fixtures. Use the same allocator object as the peer
workers for a runtime comparison. This option needs a Git checkout containing
the recorded runtime's pinned ancestor. In a shallow checkout, fetch the
history containing `3ff13b6ab4834770aa422fa26887c7ff8cef6b0e`. The rebuild
verifies all three source hashes and extracts the pinned files into its output
directory before linking. The source-to-object rebuild runs without Git history.

The complete integrated compiler build **succeeded at
`2b930a8dfa0f085d46d68bc6593bd182b58f26a2`**. An audit compared 2,891 source and
configuration files with that commit, with no differences. Both variants
reproduce their canonical bytecode, native object, and complete metadata exactly:
72,520 bytes for the retained baseline and 72,200 bytes with cold-tail sharing.
Each newly linked worker passes 162 exact-output, alias, and independent-inflation
checks. The advertised Cargo native test target also passes both tests, exercising
10,240 native calls. See `evidence/integration/integrated-rebuild.json` and its
raw build/test receipts.

The new worker executables have different hashes from the workers used for the
historical timings. Their correctness and encoder-object identity are verified;
the historical 81/61 scores are not a fresh timing result for these linked workers.
The single reproduction runs are not matched build-time benchmarks, and their
compiler-build duration includes reuse of previously compiled dependencies.

On that commit, the engine and GPU owner jobs passed, as did all 12 portable
backend tests on Linux. CI then exposed two test-only integration errors: the
non-macOS mapping helper recursed, and a UTF-8 test omitted the new phase-sharing
argument. Both are corrected in this follow-up; all 18 maintained runtime tests
pass locally. Full Linux native-policy execution still needs the next CI run.
The warning checker now accepts the documented focused-test exceptions and stops
at an inherited example (`native_tagged_map_allocations.rs`). Distribution's
404-versus-402 dependency pin and the public CLI's exact test selection also
remain unresolved. See `evidence/integration/ci-status.json`; whole-stack green
CI is not claimed.

The compact entry remains diagnostic: exact ordinary admission/accounting and
publication recovery are not claimed. This does not change the normal language
semantics or introduce an interpreter fallback. Error status/origin, alias,
allocation and stack checks are published in `evidence/baseline`.

Raw receipts preserve original machine paths as provenance; the rebuild recipe
uses paths relative to this checkout. `source-and-evidence-manifest.json` records
the published source/evidence hashes and integration status.
