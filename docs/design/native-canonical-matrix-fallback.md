# Canonical matrix indexing in the experimental fallback (R223)

Retain this correctness repair in the experimental compiler. The inherited root fallback read and indexed replacement treated a logical matrix ordinal as a physical offset. They now use the existing numeric emitter's canonical column-major-to-row-major translation. Normal compiler source remains unchanged. This is not a normal-entry speedup or qualification of the complete diagnostic.

## Exact checkpoint and scope

Base `606305101e6ac88c67945525f0820b6740e22617`; normal compiler `42c95cfd79e0dc3773d03aeb9f553d32ff3922c5`; PR base `92f75db84e2523693648084bea16cd52ad60777a`. Normal IR/work/ABI/MBC215/107/39/14. Same nightly2026-03-03, Cranelift0.131.3, sparse diagnostic frontend and mimalloc2.1.7. Source SHA256 `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`; MBC SHA256 `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`. Candidate Object SHA256 `4ed01e640ecc30de6ec4e55efe84ff17a184e8e45d52621067a8c418d96d39f6`; executable SHA256 `d0cd661816865dbd07e04d89764e7ca8ceaf38dc61286360712965b2b933b288`. All changes remain on existing draft#973's branch; no new PR or branch.

Three archived compiler files change from the frozen R222 source. Factor the existing `numeric_words::linear_address` ordinal calculation into a shared `linear_ordinal` function. The numeric entry still calls exactly the same calculation. Root packed reads supply their current view rows/columns and actual admitted source schema; indexed replacement supplies the same facts before reading/comparing/updating its element. Packed record strides remain supported.

Original one-based bounds checks precede the translation. Static vector schemas retain the identity calculation. Generic current row/column vectors take the existing unit-dimension fast path. A genuine matrix maps logical index `i` to `(i % rows) * columns + i / rows`. Zero rows cannot reach division after a successful original bounds check. All original old-value comparison, unchanged-result, current uniqueness, consume/copy, alias, capacity, old-destination retirement and error ordering remain.

All134 numeric routines and2 word-map routines have byte-identical machine code AND metadata to R222. The root changes 6,762,272→6,781,712 bytes (+19,440); root stack 86,784→86,768 bytes (-16). No additional computation class is selected. This repair changes the root adapter's semantics, not the compressor algorithm.

The full level12 source, tables, depth300, nice258, original near-EOF limits and allocation sites are pinned. Real262144-U16 tree,1500513-U32 cache and305257-U64 paths stay. The eight-byte output retains capacity13, clear/reuse and saved aliases. Allocation, initialized helper copies, complete compression, output payload transfer and private cleanup remain inside the same evaluate clock. Preparation/clear/prefix/import/rebind/output-slot setup/verification/inflate/drop remain outside. Physical final-output backing reuse is unestablished.

## Reproducer and correctness evidence

Frozen R222 original/decoded indexed-write executables both fail the independent oracle: logical element2 of a2×3 backing `[1,2,3,4,5,6]` becomes `[1,99,3,4,5,6]` rather than canonical `[1,2,3,99,5,6]`. Resident execution agrees with the oracle. Both negative receipts retain exit101. The repaired original/decoded executables pass the exact same witness. R222's separately published read finding is retained as historical evidence; it is not relabeled as a fresh R223 parent run.

- 320 freshly executed read/loop controls cover2×3,3×2,row,column,empty/changing dimensions, private and shared fallback, nonzero starting index, current values, ordered assertions/bounds/checked overflow, unselected paths, saved publication/aliases, cleanup and retries.274 selected and168 fallback observer visits actually occur.
- 160 freshly executed generic indexed-update controls cover selected first/second writes, changed/unchanged values, private or borrowed continuation, a live intermediate alias, changing2D/vector/empty shapes, selectors0/max/len+1, assertion precedence, disabled updates, prior publication, cleanup and retry. Independent literal/canonical resident results agree.
- 1280 existing ownership controls execute freshly on repaired original/decoded images. Their rows/status/origins/capacity/cleanup compare with frozen R222 receipts. The baseline controls are historical rather than freshly rerun.728 resident values,320 resident errors and232 inherited default-work denials retain the same classification and allowances.5380 nonnull result handoffs and1664 omitted-copy-owner events actually occur.
- 48 unchanged real-capacity buffer-family checks execute on fresh parent/candidate original/decoded artifacts, with hardcoded values and allowances unchanged.
- 40 complete strict-decoded-MBC14 stream smokes pass exact reference bytes, independent inflate, held aliases and final zero private payload bytes/owner records. Small original/decoded Object/metadata pairs are counted separately in `small-image-pairs.json`.

Total passing compiled small controls: 1808; the two repaired write probes and two expected parent negatives are separate. Matrix controls contain400 resident values/80 source errors/no resident denials. Original/decoded semantics stay equal. This does not establish the diagnostic's omitted ordinary accounting/publication/recovery contract.

## All20 matched complete-call timings

Fresh normalR215, frozen R222 and repaired R223 run sequentially in24 balanced permutation blocks. Each row uses one fresh reference denominator. The same512 calls per clock-limited stored block,64 for other≤1024-byte inputs and1 for larger inputs are retained. Batching stays outside the unchanged per-call timer. Every call verifies exact reference bytes/inflate and aliases; no threshold or expected-value increase. 161,832 complete Mech calls /161,712 retained observations and5120 fresh reference compressions are retained.

| Input | Normal R215 / Roc | R222 / Roc | R223 / Roc | Block-mean change vs R222 |
|---|---:|---:|---:|---:|
| reymont-prefix-32769 | 3.438542× | 2.473754× | 2.485459× | +0.473% |
| 33027-periodic-window | 3.659756× | 2.196971× | 2.201556× | +0.209% |
| repeated-32769 | 3.120131× | 2.035081× | 2.045225× | +0.498% |
| random-32769 | 5.091235× | 3.089453× | 3.127000× | +1.215% |
| 16384-distribution-change | 6.636815× | 3.225765× | 3.258703× | +1.021% |
| eight-literals | 4.322328× | 0.510436× | 0.508791× | +0.796% |
| reymont-prefix-16384 | 4.190971× | 2.456441× | 2.465941× | +0.387% |
| empty | unresolved | unresolved | unresolved | +1.282% |
| one-stored | unresolved | unresolved | unresolved | -2.498% |
| seven-stored-prefix | unresolved | unresolved | unresolved | -1.723% |
| eight-overlap | 4.183433× | 0.498572× | 0.501428× | -0.182% |
| 320-overlap | 4.068392× | 0.589084× | 0.595118× | +0.477% |
| eight-literals-prefix | 4.374710× | 0.508389× | 0.518809× | +1.427% |
| 128-biased-dynamic | 3.737088× | 0.520835× | 0.538220× | +4.378% |
| 32-high-entropy-stored | 4.029489× | 0.551204× | 0.566169× | +2.801% |
| random-64 | 4.098319× | 0.590058× | 0.599561× | +1.182% |
| random-256 | 4.064738× | 0.686661× | 0.695329× | +1.970% |
| random-1024 | 3.780974× | 0.801738× | 0.826987× | +2.712% |
| repeated-1024 | 3.939801× | 0.718558× | 0.734150× | +2.318% |
| reymont-prefix-1024 | 4.482961× | 0.987561× | 1.004851× | +1.547% |

Original1.05 median gates: 20/20. Supplementary block-mean1.05 gates: 20/20. Geometric block-mean change +1.004%; ordinary per-call median geometric change +1.151%. 1 medians fall/16 rise/3 equal. Retain contrary pairs. This is a required correctness fix, not an asserted performance optimization. The three zero-rounded stored-only reference denominators remain unresolved. Cross-round reference multiples are not compounded.

## Independent instructions, cycles and memory

Separate3120 complete Mech CPU/heap calls and1280 fresh reference counter windows/10240 verified reference compressions are retained. Instructions fall on7 inputs/rise on10; 3 instruction medians are equal; cycles fall on16/rise on4/0 equal. Counter overhead is unadjusted; CPU instrumentation zeros are not heap measurements.

| Input | Instructions R222→R223 | Change | R223 instructions / Roc | Cycle change |
|---|---:|---:|---:|---:|
| reymont-prefix-32769 | 164,111,509.5→165,004,009.5 | +0.544% | 3.467876× | +1.324% |
| 33027-periodic-window | 54,807,669.5→54,646,856.0 | -0.293% | 2.836603× | -0.074% |
| repeated-32769 | 54,251,121.0→54,089,947.0 | -0.297% | 2.828720× | -0.093% |
| random-32769 | 62,784,103.0→64,129,026.5 | +2.142% | 5.456498× | +0.613% |
| 16384-distribution-change | 43,785,589.5→44,460,140.0 | +1.541% | 4.745754× | +0.245% |
| eight-literals | 1,410,419.0→1,410,120.0 | -0.021% | 1.033530× | -7.898% |
| reymont-prefix-16384 | 82,082,916.0→82,495,849.5 | +0.503% | 3.323880× | -0.166% |

All tracked global heap/request/reallocation/incremental usable-peak and payload/copy/zero/peak fields match the parent on20/20 cases. Requested capacity is not initialized bytes copied or a live-memory measure. Writable-copy counters omit generated stores and direct memmove; they are not all memory traffic. RSS and static stack/code remain distinct.

## Current executable profiles

Six fresh wall-stack profiles execute23,978 correct complete calls and six fresh current-PC profiles execute99,117. Concurrent request delivery drains the faster eight-byte case safely; the inherited oversized-pipe deadlock repair is used from the first capture. Every PC capture resolves its own current executable's ASLR slide and current Object symbols, with no stale root source map. Timer weights are not retired-instruction counts or removable-overhead shares.

| Input | Generated exclusive | Host under generated | Other worker/system |
|---|---:|---:|---:|
| 16384-distribution-change | 93.761% | 2.312% | 3.927% |
| 33027-periodic-window | 93.777% | 3.856% | 2.366% |
| eight-literals | 42.043% | 37.217% | 20.739% |
| random-32769 | 97.255% | 1.220% | 1.525% |
| repeated-32769 | 93.572% | 4.504% | 1.924% |
| reymont-prefix-32769 | 96.942% | 1.573% | 1.485% |

## Disposition and next bounded work

The inherited generic matrix fallback read/write bug is fixed in the experimental source, with clean-parent negatives and current original/decoded positive controls. The new source expands neither operation selection nor the benchmark algorithm. Exploratory read-only filename/path errors are preserved in `failure-disposition.json`; they did not replace a measurement cohort. Any inherited resident default-cap denial stays explicit. Warnings-as-errors remain enabled on the rebuilt GPU/producer/consumer executables.

The maintained normal compiler and its finite budgets, exact errors, canonical ownership/publication and recovery remain unchanged. No fresh678-test normal run, fresh52 ordinary linked controls, fresh normal scalar latency run or all-feature dependency rebuild is claimed. The same seven inherited native exclusions remain. No fresh complete phase-observer run is performed: all134 numeric and2 map machine bodies/metadata are directly verified equal. Multi-MB timing, the separate ten-workload suite and physical final-output reuse are skipped/unestablished. Ordinary hosted CI remains cancelled by instruction;#973 stays draft pending green CI and clean review. The parity goal is still active.

Next: inspect how the ordinary pipeline selects and transports the now-measured hot non-appending array/private-frame loop class. Qualify one existing class through the current canonical admission/ownership/funding/error/retry boundary, with a minimal real-operation witness and all20 regression acceptance. Do not replace ordinary authority with raw diagnostic heap permission or count already-native source operations as newly lowered. Use the current profiles to choose one demonstrated cost; keep rejected experiments and contrary cohorts.

## Reproduction

`qualify-matrix-loops.py`, `qualify-matrix-updates.py`, `qualify-buffer-handoffs.py`, `qualify-buffer-families.py`, `compare-specializations.py`, `qualify-complete.py`, `primary.py`, `cohorts.py`, `measure-reference-counters.py` and `profile-current.py` retain exact commands and receipts. `heap-summary.py`, `analyze-matrix-controls.py`, `analyze-samples.py` and `analyze.py` replay numerical evidence without native execution. Public evidence excludes native binaries/Objects,bytecode,full executable metadata/descriptors,raw traces/raw stack samples,credentials/private environment exports. The compiler patch and only its three changed source files remain in measured-source.

## Updated next assignment

The newly supplied review brief supersedes the earlier proposed ordinary-integration next step: first establish fixed-level12/runtime-input reference parity, balanced order/equivalent warmup/validation and explicit ownership mode. Then measure the two duplicate packed-cache bindings as a source control, followed by phase range/shape proof reuse. Extra ordinary resource-contract restoration is not a prerequisite for this comparison. Private-write simplification, shared phase-packet storage and wider match extension remain separate measured experiments.

[Verified public evidence and exact three-file compiler patch](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r223-canonical-matrix-6c63d9f299a0.tar.gz) (2130079 bytes;1012 files). SHA256 `6c63d9f299a0ab198a5bfda083084803a3932bc2bdc35ee3995e0f293a9c9ee6`. Server digest, download, every file and four numerical analyses replay exactly.
