# Compact native computation: diagnostic baseline

R193 completes the first phase of Experiment B. The unchanged canonical Plan, scalar and array operations, aggregate emitter, and backend generate the complete nested optimal-path computation with compact typed state. **Production remains R191. This is diagnostic headroom, not a whole-compressor improvement.**

The nine-file compiler prototype is retained as a patch and complete changed sources in the external evidence. It applies to `25bbeed77218c614ffa0fac6f196cf7037c46dcf`. The ordinary compiler independently reproduces all five R191 image files byte-for-byte. Source, bytecode, algorithm, real buffer sizes, allocation sites and backend stay fixed; no handwritten compressor or successful placeholder operation is used.

All four generated functions use the shared computation emitter. A backward word-liveness proof derives reads/writes from shared scalar and aggregate emission, control edges and actual semantic results. Every live initial value must be a compiler constant or explicit current input. Unknown inputs reject compilation. Typed input occupies 216 bytes; the result header and origin occupy 48 bytes. Each function has 1,632 bytes of code, 160 bytes of stack and no calls, versus 3,904/4,016 bytes of code and 352/368 bytes of stack in the existing body.

The diagnostic preserves real initialized typed storage, private writable ownership, source assertions and error order, direct bounds/capacity checks, and checked/wrapping arithmetic. It omits ordinary exact resource accounting, canonical admission/publication and precise recovery contracts. It is not ordinary NoLimits or a qualified entry. Insufficient append capacity returns explicit growth-needed status; a real growth implementation is still required for the full compressor.

One matched executable contains A's actual production instruction streams and B's compact functions. Eight alternating A/B and B/A blocks swap private backing allocations after every block. Primary kernel, required-adapter and hardware-counter cohorts are separate. All observations, calibration and contrary values remain archived.

| Captured input | Positions in this phase | Existing kernel | Compact kernel | Speed versus existing phase |
|---|---:|---:|---:|---:|
| Random 32 KB | 32,769 | 203.396 µs | 139.729 µs | 1.4556× |
| Distribution change 16 KB | 8,146 | 121.354 µs | 90.917 µs | 1.3348× |
| Repeated 32 KB | 32,769 | 215.458 µs | 153.229 µs | 1.4061× |
| Natural text 32 KB | 5,000 | 139.896 µs | 107.771 µs | 1.2981× |

All eight primary blocks improve for every case. Larger kernels take 22.96–31.30% less time, with 33.72–51.91% fewer raw instructions in the independent counter cohort. The eight-byte kernel is 125→84 ns, close to the clock tick; that quantized ratio is not a full tiny-compression result. Required-adapter measurements include reservation, initialization, execution, completion and drop, excluding oracle/JSON. Neither phase timing is compared with a complete reference call. The full compressor reuses node capacity across phases, so adapter savings cannot simply be multiplied by phase count.

13,116 measured phase calls match every final node, shape, length and capacity. Five focused real-buffer cases cover repeated execution, zero trips, the source's out-of-window default-zero reads, explicit growth rejection and fresh retry. Nine original and nine decoded nested-loop controls agree on changing Boolean/current arguments, assertion-before-division, recursive assertions, arithmetic errors, unselected branches, unchanged prior private publication/input aliases, and retry. These controls do not establish the omitted ordinary account/publication contract.

Ten separate profiles retain 5,174,020 verified calls. Larger-case native functions contain 93.18–96.28% of running timer PCs in these phase batches; the surrounding work includes reset, oracle and JSON. Tiny-case profiles mostly sample the harness. Timer samples are not per-function instruction totals or a complete-call partition. Raw RSS/footprint observations also do not establish isolated B peak-memory savings: A and B share an executable and prepared backing. The smaller header backing is a representation-size comparison, not a full memory or allocation reduction.

The complete ordinary compressor also passes 15 new exact-byte/independent-inflate calls with aliases, capacity and final managed memory unchanged. A forwarding wrapper counts 3,248–9,741 root runtime callbacks per call. All 5,181 static library sites reduce to 15 operation families. The closed Plan has no provider effects or string/map/parallel boundaries. The evidence classifies every family as computation, representation, construction, ownership or concrete allocation; callback counts do not measure their CPU cost.

The next bounded step is compact typed storage across the whole Plan, including fixed records, discriminated payloads, numeric views and actual constructors. Reserve/growth, initialized copies, append and clear must have real implementations. Then restore exact accounting, current ownership/publication and cold recovery in measured increments before ordinary-entry integration. The milestone remains the entire qualified native compressor; the brief is not complete.

Three introduced diagnostic setup/test defects were corrected with failed receipts preserved: an unused helper under warnings-denied, an incorrect empty-cache error oracle, and a profiler target exiting before attachment. Receipt-analysis field/path assumptions were corrected separately. No unresolved introduced compiler failure is known. GPU/harness builds use opt3, warnings denied and frozen R145 dependencies. Fresh dependency rebuilds and a new all-20 production campaign are skipped because ordinary image generation is byte-identical; R191's existing qualification and disclosed inherited exclusions remain authoritative.

[R193 in the external evidence index](../../benchmarks/mech-deflate/experiments.json) records exact source/build pins, prototype patch, disassemblies, all raw primary/counter observations, timer exports, failure dispositions and receipt-only replay. No native binaries, bytecode or raw trace stores are uploaded. Only existing draft PR #973 is used.
