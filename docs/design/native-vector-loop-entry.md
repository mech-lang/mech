# Dedicated vector-loop entry — R233

Retain the two-file compiler experiment over retained R231, without R232's forced first scalar word. All 25 unchanged point/block gates pass in both predeclared complete cohorts and the 48 pooled balanced blocks. Pooled geometric improvement is **0.282%**. Periodic improves **2.311%**, repeated **1.964%**, and both retire about **3.17% fewer instructions**. Preserve natural32's **0.160% regression**, eleven adverse cycle medians, and the second cohort's **0.0319% adverse geometric mean**. This is a small overall win, with a clearer matching-loop result.

## Complete-call measurements

| Case | R231 / Roc | R233 / Roc | Pooled blocks | Instructions | Cycles |
|---|---:|---:|---:|---:|---:|---:|
| reymont-prefix-32769 | 2.308593× | 2.312284× | +0.160% | -0.035% | -0.381% |
| 33027-periodic-window | 1.730610× | 1.690612× | -2.311% | -3.164% | -2.341% |
| repeated-32769 | 1.636908× | 1.604756× | -1.964% | -3.175% | -1.687% |
| random-32769 | 2.822516× | 2.805143× | -0.616% | -0.014% | -0.094% |
| 16384-distribution-change | 2.789648× | 2.769035× | -0.739% | -0.006% | -0.701% |
| eight-literals | 0.548447× | 0.547064× | -1.535% | +0.002% | +1.145% |
| reymont-prefix-16384 | 2.268966× | 2.256373× | -0.555% | -0.028% | +0.040% |
| empty | 1.976190× | 1.976190× | +0.614% | +0.000% | +6.672% |
| one-stored | 1.976190× | 1.976190× | +1.011% | +0.000% | -1.047% |
| seven-stored-prefix | 2.000000× | 2.000000× | +0.693% | +0.000% | -4.406% |
| eight-overlap | 0.531505× | 0.527439× | -1.033% | -0.000% | -1.635% |
| 320-overlap | 0.612814× | 0.606239× | -0.056% | -0.427% | +3.432% |
| eight-literals-prefix | 0.548306× | 0.543597× | -0.395% | +0.000% | +2.919% |
| 128-biased-dynamic | 0.556109× | 0.559321× | +0.359% | +0.002% | -2.662% |
| 32-high-entropy-stored | 0.577661× | 0.591044× | +2.534% | +0.003% | +0.156% |
| random-64 | 0.607688× | 0.604310× | -1.972% | -0.006% | +1.304% |
| random-256 | 0.695392× | 0.698379× | +0.414% | +0.001% | +0.439% |
| random-1024 | 0.772309× | 0.775417× | -0.211% | -0.000% | -1.433% |
| repeated-1024 | 0.689277× | 0.687805× | -0.415% | -1.351% | -3.320% |
| reymont-prefix-1024 | 0.968212× | 0.967963× | +0.217% | -0.022% | -0.639% |
| heldout-window-32767 | 2.309244× | 2.303694× | -0.240% | -0.033% | +0.072% |
| heldout-window-32768 | 2.307354× | 2.305544× | -0.078% | -0.032% | +0.146% |
| heldout-slides-65537 | 2.385169× | 2.382565× | -0.109% | -0.051% | -0.318% |
| heldout-random-65537 | 3.137375× | 3.129586× | -0.248% | -0.022% | +0.025% |
| heldout-mixed-131073 | 2.578280× | 2.566679× | -0.450% | -0.437% | -0.330% |

Use these fresh matched reference denominators for both sides. Changes from historical ratios are not themselves compiler effects. Eight-byte instructions remain essentially unchanged; no new tiny-case algorithmic work saving is inferred. Individual cohorts, counter/heap data and all contrary values remain in the packet. No extra confirmation or altered 1.05 allowance.

324,444 complete primary calls / 324,144 retained observations; 3,900 separate CPU/heap calls and 50 full-stream smokes. Same 20 original and five held-out runtime inputs through 131,073 B, fixed level 12, held borrowed input/output aliases, equal 200 ms warmup, balanced order, pinned mimalloc and shared Rust clock/validation library. Compression, real scratch construction, output transfer and cleanup remain timed. Validation/preparation/logging/old output drop stay outside. All exact streams, independent inflate, aliases, prefix/capacity and final zero nonreturned private payload/owners agree. No builds/tests/profiles overlap primary or counter collection.

## Generic transformation and proof

Only codegen/wide_equal.rs and its unit test differ from R231's compiler source. Keep its private current-window certificate: two immutable flat byte windows, packed eight-byte loads, sole changing I64 induction phi, exact step 8 and a pure equality cycle. No program name, source ID, field, input size or compression-path selector.

Snapshot exactly two original predecessor instructions: one initial edge and the proved equal backedge. Additional predecessors or unproved reentry decline. Redirect their actual current arguments into a private vector header. The fresh capture gate proves initial at<=limit, representable limit+8 and both readable current windows. Scalar success and wide remaining>=16 preserve the ordered position. Invariant bases/limit dominate the new header; no invocation permission or stale backing pointer is cached.

Matched vectors advance 16 directly to the private header. Short remainder or mismatch enters the untouched scalar header with an explicit current-position argument. Original scalar first-difference/ctz, tail, checked fallback, errors/origins and retry remain. Original at+8 checks are redundant only in the admitted version; invalid/unknown captures keep the original checked clone. No source effect, debit, extra carried state or fallible predicate can be skipped. Ordinary entries receive no compact certificate.

All nine intended loops in the same three numeric bodies qualify. The complete Object root and 133 other bodies remain byte-identical. Two bodies shrink 10,308→10,304 B and frames 400→384 B. The third keeps 9,772 B/304 B while its bytes change. Root text 5,649,872 B/stack 55,088 B and full worker 6,623,288 B stay equal. All 25 global allocation request/byte/reallocation/incremental usable-peak fields and payload request/copy/zero/private-peak fields match. No allocation, streaming or whole-worker RSS improvement is claimed.

## Qualification and profiles

686 maintained tests pass under the same six features, -Dwarnings and seven inherited exclusions. The raw-pointer unit fixture explicitly models ordered admission; its fixed buffers cover every tested limit. All 1,057,536 previous expected values remain: 951,264 admitted and 106,272 declined-entry controls per run, executed twice. A new extra-entry refusal joins effects/stride/extra-state/endianness/predicate/missing-scope controls. The fixture now respects its certificate's prerequisites; real-source invalid/overflow/unselected/error paths still execute unchanged.

All 5,788 source controls rerun across four programs, parent/candidate and original/decoded forms. Eight Object/metadata pairs are byte-identical. Exact values/status/source origins/shapes, aliases, old publication on failure, cleanup and retry match the independent source-operation oracle and pinned rows. Eligibility equals the retained parent; fallible and stride12 controls select no vector loop. Fresh focused resident calls remain zero: unchanged R230 source/engine observations are pinned and reused explicitly. The maintained suite, including resident tests, reruns. Existing resident compute denials and allowances remain. Final managed bytes/nodes/private payload are zero.

Fresh 25,395 wall-profile and 100,005 qualified actual-PC-profile calls use the current executable/Object/symbol map. Samples are CPU-time attribution, not per-function retired instructions. Seventeen instruction medians decrease/three tie/five rise; fourteen cycle medians decrease/eleven rise. Instrumented CPU/heap elapsed columns also retain adverse values; they are independent of primary acceptance. Commands, profiles and complete cohorts remain available.

## Failure and recovery

An environmental OS errno28 disk-full error interrupted saving the final eight-byte diagnostic PC capture after both primary/counter cohorts and five PC captures completed. Preserve the initial failed capture/receipt. Cleanup verified and freed 1,536,711,696 B of completed test executables and 405,434,075 B of unused prior compiler builds/duplicate logs, retaining measured executables/Objects/source and verified external evidence. All 242 completed measurement/PC files remain hash-identical. Only the unfinished eight-byte profile repeats, at the same settings; its recovery has normal zero-payload/owner cleanup. No completed primary/counter case is remeasured and no allowance changes. A helper-generation quoting error was corrected before data mutation. There are no introduced compiler build/test/semantic failures or program resource denials.

## Pins and scope

- base: `253f322057268025403ec8f82c20943299df4f8a`
- experimental_parent: `14ae29af688a5372cc1a7fe5712f0efb77ae414b`
- normal_compiler: `42c95cfd79e0dc3773d03aeb9f553d32ff3922c5`
- compiler_source_manifest_sha256: `322bb1e4be404cee0000a4b05449a7e659b4f857682784076106f73176e67c8e`
- compiler_patch_sha256: `8d5a9710c86148b68c68523652078fb7bcd218a59a2b30fb0b78d72f0a940037`
- source_sha256: `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`
- bytecode_sha256: `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`
- object_sha256: `1c2796ec92aa28a3865cdd7679b14b21a1fccadfeb0f3e563fe30b5c60f8cd7f`
- worker_sha256: `61ae98062be0a785cfe0795e39ca866c8bbf6b873d020dfef552f4b616357cef`
- reference_worker_sha256: `ed1ab9175cd175d26d453afc26ba1fa7adb4827ec705aa1cf9bcd231ac84860c`
- allocator_sha256: `38c6ec82a41e90a96bb9f1ae2c89058a77909da3fbb69c16e542ae5c858bf874`

Normal compiler remains R215 at42c95cfd79e0dc3773d03aeb9f553d32ff3922c5, IR/work/ABI/MBC215/107/39/14. This standalone experiment does not restore its additional exact resource/admission/accounting/publication/recovery contract. No ordinary full-compressor speed claim, source/algorithm/allocation-site change, LLVM, new operation class, R225 combination or new branch/PR. Full parity remains open. Source/MBC pins, tables/depth300/nice258/near-EOF limits,262144 U16 tree,1500513 U32 cache,305257 U64 path capacity and output cap13 clear/reuse remain.

The complete image emits once from strict pinned MBC14; all small original/decoded pairs rerun. Multi-MB inputs, a second complete-image emit and the historical packet/write census are not claimed newly run. Next target reuse of already executed guarded packed reads in the natural-text path. Keep prefetch hints and broader proof work separate, each with a minimal alias/error/retry witness first. No blanket readonly/can_move flags or program-specific selection.

## Verified evidence

[Full compiler source, minimal patch, both cohorts, counters/profiles and tests](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r233-vector-entry-38c7926b9788.tar.gz). SHA-256 `38c7926b97887d5eb5e344ce7cb286e1d1c7e296fc38f47ad0169684ad02a941`; 6630694 bytes. All 1,429 public file hashes and numerical replay verify. Native binaries/Objects, MBC, full executable metadata/descriptors and raw traces/stack samples are excluded.
