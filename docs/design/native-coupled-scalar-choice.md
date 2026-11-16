# Coupled scalar minimum choices — R187

Reject the current coupled scalar-choice variant: the local steady loop improves16 to13 instructions and natural text improves about0.7%, but random32 is2.070262% slower in all8 blocks,12 instruction medians rise and11 cycle medians rise. Overall elapsed is nearly flat(-0.055883%). Restore the qualified R186 compiler, preserve the complete candidate and use its concrete widening instruction for the next refinement. No sample is discarded or repeated.

The tested candidate is not retained. The production compiler is restored byte-for-byte to R186/IR206. This report records candidate IR207, its full source patch and every observed result; no benchmark sample is discarded, repeated or hidden. The original qualified R186 compiler remains the starting point for a refined implementation.

## Operation and proof

The real level12 optimal-path length loop computes a U32 minimum and carries its associated winning index. The candidate uses one private U64 key: cost in high32, index+1 in low32. Low0 denotes the incoming unchanged item. Under the existing nonempty current-window entry and stop<=U32MAX proof, each reached index+1 is positive and fits low32. Unsigned minimum preserves strict incoming ties and earliest winning index. The vector-prefix missing sentinel+1 wraps to0; a real vector winner precedes the scalar remainder. Decode occurs only at the scalar exit. Source algorithm/representations/tables/allocation sites, derived recipe/refusals, funding, error order, terminal construction and publication/recovery remain.

The actual selected steady scalar path falls **16 →13 instructions** with another candidate remaining. It removes duplicate cost comparison/state transport, with extra entry/exit packing. Fresh disassembly of the pinned reference has **11** instructions; Clang and pinned Rust use one comparison shared by two selections in equivalent diagnostic scalar source. Peer helpers are never linked into Mech. Static paths are not dynamic visit counts or a whole-call event partition. The candidate still emits mov w21,w21 after an add w21; a low-word cost-width refinement is a concrete next hypothesis, not a claimed gain.

Code **8,153,276 →8,152,348 B** (−928), reported entry stack **16,480 →16,528 B** (+48). Four numeric reports change only maximum_source_instructions167→183; all other numeric recipe/proof fields remain equal. Existing source-envelope checks and allowances remain unchanged and pass. Source/MBC hashes, logical work and work/ABI/bytecode versions remain fixed. Candidate IR/work/ABI/MBC **207/107/39/14**; restored production **206/107/39/14**.

## Complete-call timings

All **20 original1.05 gates pass**: **11 faster /9 slower medians**, geometric elapsed **-0.055883%**. Passing a maximum-regression gate does not require retaining an adverse overall tradeoff. The unchanged eight-block cohort completes **3,968 primary /4,048 total calls**, maximum33,027B. Random32 worsens in **all8 paired blocks**. Distribution-change pooled median improves although only4/8 block medians improve. All contrary observations are preserved. Three zero reference medians stay unresolved.

| Input | Before / Roc | Candidate / Roc | Elapsed change | Improving/adverse blocks |
|---|---:|---:|---:|---:|
| reymont-prefix-32769 | 3.624071× | 3.598057× | -0.717818% | 7/1 |
| reymont-prefix-16384 | 4.346250× | 4.314056× | -0.740736% | 6/2 |
| random-32769 | 5.267394× | 5.376443× | +2.070262% | 0/8 |
| 16384-distribution-change | 6.638034× | 6.411380× | -3.414473% | 4/4 |
| repeated-32769 | 2.998008× | 3.004170× | +0.205526% | 4/4 |
| 33027-periodic-window | 3.514017× | 3.526781× | +0.363226% | 5/3 |
| eight-literals | 4.581944× | 4.577412× | -0.098901% | 4/4 |
| empty | unresolved | unresolved | -0.017575% | 5/3 |
| one-stored | unresolved | unresolved | -0.311418% | 5/3 |
| seven-stored-prefix | unresolved | unresolved | -0.068927% | 6/2 |
| eight-overlap | 4.339204× | 4.359751× | +0.473506% | 3/5 |
| 320-overlap | 4.309181× | 4.315537× | +0.147497% | 3/5 |
| eight-literals-prefix | 4.564475× | 4.577996× | +0.296224% | 5/3 |
| 128-biased-dynamic | 3.838908× | 3.892386× | +1.393029% | 3/5 |
| 32-high-entropy-stored | 4.133363× | 4.109142× | -0.585999% | 5/3 |
| random-64 | 4.185711× | 4.155715× | -0.716642% | 5/3 |
| random-256 | 4.200027× | 4.212591× | +0.299153% | 4/4 |
| random-1024 | 3.829208× | 3.854843× | +0.669453% | 3/5 |
| repeated-1024 | 4.039207× | 4.028871× | -0.255888% | 4/4 |
| reymont-prefix-1024 | 4.633408× | 4.633347× | -0.001323% | 5/3 |

Each pair shares a fresh reference; independent rounds are not compounded. Exact pinned raw bytes, independent inflation, logical work, input aliases, output prefix/capacity/reuse and zero final managed bytes/nodes agree. Logical work is not hardware instructions. The table describes the rejected candidate; production stays at R186.

## Instructions, cycles and memory

Separate counters show **8 lower /12 higher instruction medians**, with **11 cycle medians higher**. Natural32 instructions fall0.427068% despite a higher cycle median. Random32 instructions rise0.402718% and cycles2.486356%. These independent cohorts remain separate from elapsed and CPU samples.

| Input | Instructions before | Instructions candidate | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| reymont-prefix-32769 | 215,877,198.5 | 214,955,255.5 | -0.427068% | +0.378720% |
| reymont-prefix-16384 | 122,794,462.0 | 122,244,732.5 | -0.447683% | -0.808086% |
| random-32769 | 71,375,018.0 | 71,662,458.0 | +0.402718% | +2.486356% |
| 16384-distribution-change | 65,604,979.5 | 65,675,813.0 | +0.107970% | -0.508343% |
| repeated-32769 | 71,098,629.0 | 71,130,157.5 | +0.044345% | +1.447677% |
| 33027-periodic-window | 76,778,932.5 | 76,811,045.0 | +0.041825% | +1.355545% |
| eight-literals | 12,451,458.0 | 12,452,701.5 | +0.009987% | +3.665316% |

All20 allocation-request, requested-byte, reallocation and incremental peak usable-heap medians are exactly equal. Before/after/peak usable-live readings have a fixed **+16B** offset; its cause is unproved. No heap/RSS, streaming or initialized-copy-volume gain is claimed.

All12 original/decoded scalar controls pass unchanged latency/allocation/no-slope gates. Six adverse medians remain; ratio range **0.994931438–1.004158837**.

## Candidate executable profiles

Five fresh exact-candidate profiles complete **17,430 correct calls**. Every recorder exit and per-case/final managed cleanup is zero. Independent source-location compilation repeats the whole candidate Object byte-for-byte. New labels separately retain scalar seed/remainder/exit work; no earlier map is applied to changed code. Unknown native attribution remains explicit. Outside main includes useful runtime algorithms, other generated helpers and harness validation, not solely removable overhead. CPU-time shares are not retired-instruction counts.

| Input | Running timer samples | Inside main | Native without fine tag |
|---|---:|---:|---:|
| eight-literals | 5497 | 9.514281% | 4.038566% |
| random-32769 | 5263 | 71.917158% | 32.699981% |
| 16384-distribution-change | 5361 | 51.986570% | 23.745570% |
| repeated-32769 | 5465 | 71.729186% | 26.331199% |
| reymont-prefix-32769 | 5392 | 80.137240% | 32.140208% |

## Validation and provenance

The new test compiles actual production packing/choice/unpacking helpers and compares12,288 initialized-slice calls against an independent ordinary if-cost-less oracle: full U32 costs, missing and real incoming winners, zero/nonzero/near-U32MAX indices, ties, empty/short/repeated visits and unchanged inputs. Its owned JIT entry is restricted to an exact five-word C signature and one-word result by an explicit test-only audit entry; production gains no raw-memory boundary. Existing source controls exercise original/decoded/current/restored windows, both storage widths, vector/scalar boundaries, U32 wrapping/high halves, aliases, finite denial, unchanged publication and recovery.

**10 focused /664 candidate native tests pass; all247 CI-selected names actually execute**, with the same seven inherited exclusions. Unchanged **236 component /8 composition /1 real-capacity-path** tests pass. Fifteen architecture/catalog checks,132 workflow tests, runtime-only, formatting and warnings-as-errors pass. No expected values or allowances change. No introduced functional/build failure occurs; the performance regressions above cause rejection. Restored production is identical to R186, so its already-completed663/246 qualification is reused rather than relabeled as a fresh664-test result.

A provenance audit found R186's descriptive image-pins JSON retained R185's descriptor hash and IR205 label. Both actual executables were correct: the R186 binary contains its exact60,231,949-byte IR206 descriptor(b22ced452644c3e485b9616cbf2cfe3e47396f3e110cecbfd802f71bd0e0d808) once and its identity symbol matches; the candidate likewise contains its exact IR207 descriptor. The original R186 receipt is preserved. R187 baseline labels are corrected, with byte-for-byte executable embedding receipts. No measured binary changes, sample discard or rerun result from this bookkeeping correction.

Public parent b6a42fde80dbc011dfefa9de5d9659be334411b0; measured rejected compiler 473b458f57693cde571683288e6a1743b8b800e7; unchanged production source 582c8b00cf86df926f21845573222c3cf9ba117f. Candidate Object 9d0463adfe6b1da0e448c2ecbedf662f94d4c6987b4d634128b4ce08e49b7225; worker 85b6d86b38e326f134e65f47d4775330b90ad9eb2e90c1e5e1552f16324fe8c5. The complete five-file candidate source/patch and raw source commit are preserved, so the rejected experiment is reconstructible independently of the final evidence-only commit.

Frozen source/MBC/algorithm/tables/depth300/nice258/EOF/ties/source allocation sites, real262144-entry U16 tree /1500513-word U32 cache /305257-entry U64 path capacity, output13clear/reuse, input aliases, mimalloc2.1.7 and full evaluate_into timing remain. FrozenR145 dependencies; no math-library rebuild. Existing draft#973 only; no new PR. Ordinary exact-head hosted CI is cancelled by instruction; review automation preserved and review requested without waiting. Fresh dependency/all-feature builds, another standalone AOT harness, multi-megabyte inputs and the separate ten-workload suite are skipped. Parity and broader lowering remain incomplete.

Run checkpoint.py against chronologically unpacked evidence to reconstruct REPORT.md/results.json without compiling or compressing. Public numerical replay verifies these plus packet-replay.json.
