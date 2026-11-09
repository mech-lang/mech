# Guarded wider immutable equality — R230

Retain the five-file compiler experiment over R229. Whole-program source/algorithm and allocation sites remain unchanged. Repeated32769 improves **13.075%**, periodic33027 **12.577%** in pooled balanced block means. The all 25 geometric improvement is **1.620%**. Natural32769 regresses **0.961%**, with **0.893% more instructions** and **1.289% more cycles**; retain this contrary result.

## Timing qualification and initial failures

Both complete 24-block cohorts are retained. The first passes22/25 point-median and23/25 block-mean1.05 gates: eight-overlap and eight-literals-prefix fail both, and128-biased-dynamic fails the point gate. It is not relabelled passing. Their separate instruction counts are essentially unchanged and cycle medians lower. Exactly one predeclared complete confirmation uses fresh workers, the identical protocol and the unchanged1.05 threshold. It passes25/25 of both gates; pooling all 48 balanced blocks also passes25/25. Retention uses the confirmation plus the complete pooled evidence, not replacement of the first cohort or a relaxed allowance.

Eight literals rise4.339% initially and fall2.940% on confirmation; pooled block means rise0.581%, while pooled point medians fall0.456%. Pooled after/reference is0.507485x. No additional tiny-case speedup or strict zero regression is claimed. Natural32's repeated adverse result is the next optimization target.

| Case | R229 / Roc | R230 / Roc | Pooled block change | First block change | Confirmation block change |
|---|---:|---:|---:|---:|---:|
| reymont-prefix-32769 | 2.281206× | 2.303124× | +0.961% | +1.021% | +0.985% |
| 33027-periodic-window | 2.043920× | 1.786849× | -12.577% | -12.608% | -12.722% |
| repeated-32769 | 1.933184× | 1.680426× | -13.075% | -13.285% | -12.408% |
| random-32769 | 2.837147× | 2.778717× | -2.059% | -2.436% | -1.883% |
| 16384-distribution-change | 2.800777× | 2.765865× | -1.246% | -1.664% | -1.052% |
| eight-literals | 0.509812× | 0.507485× | +0.581% | +4.339% | -2.940% |
| reymont-prefix-16384 | 2.252538× | 2.248348× | -0.186% | -0.289% | +0.504% |
| empty | 1.976190× | 1.976190× | -0.661% | -1.087% | +0.055% |
| one-stored | 1.976190× | 1.976190× | -2.826% | -2.890% | -2.696% |
| seven-stored-prefix | 2.000000× | 2.000000× | -1.479% | -1.399% | -1.405% |
| eight-overlap | 0.499423× | 0.498989× | +1.323% | +6.399% | -1.626% |
| 320-overlap | 0.573899× | 0.564001× | -1.568% | +2.434% | -5.303% |
| eight-literals-prefix | 0.505197× | 0.502669× | +0.133% | +6.137% | -3.003% |
| 128-biased-dynamic | 0.514942× | 0.511622× | +0.290% | +4.701% | -3.521% |
| 32-high-entropy-stored | 0.540014× | 0.534397× | -0.321% | +4.156% | -3.227% |
| random-64 | 0.574909× | 0.564301× | -1.042% | +2.333% | -2.818% |
| random-256 | 0.667176× | 0.661458× | -0.810% | +0.957% | -1.368% |
| random-1024 | 0.759720× | 0.755265× | -0.219% | +1.684% | -1.167% |
| repeated-1024 | 0.700878× | 0.673094× | -3.324% | -1.626% | -5.581% |
| reymont-prefix-1024 | 0.944095× | 0.936549× | -0.500% | +0.650% | -1.376% |
| heldout-window-32767 | 2.278083× | 2.302633× | +1.078% | +1.095% | +1.041% |
| heldout-window-32768 | 2.283450× | 2.304029× | +0.901% | +0.830% | +1.021% |
| heldout-slides-65537 | 2.361941× | 2.388346× | +1.118% | +1.023% | +1.065% |
| heldout-random-65537 | 3.172299× | 3.126753× | -1.436% | -1.823% | -1.373% |
| heldout-mixed-131073 | 2.636583× | 2.587683× | -1.855% | -1.836% | -1.875% |

324,444 complete primary calls/324,144 retained observations across both cohorts. All20 original and five held-out cases through 131,073 B retain exact raw streams, independent inflate, held input/output aliases, prefix/capacity and zero final nonreturned private payload. Fixed level 12, runtime inputs, same Rust clock/pinned mimalloc, equal 200 ms warmup and 24 balanced three-actor blocks per cohort. Full compression, scratch construction, output transfer and cleanup remain timed; validation/preparation/logging/old output drop remain outside. No compilation/tests/profiling overlaps primary/counter collection.

## General rule and implementation

The existing current-capture/source-pack proof supplies a private certificate for two immutable U8 read windows, common induction/limit, eight-byte step and confined eight-byte expressions. Certificates exist only at the compact entry. Ordinary work counters, admission and publication paths receive no certificate.

After current CFG cleanup, structural matching requires one changing I64 header phi, the original successful eight-byte range guard, two I64 loads at invariant base+induction, XOR equality with zero, and a pure equality edge advancing by eight. Other state, stores, calls, debits, strides or unsupported geometry decline. No source name/ID, input size, compression level, table name or benchmark identity selects the optimization.

On the original readable edge, limit-induction>=16 admits two complete words. A native 16-byte load from each side and XOR skips two equal iterations. An unequal wider result enters the untouched scalar body at the same position; its original mismatch calculation and error/tail order determine the answer. The whole current window is already proven readable and immutable. Spare capacity and wider alignment grant no permission. Fresh gates, current views, source assertions, original matrix fallback, aliases and cleanup remain. See LIFETIME-AND-SAFETY.md.

Nine static loops in three existing numeric routines qualify;133 other numeric/map bodies and the entire5,649,872 B root body remain byte-identical. No phase/source operation becomes newly native. Actual ARM64 contains paired q-register loads, vector XOR and pairwise nonzero reduction; current-PC profiles execute those new ranges. Static eligibility is not a measured dynamic iteration count.

The three routines add 500 B in total. Two frames grow384→400 B; the third stays 304 B. Root stack stays55,088 B and the full worker remains6,623,288 B including file padding. Cleanup metadata describes the prior CFG-cleanup stage, not a final post-widening IR count. The retained R229 call-storage/code-size improvement is preserved.

## Separate hardware and memory cohort

3,900 separate CPU/heap observations. Five instruction medians fall, three tie, seventeen rise; seventeen cycle medians fall and eight rise. Sampling is not retired-instruction attribution. All25 allocation requests/requested bytes/reallocations/incremental usable peaks and source payload requests/bytes/initialized copies/zeroing/private peaks match exactly. Whole-worker RSS includes code/metadata/runtime/validation; no streaming or total-RSS reduction claim.

| Case | R229 instructions | R230 instructions | Change | Cycle change |
|---|---:|---:|---:|---:|
| reymont-prefix-32769 | 137,120,421.5 | 138,344,795.0 | +0.893% | +1.289% |
| 33027-periodic-window | 48,206,143.0 | 45,578,473.5 | -5.451% | -12.260% |
| repeated-32769 | 47,738,709.0 | 45,128,622.0 | -5.467% | -11.662% |
| random-32769 | 49,644,811.0 | 49,645,399.5 | +0.001% | -0.820% |
| 16384-distribution-change | 35,255,893.5 | 35,261,931.5 | +0.017% | -1.069% |
| eight-literals | 1,235,614.5 | 1,235,625.0 | +0.001% | -1.277% |
| reymont-prefix-16384 | 68,247,454.5 | 68,715,598.5 | +0.686% | +0.557% |

The reference executable uses the same eight-byte scalar word comparison in the inspected matching source/binary. The new Mech widening is an ordinary compiler transformation, not a delegated library compressor or an altered match search. Separate reference-matching.asm and its hash identify the actual reference excerpt.

## Tests, profiles and failure disposition

686 maintained tests pass under the unchanged six-feature -Dwarnings profile, with the same seven inherited exclusions. Two new unit tests include74,880 executed native value checks over unaligned bases, every mismatch word, tails and current buffers, plus rejection controls for writes, changed steps, extra state, unsupported endianness/predicate and missing scope.

5,788 compiled source controls (1174 normal and91 each swapped/fallible/stride12, each across parent/candidate and original/decoded) match exact status, source origin, values, shapes, aliases, prior publication and successful retries. All16 small Objects/metadata form eight identical original/decoded pairs. Actual numeric phase entry occurs on the tested mixed fallback/native path; fallible and stride12 sources select no widened loop. Final private payload and managed bytes/nodes are zero. The earlier 3,800/2,964 packet/write census is not claimed newly rerun; the maintained suite and new controls execute, and133 unchanged specialized bodies plus exact root bytes establish the unaffected code boundary.

The frozen parent reproduces resident default rejection required 16,802,477 > 16,777,216. Full-visit rejection counts are 928 normal/57 swapped/57 fallible/62 stride12 per form. Single-visit rechecks remain capped on 772/51/51/60. No allowance changes. Independent source-operation results and parent/candidate original/decoded native entries cover every case; admitted resident results agree as well. These policy rejections are not asserted to be source arithmetic/assertion outcomes. The same seven maintained exclusions remain explicit in the exact command.

Initial development compiler API misspellings and a test-client unused-method error were fixed under warnings-as-errors before retention; receipts remain. Initial scalar/frame probes met no existing complete-phase selection and did not broaden it. A one-second qualification diagnostic located the slow part in resident planning/execution, not a long-running compressor. Raw samples stay local. No new semantic compiler failure remains.

Fresh 25,185 wall-profile and 103,766 actual-PC-profile correct calls use each executable's current symbol map. Large full-worker profiles remain mostly generated code. The matching routine's current-PC sample share falls about43→35% on repeated and43→36% on periodic; these are separate sample cohorts, not absolute time or instruction savings. Natural's unchanged path-scan routine remains about48% of current-PC samples. The actual wider branch still materializes a Boolean before branching; that and first-word peeling are independently measurable next targets. Full parity remains open.

## Pins, commands and scope

- base: `f6638441b1756ce732216547f3ee4833a29c6e73`
- experimental_parent: `f6638441b1756ce732216547f3ee4833a29c6e73`
- normal_compiler: `42c95cfd79e0dc3773d03aeb9f553d32ff3922c5`
- compiler_source_manifest_sha256: `f657b5e0457f980210c793630d214953cfe61750afc7ab594df8be25e9ea8a8e`
- compiler_patch_sha256: `bb75b0639a51e54b108cc674b97e38477d0d71c885aba091fe496828c3a17845`
- source_sha256: `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`
- bytecode_sha256: `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`
- object_sha256: `83cf6a89901dd8796ee2426f6437eef34cd9e666c8f1f3034d3dba7f4bc90ee9`
- worker_sha256: `2596c731d8deb59a3aaa14775871fac63067bfd6a3379f700a67b86ae3ffb5f0`
- reference_worker_sha256: `ed1ab9175cd175d26d453afc26ba1fa7adb4827ec705aa1cf9bcd231ac84860c`
- allocator_sha256: `38c6ec82a41e90a96bb9f1ae2c89058a77909da3fbb69c16e542ae5c858bf874`

Exact build specifications, command receipts, all raw structured cohorts and analysis scripts accompany this report. Reproduction sequence: run-spec.py candidate-gpu candidate-producer candidate-tests; first-controls.py; full producer against pinned MBC14; run-spec.py mech-worker after-cpu after-memory; smoke-all.py; measure-ready.py; finish-qualification.py; confirm-and-profile.py; analyze.py. Replaying analyze.py needs only the retained structured data and does not run compression.

Maintained normal source remains R215 at 42c95cfd79e0dc3773d03aeb9f553d32ff3922c5, IR/work/ABI/MBC215/107/39/14. This standalone experiment still omits its extra exact resource/admission/accounting/publication/recovery contract. No ordinary full-compressor performance or restored-contract claim. No LLVM, new PR/branch, multi-MB run or combined R225 source variant.

The original source/bytecode pins, all tables/search limits/depth 300/nice 258/near-EOF rules,262,144-U16 tree,1,500,513-U32 cache,305,257-U64 path capacity, output cap 13 with clear/reuse and timer endpoints remain. No source algorithm or allocation site changes.

## Verified public packet

[Full compiler source, minimal patch, both complete timing cohorts, hardware/memory observations and qualification](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r230-wide-equality-11f92e64c902.tar.gz). SHA-256 `11f92e64c9029c8fd2abc87d70bf6b5f5910ef0dbb115f981a640fb5160c6304`; 7116285 bytes,1468 file hashes and the downloaded numerical replay verified. Native executables/Objects, MBC, full descriptors/metadata, raw samples/traces and credentials/private environment exports are excluded.
