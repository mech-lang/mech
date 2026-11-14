# Qualify short-call timing precision and retain the faster native candidate (R219)

**The platform timer advances in125/3ns ticks. A5% gate on an80–300ns program can be smaller than one tick. Batched request delivery outside the unchanged per-call timer, more samples and a targeted clock-resolution recheck qualify BOTH compiler variants within the original1.05 median bound. The faster R217 variant becomes the retained experimental baseline; normalR215/IR215 stays unchanged.**

No compiler, runtime, reference executable or source changes in this qualification. It is separately reviewable from R218’s one-file compiler experiment. This is not parity or qualification of the full normal entry.

## Evidence for the measurement problem

The original R218 one-byte cohort has250→291ns medians but283.625→285.390625ns means. Actual mach_timebase_info is numer125/denom3. A deterministic synthetic test measures known269ns and272ns durations at1000 uniformly spaced timer phases: per-call median ratio1.166667 falsely crosses1.05, while mean ratio1.011152 preserves the known change. A real330ns case still fails1.05. No timing overhead is subtracted or threshold enlarged.

R217’s earlier empty failure also does not persist: its unchanged executable passes all20 old median gates in the fresh R218 original four-way cohort. These facts do not prove every earlier small-case slowdown false; they require better precision. R216’s pool remains unretained and was not rerun here.

## Qualified protocol

All four existing binaries and all20 fixtures run another24 balanced side orders. Send each block’s N commands together; each existing worker still clears/prepares before its own original timer and verifies after it. This removes per-call pipe wakeups outside the timer. Actual compressor operations, aliases, allocations, cleanup and timer endpoints do not change.64 samples/block for the existing small-case class, one for large cases; retain every original per-call median and a descriptive median of block arithmetic means.

The all20 secondary cohort supplies86752 verified complete calls. A rule independent of program names refines every case where one platform tick exceeds5% of the retained median: it selects exactly empty,one-stored,seven-stored-prefix. Each gets512 samples/block /24 balanced blocks /12288 retained calls per side, supplying147480 more complete calls. These are three targeted uncertainty checks after TWO complete20-case comparisons, not a substitute for the larger inputs.

The final qualification inventory uses the17 non-clock-limited cases from the secondary cohort and the three refined cases. All20 original per-call-median1.05 gates pass for R217 and R218. The descriptive block-mean ratios also all pass. This combines disjoint case groups from matched comparisons; it is not a third homogeneous20-case run or a compounded speedup. Original primary19/20, all raw observations and the first wider uncertainty intervals stay archived.

| Short case | R211 block mean ns | R217 block mean ns | R217 ratio / interval | R218 block mean ns | R218 ratio / interval |
|---|---:|---:|---|---:|---|
| empty | 84.471 | 82.128 | 0.972265; [0.961657,0.986072] | 81.499 | 0.964820; [0.955667,0.979282] |
| one-stored | 103.556 | 106.274 | 1.026254; [1.011029,1.035924] | 103.025 | 0.994879; [0.982236,1.002733] |
| seven-stored-prefix | 110.724 | 110.066 | 0.994064; [0.986275,1.003476] | 108.654 | 0.981311; [0.971105,0.992972] |

Intervals are deterministic paired-block percentile bootstrap diagnostics,4096 resamples. They describe this cohort; they do not guarantee performance on every machine. All three upper endpoints are below1.05 for both variants. Do not interpret the quantized seven-byte median125→84ns as a32.8% speedup: its block means show a much smaller change. Reference medians still round to zero on these cases, so their multiples remain unresolved.

## Full revalidated inventory

Reference multiples below retain the secondary cohort’s ordinary per-call medians for nonzero reference times, matching earlier reports. Qualification ratios use the matched refined block means; the groups and statistics are explicitly different.

| Input | Normal / Roc | R211 / Roc | Retained R217 / Roc | R218 / Roc | R217 block ratio | R218 block ratio |
|---|---:|---:|---:|---:|---:|---:|
| reymont-prefix-32769 | 3.463383× | 2.757926× | 2.739571× | 2.733972× | 0.993345 | 0.991315 |
| 33027-periodic-window | 3.710663× | 2.610007× | 2.593667× | 2.614001× | 0.993739 | 1.001530 |
| repeated-32769 | 3.168207× | 2.400408× | 2.392971× | 2.408156× | 0.996902 | 1.003228 |
| random-32769 | 5.159893× | 4.519807× | 4.464336× | 4.472058× | 0.987727 | 0.989436 |
| 16384-distribution-change | 6.786630× | 4.229061× | 4.164293× | 4.168934× | 0.984685 | 0.985782 |
| eight-literals | 4.391781× | 0.807653× | 0.799986× | 0.816623× | 0.996132 | 1.012976 |
| reymont-prefix-16384 | 4.252654× | 2.791608× | 2.764281× | 2.758188× | 0.990211 | 0.988029 |
| empty | unresolved | unresolved | unresolved | unresolved | 0.972265 | 0.964820 |
| one-stored | unresolved | unresolved | unresolved | unresolved | 1.026254 | 0.994879 |
| seven-stored-prefix | unresolved | unresolved | unresolved | unresolved | 0.994064 | 0.981311 |
| eight-overlap | 4.189640× | 0.779538× | 0.771548× | 0.791738× | 0.996238 | 1.017546 |
| 320-overlap | 4.128330× | 0.883361× | 0.884918× | 0.892432× | 1.003829 | 1.009308 |
| eight-literals-prefix | 4.455482× | 0.811332× | 0.807309× | 0.817253× | 0.989436 | 1.001307 |
| 128-biased-dynamic | 3.730338× | 0.748387× | 0.747799× | 0.756090× | 1.005217 | 1.007548 |
| 32-high-entropy-stored | 4.061840× | 0.863116× | 0.853287× | 0.856734× | 0.985021 | 0.990230 |
| random-64 | 3.986510× | 0.895551× | 0.870339× | 0.871610× | 0.975631 | 0.975810 |
| random-256 | 4.177798× | 1.152969× | 1.070420× | 1.075841× | 0.929499 | 0.931871 |
| random-1024 | 3.666399× | 1.204728× | 1.112045× | 1.115585× | 0.923749 | 0.930334 |
| repeated-1024 | 3.898352× | 1.006609× | 1.007426× | 1.013595× | 1.001199 | 1.004014 |
| reymont-prefix-1024 | 4.563894× | 1.400076× | 1.358347× | 1.355941× | 0.970046 | 0.970256 |

Across this refined equal-case-weight inventory, geometric point ratios vs frozenR211 are0.985482 for R217 and0.987298 for R218. R218/R217 is1.001843. This0.184% point difference alone is modest; the independent instruction cohort also gives R217 lower counts on17 inputs, two equal counts and one higher count. R218’s smaller code does not justify discarding the earlier strategy’s periodic/repeated/eight advantages. Both source patches and all tradeoffs remain available.

Natural32 retained R217 global requests5184→608 and tracked writable copies949428→14932B, with unchanged large payload capacities and runtime. Random requests4262→276; distribution3176→522. These are compiler lifetime savings, not a source-algorithm shortcut or a broader allocator change.

## Selection, profiles and limits

Retain the existing R217 native Object/worker/source as the next experimental comparison baseline, after this precision qualification. R218 remains a preserved simplification alternative. Normal compiler IR215 and all canonical admission/accounting/recovery behavior stay unchanged. Neither alternative restores those omitted ordinary guarantees. No normal-performance speedup or parity is claimed.

Fresh exact-retained-executable wall and PC profiles are included separately from the R218 candidate profiles, with exact binary/Object hashes: retained22499 wall calls/66742 PC calls. A copied descriptive cohort label incorrectly says R216pool; raw labels are preserved and profile-label-correction.json identifies the correct no-pool R217/R218 code by exact hashes. No timing or profile data is rewritten. Their sample shares are timer weights, not instruction fractions. Hardware/heap cohorts are separate from all primary/precision timing; no cross-cohort multiplication estimates function instructions.

Current normal/source/fixture code remains unchanged. No new all-feature or678-test normal suite claim; historical seven exclusions remain. Original/decoded small-image qualification remains640 cases per compiler experiment. All measurement calls verify real bytes and independent inflate. Largest33027B; no multi-MB or ten-workload suite. Existing draft PR973 only; ordinary exact-head CI cancelled by standing direction, review automation retained.

Replay heap-summary.py then analyze.py. Source/measurement commands, full raw observations, clock check, source query, compiler patch and retained profiles are in the shared evidence packet. Native binaries/Objects, bytecode, executable descriptors and raw traces are excluded.

Next map the retained executable’s hottest native PCs to exact current Plan operations before another semantic optimization. Inspect existing source-location support and require byte-identical executable text for any debug-only map. Then choose one measured instruction/transport cost; keep the R217/R218 tradeoff visible.

## Public evidence

[Source patches, every timing cohort, clock qualification, controls and exact-executable profiles](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r218-r219-owner-timing-80d995f4da67.tar.gz) (3230292 bytes;877 files). SHA256 `80d995f4da6724048919dab20275431d07b9dbe7fa06e686a1f899770f9dbf22`. [Publication verification and exact heads](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r218-r219-publication.json).
