# R199: ordered retirement visible at native boundaries

**Retain the generic runtime inlining hint.** Seventeen of twenty complete-compression elapsed medians improve, all twenty instruction medians fall, and all twenty original 1.05 gates pass. Three small elapsed medians and one cycle median remain adverse. Parity is incomplete; the existing faster compressor source stays retained.

Base: `4656d37d2bc4bdb2e1880a302af4a5cc29a8ae0a`, clean review in [comment 6714795374](https://github.com/mech-lang/mech/pull/973#issuecomment-6714795374). Native IR/work/ABI/bytecode remain **214/107/39/14**. The sole production change adds `#[inline]` and a comment to the existing `retirement::run`; its body is unchanged. Existing draft #973 only.

## Complete compression timings

Eight balanced blocks place each side first four times. The 3,968 primary observations and 4,048 total complete calls preserve exact reference bytes, independent inflation, logical work, aliases, capacity and zero final managed bytes/nodes. Both sides use the same fresh reference denominator per input. Geometric elapsed falls **0.531319%**; these independent rounds are not compounded.

| Input | Before / Roc | Current / Roc | Elapsed change |
|---|---:|---:|---:|
| reymont-prefix-32769 | 3.334151× | 3.317883× | -0.487917% |
| random-32769 | 4.745694× | 4.717103× | -0.602471% |
| 16384-distribution-change | 6.286899× | 6.179520× | -1.707995% |
| repeated-32769 | 2.923528× | 2.891057× | -1.110700% |
| 33027-periodic-window | 3.394935× | 3.368267× | -0.785546% |
| eight-literals | 4.512777× | 4.535279× | +0.498623% |
| reymont-prefix-16384 | 4.029960× | 4.019387× | -0.262358% |
| empty | unresolved (zero reference median) | unresolved (zero reference median) | -0.107296% |
| one-stored | unresolved (zero reference median) | unresolved (zero reference median) | -0.104636% |
| seven-stored-prefix | unresolved (zero reference median) | unresolved (zero reference median) | -0.138565% |
| eight-overlap | 4.356179× | 4.372341× | +0.371015% |
| 320-overlap | 4.237883× | 4.229236× | -0.204023% |
| eight-literals-prefix | 4.461381× | 4.394063× | -1.508906% |
| 128-biased-dynamic | 3.782392× | 3.752995× | -0.777213% |
| 32-high-entropy-stored | 4.141115× | 4.167103× | +0.627556% |
| random-64 | 4.106009× | 4.080995× | -0.609195% |
| random-256 | 4.224816× | 4.180976× | -1.037680% |
| random-1024 | 3.900768× | 3.845806× | -1.409009% |
| repeated-1024 | 4.130388× | 4.113123× | -0.417995% |
| reymont-prefix-1024 | 4.718301× | 4.679959× | -0.812624% |

Eight literals rise 0.498623%, eight overlap 0.371015% and 32-byte high-entropy stored 0.627556%. These measurements are retained, not dismissed as noise. The three zero reference medians have unresolved reference multiples. Raw blocks and denominators remain in the evidence.

## Actual mechanism and scope

The R198 executable contains a call from `Invocation::retire_tail_locals` into `retirement::run`. The latter contains the shared ordered cleanup loop, current recipes and paid cleanup state. It has one production caller. The inlining hint exposes those existing values to that caller; it adds no permission cache, unchecked execution, refund, compressor-specific rule or new algorithm.

Actual linked ARM64 disassembly confirms the call disappears. Before: caller 230 static instructions, separate run 447, separate retire_prepaid 105. After: caller 541, with the two separate symbols absent. Selected body extents shrink 3,128→2,164 bytes. Static sizes are not dynamic event attribution and do not explain every whole-call instruction difference. The full measurement harness text shrinks 57,980,740→57,975,804 bytes (−4,936); file size shrinks 27,616 bytes. This harness includes compiler/JIT preparation outside timing and is not a minimal standalone runtime distribution.

The complete native image is independently rebuilt. All five files—Object, descriptor, metadata, source and bytecode—are byte-identical to R198. Generated text stays 8,152,436 bytes and reported entry stack 16,480 bytes. Current-view validation, preview, finite charges and error order, fallbacks, generations, managed drops, aliases, publication and recovery are unchanged. No expected values or allowances increase. Fast scalar emission remains maintained; no new scalar-latency cohort is claimed.

Source SHA-256: `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`; bytecode: `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`; Object: `dbeb6ab788358909bb787b97db8005b724aac899062aa8a2bd5896cdd8edfe8e`. The pinned level 12 algorithm, tables, depth 300/nice 258, original search/EOF limits, 262,144 U16 tree, 1,500,513 U32 cache, 305,257 U64 path capacity and source allocation sites remain. Pinned mimalloc 2.1.7 and complete `evaluate_into` timing remain. Seed, clear, prefix, rebind, image restoration and validation/drop stay outside timing as before.

## Independent counters and memory

Separate CPU and heap cohorts retain sixteen interior observations per side per input. All twenty instruction medians fall. Natural 32 KB has the only higher cycle median (+0.111036%); it remains explicit. These are separate cohorts from primary elapsed.

| Input | Instructions before | Current | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| reymont-prefix-32769 | 208,704,104.5 | 208,195,864.5 | -0.243522% | +0.111036% |
| random-32769 | 71,217,857.5 | 71,012,951.0 | -0.287718% | -0.029275% |
| 16384-distribution-change | 65,118,298.5 | 64,687,715.5 | -0.661232% | -0.304152% |
| repeated-32769 | 67,001,642.5 | 66,842,773.5 | -0.237112% | -1.914599% |
| 33027-periodic-window | 72,731,965.0 | 72,491,662.5 | -0.330395% | -1.264453% |
| eight-literals | 12,608,808.5 | 12,432,447.5 | -1.398713% | -2.559857% |

All twenty allocation-request, requested-byte, reallocation and incremental usable-peak medians are equal. Eight literals retain 3,354 requests / 17,643,969 requested bytes / 15,279,752 incremental usable peak bytes. Repeated 32 KB retains 3,135 requests and periodic 33 KB 3,679. No allocation, copy-volume or live-memory reduction is claimed. RSS and physical-footprint observations include image/setup state and remain separate from incremental usable heap.

The benchmark retains capacity, prepares the next invocation from the prior output using clear, and preserves the caller's still-live output-buffer argument. Its pointer probe reports different input/result backing in every compression call on both sides. Capacity 13 on eight bytes is established; physical zero-allocation output reuse is not established or claimed. This inherited ownership cost is unchanged by this round. Requested capacity must not be called initialized-copy volume.

## Fresh executable profile and next investigation

The exact new executable completes **6,274** correct calls in four fresh wall-stack profiles, all ending at zero managed bytes/nodes. The baseline profile is explicitly reused from the exact R198 executable (6,472 calls); its worker hash matches the fresh R199 primary before-worker hash. Primary and CPU/heap before/after cohorts are both fresh. Emitted word-collection helpers are counted as generated code.

| Input | Generated exclusive | Host below entry | Other host inside evaluate | Outside evaluate |
|---|---:|---:|---:|---:|
| 33027-periodic-window | 64.58% | 30.41% | 2.41% | 2.60% |
| eight-literals | 10.35% | 77.12% | 7.38% | 5.15% |
| repeated-32769 | 69.87% | 25.37% | 2.85% | 1.91% |
| reymont-prefix-32769 | 77.94% | 19.62% | 0.98% | 1.46% |

These exclusive child-subtracted wall-stack weights are neither instruction-event samples nor callback counts. Host frames include useful buffer/algorithm work. Their percentages are not removable-overhead estimates and are never multiplied into retired-instruction totals. Large cases still spend most sampled time in generated code; eight-byte setup/admission callbacks remain prominent. The next bounded investigation should inspect repeated short ordered reductions in actual generated code and require a matched whole-program improvement. No new semantic compiler change is included here.

## Qualification and remaining coverage

Fresh optimized qualification passes **675 native tests**, including eighteen cleanup/finite-charge/alias/retry controls, and all **258 CI-selected names actually execute**. The same seven inherited exclusions remain named in native-command.json. The generic original/decoded state harness also passes assertion-before-arithmetic, unselected failure, finite denial, prior publication, cleanup and recovery controls. No test is added solely to check an inline attribute; machine code and the unchanged semantic controls establish this change.

The unchanged **236 component / eight composition / one real-capacity** qualifications pass without allowance changes. The 1,724 independent-oracle native match controls are **reused from R198**, not freshly executed; the R199 harness builds, and the complete native image is byte-identical. Fresh all-input linked compression still validates every measured call. Fifteen architecture/catalog checks, 132 workflow Python tests, runtime-only compilation, formatting and warnings-as-errors pass.

Feature profile: aot, canonical-native, canonical-native-program, canonical-native-program-runtime, canonical-native-runtime, jit. Libraries use opt3 and qualification roots opt1; rustc 1.96-nightly 2026-03-03, aarch64 Apple M1 / 8 GiB. The same frozen R196 Engine/Compute/Runtime/thin selected Stdlib and R145 Core/Math/Syntax/external dependencies are reused; the full math library is not rebuilt. Exact command arrays, dependency paths, source hashes and worker hashes remain in receipts. Workflow tests use the existing pinned temporary PyYAML 6.0.3 through explicit PYTHONPATH.

No unresolved introduced qualification failure remains. A read-only status-snippet typo and guessed missing file paths were corrected; no measured worker or production source was changed by them. The seven excluded native paths and skipped wider suites are inherited coverage limits, not newly passing tests. Skipped: fresh full Cargo/all-features/dependency/registry-project runs, multi-megabyte timings, the ten-workload suite, standalone scalar-latency, a new full state-source matrix and fresh 1,724-case execution. Ordinary exact-head hosted CI cancellation is requested and not awaited; no green hosted-CI claim. #973 stays draft and review is requested on the published head.

## Reproduction

After unpacking evidence, run `python3 evidence/heap-summary.py` then `python3 evidence/analyze.py`. They verify raw observations, balance, medians, image-equality receipts, cleanup and qualifications and reproduce results.json without executing a compiler or compressor. Current source/patch, actual disassembly, separate counter/heap observations, aggregate profiles and exact test commands are retained. Native binaries, bytecode, executable descriptors and raw stack traces are excluded.

[Immutable evidence and numerical replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r199-retirement-inlining-78830c5bcfa8.tar.gz) — SHA-256 `78830c5bcfa865211212c19eea2f0e0ae48c61250bf59e812a78323a9734cc83`, 2,347,634 bytes / 364 hash-listed files. A fresh public download verified every file and reproduced results.json byte for byte without compiler or compressor execution.
