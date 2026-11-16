# R203: whole compact-native diagnostic, copying bottleneck exposed

**The full unchanged level-12 compressor now executes as one typed native computation and produces the exact stream for all20 cases. Production stays R199 / IR214.** This first complete diagnostic is much slower on larger inputs because dead aggregate fields retain buffer aliases and force repeated copy-on-write. It is preserved as a negative baseline, not adopted and not interpreted as an isolated cost of the production guarantees.

The complete native entry uses the existing Plan, shared scalar/aggregate/branch emission, typed records/tags, real collection construction, private U8/U16/U32/U64 storage, reserve/growth, initialized copies, append/clear and final unique-output allocation transfer. Allocator helpers perform real allocation/free/COW; there is no generic canonical operation dispatcher, schema binder or frame transport in the generated computation. The algorithm, source, tables, real scratch capacities and source allocation sites are unchanged. Whole-value ownership liveness is inadequate: it does not yet satisfy the brief’s requirement to keep only live fields.

## Matched complete-call results

| Input | Retained / Roc | Compact diagnostic / Roc | Compact / retained |
|---|---:|---:|---:|
| reymont-prefix-32769 | 3.328× | 151.134× | 45.415× |
| 33027-periodic-window | 3.394× | 235.224× | 69.297× |
| repeated-32769 | 2.899× | 223.671× | 77.162× |
| random-32769 | 4.757× | 411.985× | 86.613× |
| 16384-distribution-change | 6.280× | 144.157× | 22.954× |
| eight-literals | 4.458× | 2.440× | 0.547× |
| reymont-prefix-16384 | 4.054× | 125.467× | 30.946× |
| empty | unresolved | unresolved | 0.002× |
| one-stored | unresolved | unresolved | 0.003× |
| seven-stored-prefix | unresolved | unresolved | 0.003× |
| eight-overlap | 4.281× | 2.334× | 0.545× |
| 320-overlap | 4.207× | 4.172× | 0.992× |
| eight-literals-prefix | 4.474× | 2.466× | 0.551× |
| 128-biased-dynamic | 3.766× | 2.601× | 0.691× |
| 32-high-entropy-stored | 4.016× | 2.804× | 0.698× |
| random-64 | 4.117× | 3.258× | 0.791× |
| random-256 | 4.156× | 5.061× | 1.218× |
| random-1024 | 3.797× | 7.933× | 2.089× |
| repeated-1024 | 4.044× | 8.600× | 2.126× |
| reymont-prefix-1024 | 4.563× | 10.029× | 2.198× |

Eight balanced blocks: 3,776 primary observations /3,856 complete calls. Each side is first four times. Fresh256-repetition reference elapsed results supply each row’s shared denominator; three zero-clock medians remain unresolved. There are 10 lower compact medians and 10/20 original1.05 gates pass. Those gates do not qualify the omitted ordinary contracts. No expected value or allowance is changed.

Preparation, clear/prefix, caller aliases, capacity and validation are outside the timed boundary on both sides. The compact boundary includes its actual Heap construction, full native computation, unique final payload transfer and private cleanup. Previous result publication/drop is excluded as in the retained evaluate_into harness. Input and still-live prepared output aliases remain unchanged. Physical reuse of borrowed output backing is not claimed. Exact raw bytes and independent miniz inflation pass for every observed call; miniz does not compress for Mech.

## Instructions, allocation and copy evidence

CPU and full global allocator cohorts are separate:16 retained observations plus warm/profile calls per side/kind/case,1,440 complete diagnostic calls total. No compiler/test/profile/archive competes with primary or counter measurement. No new hardware instruction ratio uses historical reference counts.

| Input | Instructions retained / compact | Allocation requests retained / compact | Requested bytes compact | Tracked initialized COW bytes compact |
|---|---:|---:|---:|---:|
| reymont-prefix-32769 | 208,351,264 / 4,195,647,944 | 6,449 / 515,206 | 400,283,694,952 | 16,397,989,720 |
| 33027-periodic-window | 72,490,334 / 2,448,875,964 | 3,679 / 203,200 | 161,366,677,275 | 8,904,553,436 |
| repeated-32769 | 66,843,137 / 2,418,269,324 | 3,135 / 200,052 | 160,105,452,319 | 8,766,639,309 |
| random-32769 | 71,016,107 / 3,132,017,645 | 5,201 / 543,998 | 240,386,402,249 | 13,378,320,622 |
| 16384-distribution-change | 64,686,492 / 925,107,972 | 5,679 / 313,030 | 180,110,390,592 | 2,718,088,506 |
| eight-literals | 12,433,229 / 6,473,882 | 3,354 / 3,650 | 95,196,360 | 3,121,824 |
| reymont-prefix-16384 | 118,861,568 / 1,326,394,868 | 6,805 / 307,476 | 227,965,150,828 | 3,892,663,648 |

Natural32 requests about400.3GB of capacity but tracks16.4GB of initialized COW copying and only15.0MB peak private payload. These are different quantities; the counter does not count every generated memmove/store. Full allocator incremental usable peak is15.26MB versus18.72MB retained. Process RSS/physical footprint also includes different linked code, metadata and initialized runtime state, so it is not a like-for-like incremental payload measure. Every nonreturned private payload/owner is gone at success.

Four fresh exact-compact stack profiles complete 7214 correct calls. Large-case native-exclusive timer weights are about9–10%; host below the entry is about90–91%, predominantly memmove. Eight literals are72.79% native-exclusive /18.30% host below entry. These are disjoint CPU timer samples, not instructions, dynamic call counts or removable-cost claims. The retained R199 stack census is explicitly reused; exact exclusive production host-family costs remain outstanding.

Code10,260,180B and entry stack83,264B versus production8,152,436B /16,480B. The compact image has70,889 static calls, not a dynamic callback count. State/refcount transport remains excessive; generic host removal alone has not made this representation efficient.

## Narrow build blocker and qualification

The first whole-image attempt was killed at65.17s with3,317,972,992B measured peak RSS. R194’s two earlier failures are retained. No exact OS/quota denial is proved. Inspection identifies dense Cranelift0.131.3 SSA variable×block bookkeeping and single-predecessor path caching. An isolated frontend copy replaces the inner SecondaryMap with a deterministic sparse BTreeMap and caches the queried block rather than every intermediate predecessor. It leaves the existing undefined/loop/merge algorithm intact. This is not a root dependency change.

All11 actual upstream SSA unit tests pass. Four verified standalone SSA probes produce byte-identical CLIF: at2,049 variables/blocks peak requested live bytes9,241,188→1,299,084. This probe is not the compressor’s total build memory. The full source Plan then compiles in278.878s with2,981,756,928B peak RSS. Source41,039 blocks /39,932 reachable and84,016 logical typed words remain large. Compiler compile time is excluded from execution timing.

Collection and real-capacity buffer controls pass through original/decoded images, byte-identical per pair. They exercise262144-entry construction,305257-U64 reserve, append/clear and13-byte output capacity with live aliases and selected checks. The full20-case smoke witness completes40 correct calls before the matched cohorts. The complete image consumes the unchanged pinned bytecode and verifies encode roundtrip; a second independently decoded whole image is not built. A missing shared-layout trait, a probe formatting error, missing heap-counter link object, and a measurement-script syntax error are corrected; initial failed receipts are preserved. Warnings remain denied. Production675-test/258-CI/component qualification is reused for unchanged production, not claimed as a new diagnostic qualification.

The ordinary exact resource admission/accounting, canonical external import/publication and precise error-prefix/recovery contracts remain omitted and explicit in the diagnostic entry. This is neither NoLimits semantics nor a replacement public compiler entry. No guarantee-restoration increment is claimed complete. The seven inherited production exclusions and inherited debug-profile accounting difference remain as recorded in R202. No new production failure is introduced by this evidence-only checkpoint.

## Next bounded work

Implement owner-field liveness over the same source CFG. Scalar/metadata use of a record must not retain its obsolete array fields; array data reads, genuine aliases, selected aggregate transfers, builder lifetime and final exclusivity still require ownership. Prove the distinction on original/decoded live-alias controls, then repeat this exact full20 witness and matched matrix. Until the extra copies are removed, this is not a useful performance floor. Direct reduction completion into the parent remains the next production optimization. Induction-derived exit accounting remains later; R202’s rejected balance-only experiment does not complete it.

Base5aef4af18e13318b3ea02ca1813d662485115fff; isolated source snapshot8879d21bcde5828a4244a5b029e72cda5ead76dd, production7748204bd12e57a20369ce7970f31f8d15c3901f, IR/work/ABI/MBC214/107/39/14. Source SHA331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9; bytecodea2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7. Depth300/nice258/originalEOF,262144U16/1500513U32/305257U64 and pinned mimalloc2.1.7 stay fixed. Frozen R145 dependencies/R196 engine/compute/stdlib, nightly-2026-03-03aarch64, opt3 -Dwarnings; exact feature/build commands are in receipts.

Run heap-summary.py, analyze-samples.py (raw samples stay local; archived summary is reused), then analyze.py to replay numerical results without compiling or executing compression. Published replay runs heap-summary.py and analyze.py against the archived aggregate sample summary. The complete brief remains open. Existing draft#973 only.

[Immutable evidence and numerical replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r203-whole-compact-copy-baseline-efaa95f3dcfe.tar.gz) — SHA-256 `efaa95f3dcfec9c54e81dff28d8fd5e04af5222ba0e879063c24a032d997db62`, 623,136 bytes / 321 hash-listed files. Fresh public download validates all hashes and reproduces results.json byte for byte.
