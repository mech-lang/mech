# R204: field and current-path ownership in complete native compression

**Production remains R199 / IR214. The complete diagnostic is correct on all20 cases and becomes about10× faster on the large natural-text case, but it is still slower than production.** The experiment now exposes excessive generated computation after removing the dominant copies. It is not adopted, and it is not an isolated cost-of-contracts floor.

The pinned source and bytecode are unchanged. Both independently generated whole-program original/decoded Objects and metadata are byte-identical; separately linked Rust workers execute all20 exact streams, independent inflate, input/prepared-output aliases, capacities and private cleanup. Constructors/reserve/growth/initialized copying/append/clear/final ownership transfer remain real measured operations. No compression library performs the Mech computation.

## Matched complete-call measurements

| Input | Retained / Roc | R203 diagnostic / Roc | R204 diagnostic / Roc | R204 / R203 |
|---|---:|---:|---:|---:|
| reymont-prefix-32769 | 3.342× | 149.617× | 14.445× | 0.096547× |
| 33027-periodic-window | 3.519× | 235.408× | 24.942× | 0.105954× |
| repeated-32769 | 3.019× | 225.168× | 23.178× | 0.102934× |
| random-32769 | 4.951× | 413.219× | 24.228× | 0.058631× |
| 16384-distribution-change | 6.362× | 143.656× | 20.891× | 0.145423× |
| eight-literals | 4.439× | 2.400× | 2.187× | 0.911016× |
| reymont-prefix-16384 | 4.147× | 123.205× | 15.082× | 0.122411× |
| empty | unresolved | unresolved | unresolved | 0.836000× |
| one-stored | unresolved | unresolved | unresolved | 0.901442× |
| seven-stored-prefix | unresolved | unresolved | unresolved | 1.000000× |
| eight-overlap | 4.403× | 2.377× | 2.185× | 0.919004× |
| 320-overlap | 4.178× | 4.166× | 3.029× | 0.727124× |
| eight-literals-prefix | 4.464× | 2.391× | 2.189× | 0.915809× |
| 128-biased-dynamic | 3.814× | 2.619× | 2.146× | 0.819247× |
| 32-high-entropy-stored | 4.054× | 2.832× | 2.437× | 0.860568× |
| random-64 | 4.122× | 3.240× | 2.661× | 0.821046× |
| random-256 | 4.273× | 5.137× | 3.593× | 0.699489× |
| random-1024 | 3.871× | 7.953× | 4.405× | 0.553902× |
| repeated-1024 | 3.938× | 8.277× | 4.902× | 0.592175× |
| reymont-prefix-1024 | 4.613× | 9.956× | 6.095× | 0.612226× |

One three-way primary campaign: 8,496 observations /8,616 complete calls. All six side-order permutations run twice, so each side is first/middle/last four times. Large cases retain24 observations per side; smaller cases192. Fresh256-repetition level12 reference elapsed data supplies each row’s common denominator; three zero-clock medians remain unresolved.19 new diagnostic medians fall and one is equal.11/20 unchanged1.05 production gates pass; this unqualified diagnostic cannot be adopted even where its elapsed time is lower.

All timed calls are sequential and isolated from compilation, tests, profiling and archives; startup is concurrent and excluded. The compact boundary includes Heap construction, full computation, unique final payload transfer and private cleanup. Seed/clear/prefix, still-live borrowed arguments, output slots, validation and prior result drop remain outside the boundary as in the evaluate_into harness. Capacity-level reuse is checked; physical reuse of borrowed output backing is not claimed. No allowance, expected value, algorithm, table, depth300, nice258, EOF rule or source allocation site changes.

## Two distinct ownership findings

The first field-only variant removes484 payload requests on natural32 and reduces code/stack, but barely changes the16.4GB copy volume. Preserve that unsuccessful full witness and its exact source, Object hashes and machine excerpts. The focused dead-field example was necessary but did not predict the real bottleneck.

Field liveness distinguishes allocation-owner lanes from scalar metadata. Payload reads keep owners; length/tag reads do not. Aggregate transfers propagate the owner fields actually needed by the destination. Existing all-path source initialization stays independent. Dropping an unused array field never authorizes consuming a genuinely live alias; the concrete heap retains its final reference-count/exclusivity check.

A separate diagnostic forwards the exact C writable ABI and records its actual native return PC, current owner count, consume flag, capacity and initialized length. On natural32,163,845 fallbacks have one current owner and enough capacity but consume=0. They request400,118,665,320 capacity bytes and copy16,259,867,920 initialized bytes. Actual Object return PCs map them to append sites12806/15683 in the unchanged optimal-path scan. Its repeated more predicate has a contradictory old-buffer return path in ordinary syntactic liveness. The generic existing unsigned-buffer retirement inventory conservatively declines these sites; no previous completed ownership optimization is relabeled as new.

The second change derives must-facts for exact Bool value homes on the same closed Plan. Joins intersect facts; every definition kills its old fact. No relation is inferred between distinct predicates. A future-owner query prunes only edges contradicted by a current must-fact and declines if any supporting predicate is redefined before the old owner is killed. It never deletes source branches/checks or grants runtime exclusivity. Ordinary owner retirement remains conservative. Both real append sites qualify; the final exact-Object trace records **zero path-buffer writable fallbacks**. New position visits are not separately instrumented; the163,845 count describes the previous helper calls, not hardware instructions.

## Separate CPU, allocator and memory cohorts

| Input | Instructions retained / R203 / R204 | Allocation requests retained / R203 / R204 | R204 requested bytes | R204 incremental usable peak |
|---|---:|---:|---:|---:|
| reymont-prefix-32769 | 208,297,776 / 4,195,605,374 / 649,636,076 | 6,449 / 515,206 / 9,224 | 12,057,256 | 10,013,032 |
| 33027-periodic-window | 72,493,410 / 2,448,858,361 / 591,787,702 | 3,679 / 203,200 / 932 | 10,262,803 | 9,972,104 |
| repeated-32769 | 66,844,273 / 2,418,272,511 / 588,075,124 | 3,135 / 200,052 / 354 | 10,123,399 | 9,972,104 |
| random-32769 | 71,012,593 / 3,132,026,980 / 278,326,534 | 5,201 / 543,998 / 5,750 | 11,913,301 | 9,972,104 |
| 16384-distribution-change | 64,689,292 / 925,105,322 / 181,461,186 | 5,679 / 313,030 / 4,670 | 10,238,392 | 9,213,016 |
| eight-literals | 12,434,418 / 6,473,575 / 4,999,134 | 3,354 / 3,650 / 498 | 9,091,628 | 9,193,416 |
| reymont-prefix-16384 | 118,860,837 / 1,326,412,336 / 324,438,508 | 6,805 / 307,476 / 9,630 | 11,050,220 | 9,213,768 |

Each CPU/heap cohort uses eighteen retained observations plus warm/profile calls, with all six three-way permutations:2,400 separate complete calls. Full instructions/cycles and RSS/physical/heap measurements for every case remain in results.json. CPU samples are not converted into instruction counts, and independent cohorts are not multiplied together. No new instruction ratio uses historical reference counters.

Natural32 full allocator requests515,206→9,224, requested bytes400,283,694,952→12,057,256, incremental usable peak15,263,080→10,013,032. Tracked initialized COW copies16,397,989,720→2,178,080; payload requests257,598→4,608. Requested capacity, initialized copy volume, private payload peak, allocator usable peak and process RSS are different metrics. The COW counter excludes direct generated memmove/store work. Inputs are borrowed and remain unchanged; this counter alone is not a streaming or total-copy measurement.

## What still makes the diagnostic slow

Fresh four-case wall-stack sampling completes8,297 correct calls. Native-exclusive large-case weights are98.9996% natural /99.2554% periodic /99.2986% repeated; host below entry is below0.7%. Eight literals are80.6029% native /12.7567% host below entry. These are disjoint timer weights with direct children subtracted, not removable-cost or instruction fractions. The retained R199 stack census is explicitly reused. Exact exclusive retained-host cost by operation family remains outstanding.

Three fresh exact-executable PC profiles complete973 correct calls. They identify a concrete comparison defect: the raw compact lowering has not retained the ordinary compiler’s numeric specialization throughout the program. Periodic/repeated hot PCs0x69594/0x681a4 assemble words from individual byte loads and shift/OR chains, with repeated large-offset stack reloads. The exact retained Object instead has two64-bit loads and a15-instruction equal-word matching edge at0x695fc. Natural-text hot PCs0x66dd3c/0x66ffac retain general state tests, scalar selection and owner/spill work; they are not evidence for a per-function instruction total. Current disassembly and matching Object hashes are retained; no stale source map is used.

This explains why removing generic host dispatch is insufficient here and why restoring contracts immediately would not isolate their cost. Natural32 still executes649.6M instructions versus208.3M retained; repeated32 executes588.1M versus66.8M. The complete diagnostic must preserve existing qualified numeric computation lowering before it can provide a useful cost-of-contracts baseline. This is an experimental-path regression, not a regression of the retained3.3× compiler.

Code/entry stack: R20310,260,180B /83,264B; final8,315,704B /78,160B; production8,152,436B /16,480B.58,193 is a static call count, not dynamic callbacks.562 is a static buffer-source permission count, not executed or successful in-place operations. The field-only image has8,298,964B /78,096B; all stages are archived.

## Qualification, failures and provenance

180 focused cases across20 baseline/candidate original/decoded workers pass: dead fields versus live old-array aliases, same versus independent predicates, predicate reassignment, fresh recursive visits, selected failures, unchanged result slots/prior publication and success→failure→success. Existing collection8 and buffer12 controls pass in both forms, plus four real shared-rejection/unique-U8-transfer sequences. The real262144-U16 constructor,305257-U64 capacity, append/clear and13-byte output capacity remain covered. The complete original and separately linked decoded smoke witnesses add80 calls. All measured/profiled calls verify exact reference bytes, independent decompression and cleanup.

The source definition/lexical checks and all ordinary production qualification remain unchanged. R199/R202’s675 native tests,258 executed CI selections and component/architecture/workflow qualifications are reused for unchanged production, not claimed as fresh qualification of this omitted-contract diagnostic. The seven inherited exclusions and inherited debug-profile work discrepancy remain as recorded in R202. Eleven upstream sparse-SSA tests/four same-CLIF probes are reused from unchanged R203 frontend; no root dependency is adopted.

Introduced setup failures are corrected and preserved: relative command paths under a helper with a fixed working directory, a formatting-sensitive insertion, an unused diagnostic transfer method caught by warnings-as-errors, and a missing space before a Rust ABI literal. A malformed temporary question-mark binding is corrected to the supported Bool binding syntax. A separate helper-based alternative fixture reaches an inherited native WitnessTypeMismatch under the clean R203 producer; its exact source/log remains archived, it does not block the pinned compressor, and no fix or successful execution is claimed. Hardware reads initially fail in the restricted environment and then succeed through the authorized read; no compiler/resource allowance changes.

Ordinary exact accounting/admission, precise error-prefix/recovery, canonical external import and current publication settlement remain omitted by design. This is not ordinary NoLimits. Its private fresh-heap failure/retry controls do not establish the full managed recovery contract. Finite-budget/error-prefix behavior, scalar capability, strict bytecode rejection and architecture/catalog checks are not weakened in production. No new production failure is introduced by this evidence-only checkpoint.

Review base9e38c3346fd856b9b0d45824af21002640011676; compiler production7748204bd12e57a20369ce7970f31f8d15c3901f, IR/work/ABI/MBC214/107/39/14. The isolated source continues R203’s8879d21 snapshot with hash-listed patches. Source SHA331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9; bytecodea2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7. Raw entry is the separate typed mech_compact_closed_v2 ABI, not managed ABI39. Source buffers262144U16 /1500513U32 /305257U64, pinned mimalloc2.1.7 and timing preparations remain fixed. Hardware AppleM1 /Macmini9,1 /8GiB; nightly-2026-03-03aarch64, opt3 -Dwarnings, selected canonical-native/program/runtime/aot/jit features with frozen R145/R196 dependencies. Exact command arrays and dependency hashes are included.

Final full builds: original199.484s /2,005,352,448B peak RSS; decoded200.190s /2,407,006,208B. They are outside performance claims and are not a matched compiler-speed cohort. The earlier field-only dual build396.633s /2,474,229,760B is preserved. No exact OS/quota cause is assigned to the historical R194/R203 SIGKILLs.

## Milestone disposition and next bounded work

A complete stream and a matched whole-program baseline now exist. Field/private-buffer ownership is substantially better, but the raw complete path still needs existing optimized numeric kernels and smaller live computation state. Reuse the maintained packed-word/numeric-region emission rather than invent a new compressor-specific algorithm. The next retained-production target remains direct reduction completion into its parent, preserving ties, no-strict-improvement, selected errors, exact prices and recovery. The induction-derived funded-loop accounting proposal remains later; R202’s rejected balance-only experiment did not complete it.

The four guarantee-restoration increments are not started. The exclusive production-host operation-family ledger is still open. The full brief and parity are not complete. Only existing draft#973 is used; ordinary exact-head CI is cancelled under standing instructions and automated review is requested.

Numerical replay: heap-summary.py then analyze.py. The committed sample/PC summaries and disassembly are inputs; raw stacks/traces, native binaries, bytecode and executable descriptors stay local. Replay does not run a compiler or compressor.

[Immutable evidence, full compiler patches and numerical replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r204-compact-guarded-ownership-651f4be7cec1.tar.gz) — SHA-256 `651f4be7cec1ece04b418d24dd1bfb43067dba2bf4c9fe45d42728c37b7cf355`, 1,196,876 bytes / 591 hash-listed files. Fresh public download validates every file and reproduces results.json byte for byte.
