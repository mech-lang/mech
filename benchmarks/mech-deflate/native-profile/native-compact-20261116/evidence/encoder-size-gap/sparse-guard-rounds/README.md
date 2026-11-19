# Sparse guard-closure rounds

**Selected compiler recipe: `m1-sparse-guard-rounds`.** It produces the same
qualified **63,804 B** encoder as `m1-preserved-root-registers`, with a paired
**33.998 ms complete-build saving**, **8/8 faster**, and a 90% interval of
**30.849–38.061 ms saved**. The emitted object, complete metadata, canonical
bytecode and fixed-support benchmark worker are byte-for-byte unchanged.

| Same-session Apple M1 build | Encoder text | Source through final link | Through strip |
| --- | ---: | ---: | ---: |
| Full-scan control | 63,804 B | 2.240604 s | 2.251746 s |
| Selected sparse rounds | 63,804 B | **2.205950 s** | **2.217085 s** |
| Roc | 57,696 B | 2.977386 s | 2.988315 s |
| libdeflate | 37,080 B | 0.480327 s | 0.491054 s |

All actors have fixed support prebuilt, and every measured build includes final
linking. Eight balanced orders give each actor every position twice, with
excluded warmups. Paired build ratios are **0.74131× Roc**, **4.59590× C** and
**0.98483× the full-scan control**. Code ratios stay 1.10587× Roc and 1.72071× C.
Stripped deployment remains **276,072 B**. Independently calculated medians
need not add or equal the median paired difference.

The native phase falls **1.246281 → 1.212927 s**. Source CPU medians are
2.263389 and 2.229214 s, and peak RSS 392,355,840 and 392,683,520 B. Parsing,
source semantics, artifact creation, bytecode encoding and final linking are
all recorded per build. This is a compiler-time improvement; it does not shrink
the emitted code or establish a runtime speedup. The separate checked-codebook
phase's 14 ms effect belongs to another cohort and is not added here.

## Work removed and preserved ordering

The initial full scan retains the original guard-equation order. If it infers
nothing, closure finishes without constructing an inverse index. If it adds
facts, a lazily derived compact index identifies only the equations affected by
those facts for subsequent rounds. It uses offsets for existing related-value
positions, rather than allocating a vector for every possible global value ID.
Unchanged relation sets share the immutable index across CFG edges.

Each later round sorts and deduplicates equation positions, evaluates against
the preceding round's facts, then commits all inferences in the original order.
This preserves first-inference priority on contradictory syntactic paths,
projection-peer ordering, cyclic relations, redefinitions and intersections.
It adds no runtime permissions. Checked errors, finite admission, ownership,
accounting, SSA verification and native resource checks remain unchanged.

The eager-seed prototype constructed the index before discovering whether a
second round was needed. It was **9.011 ms slower**, 2/8 faster, with a 90%
interval of **+1.231 to +14.215 ms cost**. Its control/experiment medians were
2.249828/2.261848 s. The lazy-index repair keeps the first full scan and avoids
that setup for no-inference closures. `eager-seed/` and `lazy-index/` preserve
both source patches, raw observations, plans, compiler identities and summaries.
See `plan.md` and `lazy-index-plan.md` for the before-edit hypotheses. The
production source contains the repaired mechanism; the eager version is archived.

## Qualification and source checks

The rebuilt fixed-support benchmark worker is exactly
`5afce59aa83c03df81c693d78e9a65f17df1c88c64a7e04c490c58ecc4fffba0`.
It holds **81/81 Roc and 62/81 libdeflate in both complete M1 orders**, all
supported at the configured precision, covering 74 and 56 distinct contracts.
Those measurements are the [existing complete qualification](../preserved-root-registers/README.md),
reused through exact worker identity rather than represented as a new timing
campaign. `lazy-index/worker-identity.json` records the new link's identity.

All **29 focused tests pass**. Both schedules match the original map reference
through updates and joins (64 seeds × 1,000 steps per mode), and through
exhaustive small-truth/definition expiration cases. Tests also cover backward
chains, conflicting inferences, projection peers and unrelated equations at
larger IDs. Enabling sparse scheduling for the whole test process passes all
tests too. The existing focused target's route-reach include pointed to a moved
file; it now points to the current `route_uses/reach.rs` owner.

The named source rebuild reproduces all artifacts and passes **162 exact
output/decompression/borrowed-alias observations, six allocation-failure cases
and 12 recoveries**. Its fresh host-support worker `a3325554…` has a separate
identity and no runtime-win claim. `source-rebuild.json` and the three compressed
source-check records preserve those observations. The original Mech source is
unchanged. No LLVM tool or library dependency is introduced.

## Reproduce

From the benchmark packet directory on Apple AArch64:

```sh
python3 -B rebuild.py /tmp/mech-sparse-guard-rounds \
  --variant m1-sparse-guard-rounds --mimalloc-object /path/to/mimalloc.o
python3 -B evidence/encoder-size-gap/sparse-guard-rounds/verify-results.py
```

The named recipe pins all source, object, metadata, bytecode and layout-profile
identities. Compiler source is `0fb8fec13933d53d822c566f3e13dac4009c3dc1`,
as-run compiler `e7bb105011a9632bc5d1033494bc46f6d883f21e989df4a9e72757d3f0384eb1`.
Its process-start option is `MECH_GUARD_SPARSE_ROUNDS=1`; setting it to `0` in
that same compiler selects the full-scan control. `measure-as-run.py` in each
variant records the acquisition code and original paths; the raw observations
and portable verifier can be checked without those host paths or executables.

The full objective remains **1× libdeflate code and build time**, at least
81 Roc and 62 C wins, and eventually more C wins. This step leaves the code gap
26,724 B and this cohort's median build-time gap 1.725623 s. The small-controller
architecture remains incomplete.
