# Focused native profiling build

Build the compression profiling tools as an ordinary consumer of the maintained
compiler and catalog. Building a `mech-gpu` example activates that package's
dev-dependencies, including `mech-stdlib/full_compiler`, even with
`--no-default-features`. That selects the entire math library.

This manifest selects five math operations (add, subtract, multiply, divide and
remainder), six comparisons, Boolean and/or, and inclusive ranges. Only the
math, comparison, logic and range machine crates are built. The other local
machine patches are needed for Cargo's resolution of optional dependencies;
they do not enable those crates. Compiler, syntax and value features retain the
previous configuration, including the Bool/U64/I64/F64 scalar controls.

From the repository root, use the same target directory on subsequent builds:

```sh
CARGO_TARGET_DIR=/tmp/mech-native-profile-target CARGO_INCREMENTAL=0 \
  cargo build --locked --release -j1 \
  --manifest-path benchmarks/mech-deflate/native-profile/Cargo.toml \
  --bins --message-format=json
```

The repository's warnings-as-errors configuration still applies. For a pinned
offline experiment, also select its toolchain and pass `--offline`. Record the
Cargo JSON output to identify the actual linked libraries; do not select an
arbitrary similarly named file from the target directory.

For subsequent optimized compiler rebuilds on the eight-core, 16 GB Apple M3
profiling host, use four Cargo jobs and the existing optimized profile:

```sh
CARGO_TARGET_DIR=/tmp/mech-native-profile-target CARGO_INCREMENTAL=0 \
  cargo build --locked --offline --profile compiler -j4 \
  --manifest-path benchmarks/mech-deflate/native-profile/Cargo.toml \
  --bin compile_profile --message-format=json
```

Cargo jobs schedule independent crates. The latest warm-cache rebuild took
275.189 seconds with two jobs; its three rebuilt crates form a dependency chain.
No four-job speedup has been measured. Thin LTO and one codegen unit remain in
the profile. Rust compiler rebuilding and Mech program compilation have separate
timings; increasing Cargo jobs does not parallelize Mech's compiler process.

`native_frame_transport` uses the existing example source without a copy.
`artifact_review SOURCE ARTIFACT` compiles the full compressor source through
the ordinary catalog, requires byte-for-byte agreement with the pinned artifact,
and checks decoded re-encoding. Run it before using this profile for timing.
The timing harness, allocator, compressor source, canonical artifact and
execution boundary must remain pinned independently of this build selection.

This is a local profiling profile. The full library qualification profiles stay
available and unchanged. A first build may rebuild dependencies because Cargo's
feature graph changed; reuse this profile for subsequent iterations. Build time
is separate from compression runtime, and switching profiles must be disclosed
in before/after comparisons.

The nondefault `linked-qualification` feature adds `mech-build` and its process
supervision for the maintained linked/Object tests. It can change transitive
dependency features and rebuild compiler libraries. Run it separately, record its
Cargo artifacts, then restore the default profile before primary timing and
scalar controls. It is not part of the timing feature configuration.

`unit-qualification` additionally enables the runtime's compiler interface needed
by existing GPU unit-test helpers. It retains the selected operation catalog;
it does not activate the GPU package's full-catalog dev dependencies.

## Tiny fixed-block qualification

`short_literal_proof` runs the maintained compressor predicate through original
and decoded artifacts with the established finite native qualification policy:

```sh
CARGO_TARGET_DIR=/tmp/mech-native-profile-target CARGO_INCREMENTAL=0 \
  RUSTFLAGS='-D warnings' cargo +nightly-2026-03-03 test --locked --offline \
  --release -j2 --manifest-path benchmarks/mech-deflate/native-profile/Cargo.toml \
  --test short_literal_proof -- --nocapture
```

The [cost proof and measurement status](../../../docs/design/native-tiny-fixed-block.md)
separate the source shortcut, allocator crossover, primary elapsed samples,
instruction/memory cohort and remaining full-matrix timing gate.

## Compile-time profiling

`compile_profile` measures parsing, the source catalog, checked source compilation,
canonical artifact construction/encoding and native Object emission separately.
The `benchmark` source is reconstructed from the maintained compressor components
with the exact two-input entry used by the complete level-12 comparison. Its
SHA-256 is checked before compilation. `fixture` selects the public application
entry, and a source filename selects another program. The output directory keeps
source, canonical bytecode, Object, metadata and the phase receipt.

```sh
CARGO_TARGET_DIR=/tmp/mech-native-profile-target CARGO_INCREMENTAL=0 \
  cargo build --locked --offline --release -j2 \
  --manifest-path benchmarks/mech-deflate/native-profile/Cargo.toml \
  --bin compile_profile

python3 benchmarks/mech-deflate/native-profile/profile_compile.py \
  /tmp/mech-native-profile-target/release/compile_profile \
  benchmark /tmp/level12-compile compact sample
```

The optional final `sample` argument records consecutive 20-second macOS CPU
samples at 5 ms intervals. Omit it for a compile without sampling. Stage logs are
stamped independently of the compiler's phase timers. Process time includes
startup, final output writes and teardown; the compile receipt ends after native
emission and before final output writes. `compact` selects the same diagnostic
Object API as the compression benchmark. `managed` selects the ordinary Object
API, while `frontend` stops after canonical encoding. These modes have different
compilation boundaries; do not pool their timings. Rust tool builds and linking
a generated host application are separate from every measured source compile.

Process completion is now timestamped by a thread blocked in `process.wait()`;
the receipt records `driver_start_to_process_wait_completion`. The main driver's
0.2-second wait for sampling readiness no longer rounds up process completion.
Historical receipts without this boundary field retain their original polling
tail and are not rewritten. This is a measurement correction, with no compiler
speedup credited to it. The external timer includes source assembly and all
output writes; the existing internal compiler timer retains its narrower phase
boundary. Report both and compare subjects using the same driver.

The guard state, owner-read filter, numeric liveness solver and vendored SSA builder
have focused test targets. They
compare against the original analyses and independent read-before-definition
paths, including contradictory guards, joins, redefinition, cycles, sparse IDs,
shared-state independence, packed word boundaries, bounded dense scratch, sparse
fallback and SSA cycle-detection generations:

```sh
CARGO_TARGET_DIR=/tmp/mech-native-profile-target CARGO_INCREMENTAL=0 \
  cargo test --locked --offline --release \
  --manifest-path benchmarks/mech-deflate/native-profile/Cargo.toml \
  --test guard_state --test numeric_liveness --test ssa_builder \
  --test dimension_remap --test old_value_reads --test numeric_reachability
```

The [Apple M3 compile-time report](../../../docs/design/native-compile-time-profile.md)
records the measured before/after work and exact output checks. Raw evidence is
archived under ignored `.experiments/`; only its compact receipt belongs in source Git.

The [sub-minute follow-up](../../../docs/design/native-compile-under-minute-profile.md)
records the guard/ownership and preparation optimizations, two fresh compiles of
the current qualified worker, unchanged native output and offline replay.

The [canonical parser profile](../../../docs/design/native-parser-compile-profile.md) records the grammar-preserving parser speedup and the fresh pinned C/Roc comparison.

The [parser execution-work follow-up](../../../docs/design/native-parser-work-amplification.md) records continuation/checkpoint counts and two rejected frontend schema screens.

The [predictive parser report](../../../docs/design/native-parser-prediction-profile.md) qualifies another 28% parse-time reduction, identical emitted program bytes and the remaining roughly 50-second full compile.

The [bounded ownership-query report](../../../docs/design/native-owner-read-compile-profile.md) records a further 19.7% preparation reduction and 6.3% full-compile reduction to about 47 seconds, with identical output bytes and 14,314 matching real-plan query results.

The [sparse SSA lookup report](../../../docs/design/native-ssa-sparse-compile-profile.md) records source CFG construction falling 67.9% and full compilation falling 11.0% to about 42 seconds, with identical emitted bytes and all 61 focused tests passing.

The [scoped numeric-facts report](../../../docs/design/native-numeric-facts-compile-profile.md) records a further 5.1% native reduction and full compilation near 40.24 seconds, with identical output bytes, six matching smaller programs and a separate frontend CPU sample.

The [inline dimension-remapping report](../../../docs/design/native-frontend-affine-compile-profile.md) records frontend time falling 21.6% and complete compilation to 38.90–39.11 seconds, with identical outputs and 63 focused tests passing. The separately rebuilt PR integration preserves the same outputs in 39.96 seconds.

The [scalar constructor trial](../../../docs/design/native-resolved-scalar-compile-trial.md) is rejected: frontend time rises 5.85% and instructions 1.72%, despite 271 passing Core tests and identical outputs. Production is restored; fresh retained-compiler samples identify repeated numeric-exit reachability as the next target.

The [numeric forest reachability report](../../../docs/design/native-numeric-reachability-compile-profile.md) records preparation falling 17.1% and complete compilation to 37.11–38.22 seconds. All outputs remain identical, 66 focused tests pass, and the sub-10-second goal remains open.

The [checked tail-finish role report](../../../docs/design/native-tail-role-compile-profile.md) records another 4.3% preparation reduction and complete compilation to 36.73–36.88 seconds. Fourteen existing engine validation tests and six complete fixture comparisons pass with identical outputs. The report retains the initial Cargo selection error and excludes a zero-test filter attempt from qualification.

The [rootless dimension report](../../../docs/design/native-rootless-dimension-compile-profile.md) records frontend time falling 16.1% and complete compilation to 36.10–36.39 seconds. All outputs remain identical; 269 Core/type-system tests and `no_std` pass. The candidate's whole-compiler RSS is adverse and is retained in the report.

The [current process and peer profile](../../../docs/design/native-compiler-process-profile.md) records two fresh C/Roc builds each, eight Mech CPU windows and a complete main-entry IR census. It identifies 19.5 million distinct temporary SSA aliases and repeated semantic/lexical lookup as concrete optimization targets. Clean Mech process timings remain 36.52–37.14 seconds; diagnostic timings are separate.

The [lexical-site index report](../../../docs/design/native-lexical-index-compile-profile.md) records canonical lowering falling 94.3%, preparation 13.3% and complete compilation 12.6% to 31.49–31.53 seconds, with identical program bytes. All 30 focused tests, 114,688 differential queries and six complete fixture comparisons pass. Both subjects include the newly pulled PR 973 reduction-bound option; candidate mean RSS is adverse.

The [shared-schema compiler result](../../../docs/design/native-shared-schema-compile-profile.md)
records complete fresh compilation at 21.196–21.415 seconds, with identical
generated bytes. Complete packets, receipts and offline replay are indexed in
[compiler-profile evidence](../evidence/compiler-profile-20261114/README.md).

The [scope-sized validation result](../../../docs/design/native-scope-graph-compile-profile.md)
records fresh compilation at 20.368–20.777 seconds, preserving every generated
byte and the original admission limit. Its complete packet is in the same
[compiler-profile evidence index](../evidence/compiler-profile-20261114/README.md).

The [proof-local traversal result](../../../docs/design/native-local-traversal-compile-profile.md)
records fresh compilation at 19.741–20.163 seconds with unchanged generated bytes.
Its full archive, receipt and replay are in the
[compiler-profile evidence index](../evidence/compiler-profile-20261114/README.md).

The [packed helper-liveness result](../../../docs/design/native-helper-liveness-compile-profile.md)
records fresh compilation at 18.884–19.110 seconds with identical generated bytes.
Its full archive and offline replay are in the
[compiler-profile evidence index](../evidence/compiler-profile-20261114/README.md).

The [index-range facts result](../../../docs/design/native-index-facts-compile-profile.md)
records fresh compilation at 18.691–18.903 seconds with identical generated bytes.
Its full archive and offline replay are in the
[compiler-profile evidence index](../evidence/compiler-profile-20261114/README.md).

The [selected exit-definition qualification](../../../docs/design/native-exit-definitions-compile-profile.md)
preserves whole-inventory validation and removes full-value enumeration from
individual exit queries. Sixteen matched complete processes take
18.690–19.107 seconds (-0.27%
mean change); preparation changes -0.60%. Overall
compiler time is effectively flat with mixed block directions. All generated
bytes match. Seventeen focused Rust tests and fourteen fixture pairs pass.
Every phase, adverse counter and failed setup is retained in the sixteenth
[complete repository packet](../evidence/compiler-profile-20261114/README.md).

The [nested parser qualification](../../../docs/design/native-nested-parser-compile-profile.md)
applies existing bounded predictions to nested base calls without changing grammar
or logical work. Eight matched complete processes take
18.467–18.483 seconds (-2.20%
mean change); parsing changes -13.87%. Generated bytes match.
All 526 document-module units, 1,667 full parse comparisons and fourteen fixture
pairs pass. Seven separate CPU windows, all adverse values and setup failures
are in the seventeenth [complete packet](../evidence/compiler-profile-20261114/README.md).

The [local-span SSA storage trials](../../../docs/design/native-ssa-span-trials.md)
reject both private layouts: source CFG construction increases 5.53% and 5.07%,
compiler phases 0.61% and 0.94%, and instructions 0.55% and 0.48%.
All output bytes match; 17/18 focused tests, 24 paired complete IR cases and
fourteen fixture pairs pass per variant. Both compiler-time blocks are slower.
Source is restored to the retained nested-parser compiler. Every observation,
patch and probe is in the eighteenth [complete packet](../evidence/compiler-profile-20261114/README.md).

The [word-home trials](../../../docs/design/native-word-home-compile-profile.md)
reject deferred entry zeros and direct indexed lookup. Source CFG time changes
+0.015% over eight observations and +0.107% over sixteen; instructions change
-0.078% and -0.031%. The favorable original indexed-home cohort and mixed repeat
both remain. Objects match; all metadata matches for indexed homes and only two
diagnostic counts differ for deferred zeros. Both sources are restored. The
nineteenth [complete packet](../evidence/compiler-profile-20261114/README.md)
includes all measurements, source snapshots, tests and fourteen fixture pairs per
trial. No compiler saving or runtime-latency campaign is credited.

The [bounded exit-liveness qualification](../../../docs/design/native-point-liveness-compile-profile.md)
uses a fresh 61-flag control after private overwrite-result reuse. Complete
source-to-Object processes take 17.420–17.638 seconds; compiler phases change
-5.08%, preparation -32.52% and instructions -4.97%. Both balanced blocks improve.
Objects and complete metadata match; compiler RSS increases 1.93%. Sixteen tests,
790 independently re-solved exact graphs and fourteen fixture pairs pass. Every
raw measurement, kernel budget, output and capture correction is in the twentieth
[complete packet](../evidence/compiler-profile-20261114/README.md). The ten-second
goal remains open.

The [batched SSA sealing trial](../../../docs/design/native-ssa-batch-seal-trial.md)
is rejected: source CFG construction increases 1.59%, compiler phases 0.39%,
and instructions are effectively unchanged across eight matched 61-flag compiles.
Both compiler-time blocks are slower. All output bytes match; fifteen focused
tests, 88 paired complete IR cases and fourteen fixture pairs pass. Production
source is restored. All measurements and the four-job rebuild selection are in
the twenty-first [complete packet](../evidence/compiler-profile-20261114/README.md).

The [parallel compilation design](../../../docs/design/native-parallel-compilation.md)
separates function jobs from ordered module publication and records the current
variant-choice dependencies. Its twenty-second
[evidence packet](../evidence/compiler-profile-20261114/README.md) contains two
controls with precise process timing, separate schema/SSA/cleanup counters,
the failed diagnostic build, and the frozen runtime/entry adapter. Three warm
native links take 0.081–0.093 seconds and reproduce every final executable byte;
three earlier attempts fail the byte gate and remain recorded. At that
publication no parallel Mech compiler saving or PGO gain was established.
The later integrated-worker and independent-PGO qualifications are separate.

The [independent compiler PGO result](../../../docs/design/native-pgo-compile-profile.md)
reduces complete source-to-executable compilation 17.606 to 15.973 seconds
(9.27%) across eight balanced serial builds with a cached fixed adapter.
Every program and executable byte matches. A separate source-to-Object cohort
improves process time 11.30% and instructions 10.18%, with its slow control
retained; frontend and encoding regress slightly. Fourteen fixture pairs match,
and seven held-out programs receive 56 additional balanced compiles. All raw
profiles, build/training/timing logs, outputs, configuration and pure replay
are in the twenty-sixth [complete packet](../evidence/compiler-profile-20261114/README.md).
This pair uses frozen `fad53` source before schema reuse and phase workers;
the later current-source native-CPU + PGO cohort is separate and gains must not
be added.
The ten-second complete-compilation goal remains open.

The [constant-phi propagation trials](../../../docs/design/native-constant-phi-compile-trials.md)
reject two worklist layouts and restore the original solver exactly. Each has
eight balanced source-to-Object compiles, four passing focused tests (512 graph
cases and 72 complete post-pass IR pairs), 1,782 matching complete-map calls,
fourteen fixture pairs and exact executable link gates. The first increases
cleanup/instructions; the compact revision lowers work slightly but its separate
eight-build executable cohort is 0.46% slower with mixed blocks. All data,
source/patches and replay are in the twenty-seventh
[complete packet](../evidence/compiler-profile-20261114/README.md). Both subjects
use the same 61 flags plus two workers, with PGO disabled. No useful complete-build
gain or sub-ten-second result is claimed.

The [empty repair traversal trial](../../../docs/design/native-route-work-compile-trial.md)
is rejected and source/tool are restored. Eight source-to-Object compiles increase
compiler phases 0.59% and cleanup 0.45%, with both blocks slower and RSS +0.98%;
instructions fall only 0.04%. One differential test passes 128 complete
IR/alias/machine comparisons; fourteen fixture pairs and cached full-executable
gates match. No balanced complete-executable cohort is run. Two separate
current-worker stack samples preserve every program byte and show per-edge
source range-map copying for subsequent investigation. Sampled times are
diagnostic, excluded from primary means. The twenty-eighth
[complete packet](../evidence/compiler-profile-20261114/README.md) includes
source/patches, tests/failure, every observation, native inputs, samples and replay.
No additional parallel gain or sub-ten-second result is established.

The [current-source native-CPU + PGO qualification](../../../docs/design/native-current-pgo-compile-profile.md)
uses frozen `aaef` source with the retained frontend changes and two workers in
both subjects. Eight complete builds improve 17.347 to 15.921 seconds (8.22%),
with both blocks faster and every executable byte identical; the fixed adapter
is cached. Eight source-to-Object compiles improve process/instructions
9.06%/9.24%; frontend (+3.01%), RSS (+1.87%), encoding (+2.52%) and footprint
(+1.92%) are adverse. Fourteen fixture pairs and 56 balanced held-out compiles
preserve all bytes. Four tool builds, training8, raw/merged profiles, every
diagnostic/observation/output and pure replay are in the twenty-ninth
[complete packet](../evidence/compiler-profile-20261114/README.md). This qualifies
the combined host-build strategy; earlier percentages stay separate. Complete
compilation below ten seconds remains open.

The [updated-compressor parallel result](../../../docs/design/native-refreshed-parallel-compile-profile.md)
uses `934` source after `9e34` Deflate/runtime changes. Eight complete builds
average 19.166 seconds serial versus 19.196 seconds with two workers (+0.16%),
with mixed blocks and no useful saving established. All program and executable
bytes match; the fresh current runtime adapter is cached, with first-use
construction separately recorded. One standard compiler binary and the same
61 flags compare scheduling 0/2. All seven worker source files remain unchanged
from `1090`; default remains serial. Every observation, source/runtime/worker
snapshot, output/native byte and replay is in the thirtieth
[complete packet](../evidence/compiler-profile-20261114/README.md). Earlier
smaller-workload and PGO timings stay separate; the ten-second goal remains open.

The [scoped source-facts qualification](../../../docs/design/native-scoped-source-facts-compile-profile.md)
retains `972039beaa`: complete builds improve 19.161 to 18.646 seconds (2.68%),
with both blocks faster and every program/executable byte identical. Eight
separate Object runs reduce cleanup 13.21%, instructions 3.26% and compiler
phases 2.45%, with slight RSS/footprint and other-stage regressions retained.
Five tests compare 112 complete IR/machine pairs and 1,760 route queries with
full map restoration. Fourteen fixture pairs, default-serial outputs and full
native gates pass. Two later stack samples are diagnostic. The thirty-first
[complete packet](../evidence/compiler-profile-20261114/README.md) contains all
source/patches, proofs/failures, measurements/outputs, native inputs and replay.
This removes per-edge source-map clones, keeps all proofs fresh, and changes no
grammar/language. Complete compilation below ten seconds remains open.

The [SSA lookup trials](../../../docs/design/native-ssa-lookup-trials.md) reject
inline-first definition storage and shared predecessor roots on the current
Deflate/runtime. Compiler phases increase 0.79%/0.38%, source CFG 4.50%/1.02%
and instructions 1.00%/0.26%; both balanced blocks regress in all three. Every
program and cached complete-executable byte matches. The 29/30 copied-module
tests, 160 paired complete IR cases and fourteen fixture pairs per trial pass.
Both sources and the qualified canonical tool are restored. All observations,
the first sandbox-denied counter attempt and pure replay are in the thirty-second
[repository packet](../evidence/compiler-profile-20261114/README.md). No balanced
full-native timing cohort is run for these rejected trials. The retained complete
Deflate build remains 18.646 seconds; the ten-second goal remains open.

The [frontend cache trials](../../../docs/design/native-frontend-cache-trials.md) reject both normalized-type caching and
preservation of the existing annotation schema owner. Frontend improves
0.63%/2.27% and instructions 0.12%/0.33%, in both blocks for each trial, but the
type-cache compiler is 0.40% slower and annotation compiler time is flat/mixed.
Eight separate annotation complete-executable builds average 18.563 to 18.588
seconds (+0.13%), with mixed blocks. Every program and executable byte matches;
15/28 focused tests and fourteen fixture pairs per trial pass. Both sources and
the qualified tool are restored. The thirty-third [complete packet](../evidence/compiler-profile-20261114/README.md)
preserves all raw measurements, sources/patches, tests, outputs, native inputs,
selection diagnostics and offline replay. Initial exploratory profile parsing
and a test-count description correction remain. No complete-build improvement
or sub-ten-second result is established.

The [late known-origin SSA trials](../../../docs/design/native-known-origin-ssa-trials.md)
keep original sealing and fallback paths. The larger shortcut removes 5.28 million
temporary values but increases compiler time 1.92% and source CFG 18.67% in eight
balanced Object observations; both blocks are slower. Instructions fall 0.56%
and compaction 25.67%, with a small RSS increase. Source/tool are restored;
all four outputs and complete metadata, fourteen fixtures and both complete executable gates match.
No primary full-native cohort is run. The pending-only predecessor is built
and tested, then superseded without primary program timing. Each has 304 paired
compacted IR/machine cases and 30 or 32 unit tests. Comparing the old early-sealing
implementation after current compaction still finds 87 IR and 30 machine differences.
The thirty-fourth packet retains both sources/patches, every test/setup failure,
all observations/outputs, native inputs and pure replay. Sub-ten-second
compilation remains open.

The [compiler allocator qualification](../../../docs/design/native-compiler-allocator-compile-profile.md)
retains source `d834508336`, whose entire seven-file diff matches the measured
patch. Eight complete builds improve 18.538 to 16.965 seconds (8.49%), with both
blocks faster and identical executable bytes. A separate eight-run Object
cohort reduces compiler time 9.45%, instructions 13.57%, parsing 12.08% and
preparation 16.71%. Compiler maximum RSS rises 10.04% (about 0.61 GB); footprint
rises 10.24%. All observations remain. The shared pinned allocator module is
registered in compiler executables; libraries and emitted programs keep their
allocators. Fourteen fixture pairs and default-serial full-output/executable
gates pass. The real native CLI check passes with `cranelift_build`; the same
existing default-feature errors are reproduced on the original baseline and
candidate. The thirty-fifth packet preserves all source/patches, builds/checks,
crate archives, failures, observations/outputs, native inputs, whole-commit
proof and pure offline replay. The sub-ten-second goal remains open.

The [control schema owner qualification](../../../docs/design/native-control-schema-owners-compile-profile.md)
retains source `12c5001086`, whose entire four-file diff matches the measured
patch. Eight complete builds improve 16.904 to 16.084 seconds (4.85%), with both
blocks faster and identical executable bytes. A separate eight-run Object cohort
reduces frontend time 34.41%, compiler time 5.45%, instructions 4.39% and maximum
RSS 32.99% (6.70 to 4.49 GB). Parsing, preparation, cleanup and catalog means
increase slightly; every observation remains. Private captures/parameters keep
existing immutable schema owners and their pure canonical data; selection
borrows input during fresh remapping. No new cache, grammar, inventory/identity
pruning or weaker fresh proof is introduced. Thirteen existing foundation tests,
fourteen fixture pairs, native CLI and default-serial byte/executable gates
pass. Two earlier current-control stack diagnostics are separate from primary
means; corrected call-graph analysis and demangling replay exactly. The
thirty-sixth packet retains all source/patches, tests, observations/outputs,
profiles, native inputs, entire-commit proof and pure offline replay. Complete
compilation below ten seconds remains open.

The [current compiler parallel comparison](../../../docs/design/native-current-parallel-compile-profile.md) measures the implemented
workers on the retained allocator/schema-owner compiler, changing only worker
count zero/two. Eight complete fresh source-to-executable builds average
**16.182550 seconds serial versus 16.015580 seconds with two workers**:
**0.167 seconds saved (1.03%)**, with both blocks faster. All four outputs,
complete metadata and full executables are identical. Default remains serial.
The fixed runtime/adapter is cached; its first use is excluded. This is separate
from earlier program/compiler cohorts. Parsing increases 0.15%, catalog time
1.01% and handoff 9.46% (0.108 ms); all observations remain. Helper machine-code
compilation runs in workers; construction/cleanup and the other main stages
remain serial. The thirty-seventh packet includes all commands/logs, outputs,
native inputs, unchanged worker sources, two separate current-compiler stack
diagnostics and the setup filename failure before compiler execution. Fresh
extraction/verification and pure replay reproduce the compact result exactly.
No new compiler source change, runtime timing or sub-ten-second result is claimed.

The [shared action-output inventory qualification](../../../docs/design/native-action-output-inventory-compile-profile.md) retains a lazy,
complete sorted inventory for one immutable plan owner while each phase keeps
its original liveness, scope, type, definition and read checks in order. Eight
balanced complete builds improve **16.032216 to 15.689968 seconds (2.13%)**,
with both blocks faster. A separate Object cohort reduces preparation 20.48%,
compiler instructions 2.03% and compiler time 1.93%. RSS increases 0.0042% and
footprint 0.0061%; parsing, cleanup, source CFG and link regressions remain in
the report, including the first slightly slower complete pair. Every source,
bytecode, Object, complete metadata and full executable matches, together with
14 fixture pairs and a separate default-serial byte gate. Seven focused tests
and the real native CLI check pass. The first focused harness link fails on
an LLVM bitcode-version mismatch before tests execute; matching Rust ThinLTO
fixes setup without changing the frozen compiler patch. Source commit f077bec331
contains exactly the measured three-file diff. The thirty-eighth packet keeps
all commands/logs, sources/patch, tests/failure, observations/outputs/executables,
native inputs and pure replay. Its repository copy verifies and replays exactly.
No language or emitted-program change, additional parallel gain or sub-ten-second
result is claimed; the fixed runtime/adapter remains cached during primary runs.

The [once-queued reachability qualification](../../../docs/design/native-reachability-worklist-compile-profile.md) retains an immutable-CFG
traversal that marks each non-stop block before queuing it. The complete bitmap,
32-row FIFO memo and current proof/selection paths stay the same. Eight balanced
complete builds improve **15.694847 to 15.463704 seconds (1.47%)**, with both
blocks faster. A separate Object cohort reduces cleanup including compaction
5.45%, compiler time 1.39% and instructions 0.41%. RSS increases 0.0029% and
footprint 0.0027%; parsing, catalog, preparation, source CFG and link increases
remain in the report. Every source, bytecode, Object, complete metadata and
whole executable matches, together with 14 fixture pairs and a default-serial
byte gate. Three focused graph suites and the native CLI check pass. Both
saved sample caller addresses normalize to one successor-worklist append;
bounded disassembly disproves the earlier FIFO-shift hypothesis. Source commit
d26cf2c65e contains exactly the measured one-file patch. The thirty-ninth packet
keeps all commands/logs, sources/patch, tests, observations/outputs/executables,
native inputs and raw/demangled/address/disassembly selection evidence. Its
repository copy verifies and replays exactly. No language or emitted-program
change, additional parallel gain or sub-ten-second result is claimed.

The [current-source CPU-native PGO qualification](../../../docs/design/native-refreshed-pgo-compile-profile.md) retains the combined
compiler build configuration on source d26cf2c65e and the current larger program/
runtime. Eight balanced complete builds improve **15.620738 to 14.027481 seconds
(10.20%)**, with both blocks about 10.2% faster. The standard B72 control is reused;
three new compiler builds preserve warnings-as-errors, explicit target and the
compiler ThinLTO profile. Eight fixed training programs produce matching artifacts;
matching LLVM merges eight raw profiles into a 100,519,368-byte profile. The
profile-use compiler reports 18,617 functions without training data and no hash/
control-flow mismatches. Every main/fixture/training/held-out artifact, complete
metadata and full D6 executable matches. Fourteen fixture pairs and 56 balanced
held-out compiles pass; all seven held-out cases improve process time in both
blocks. The Object cohort lowers compiler time 9.93%, instructions 13.00% and
parsing 18.23%; encoding increases 14.27% and native link 5.78%. All observations
remain separate from earlier source/parallel/PGO cohorts. The extra default-serial
gate first uses an incorrect helper path and fails before compilation; the
corrected frozen configuration passes every artifact and executable byte check.
The selected local PGO tool is root/use (5a1687); the Cargo-managed standard tool
is preserved. The fortieth packet includes profiles, source configuration, all
commands/logs, outputs/executables, native inputs and pure replay. Its repository
copy verifies every manifest/file/Git blob and replays byte-identically. Gzip
level 6 preserves the exact tar stream while reducing transport size. No compiler
source or emitted-program change, component/additive gain or sub-ten-second
result is claimed; the fixed runtime/adapter is cached during primary runs.

The [dense-prefix SSA lookup qualification](../../../docs/design/native-dense-hole-lookups-compile-profile.md) retains source
931b63c878, whose entire one-file commit equals the frozen measured patch.
Eight complete builds improve **15.500449 to 15.421547 seconds (0.51%)**,
with both blocks faster and every executable byte identical. The standard
B72/dd7 compiler comparison uses the same 61 flags and two workers. Source
CFG construction improves 4.31% and instructions 0.47%; Object compiler time
improves 0.30% with mixed blocks, cleanup increases 1.11% and final linking
8.20%. All observations remain. Dense-prefix holes now prove absence without
a sparse-tail probe; storage, sealing, value allocation and proof paths are
unchanged. All 29 production-module units, 304 paired raw/compacted IR and
machine-code cases, 14 fixture pairs, native CLI check and default-serial byte
gate pass. Two separate current-PGO stack diagnostics, address/disassembly,
symbol bridge, all commands/logs, source/patch/whole-commit proof, outputs and
native inputs are in the forty-first packet. Fresh extraction verifies and
replays byte-identically. The new source is not combined-PGO qualified; the
separate 14.027481-second d26 result remains the best qualified configuration.
No grammar or emitted-program change or sub-ten-second result is claimed.

The [stable batch phi-removal trial](../../../docs/design/native-batch-phi-params-compile-trial.md) is rejected and the original
two production files and immutable standard compiler are restored. Eight
complete source-to-executable builds take **15.414733 versus 15.444185 seconds
(+0.19%)**, with mixed blocks. A separate Object cohort lowers compiler time
0.66%, cleanup including compaction 2.58% and instructions 0.82%; both compiler
blocks improve, but the complete-build result establishes no useful saving.
Parsing +0.28%, frontend +0.16%, preparation +0.13% and compaction +2.97% remain.
All artifacts, complete metadata, 14 fixture pairs and every whole executable
match. Three focused tests cover 2,047 removal masks, 20 wide/pool boundary
cases and 128 complete IR/alias/type/ordinal/machine pairs. The solver and
replacement/alias order are unchanged; only batch parameter bookkeeping is
tried. All checks pass. The forty-second packet preserves the rejected
source/patch, tests, commands/logs, all observations/outputs, native inputs,
selection diagnostics and restoration receipt. Fresh extraction verifies and
replays byte-identically. No source optimization is retained and no new PGO,
parallel or sub-ten-second saving is claimed; the separate best qualified PGO
result remains 14.027481 seconds.

The [imported-cleanup analysis-owner trial](../../../docs/design/native-imported-analysis-owner-compile-trial.md) is rejected and both
source files and the immutable standard compiler are restored. Eight complete
builds are effectively flat: **15.287110 to 15.281101 seconds (-0.04%)**, with
mixed blocks. Their compiler-process component increases 0.05%, while final
linking falls 10.58%. The separate Object cohort lowers compiler time 0.35%,
cleanup including compaction 1.01% and instructions 0.38%; source CFG +1.20%,
compaction +4.33%, RSS +0.0127% and all other movement remain. Every artifact,
complete metadata, 14 fixture pairs and full executable byte matches. Three
focused actual-Cranelift tests compare fresh graph/dominator/alias/IR state for
64 unchanged seeds, 16 successor-order/layout seeds and 16 graph-only alias
seeds. The candidate shares descriptive analyses across the audited five-pass
suffix, preserves all enable checks/pass order/verification, and invalidates
on every mutation including successor ordering. No useful complete-build gain
is established. The forty-third packet keeps source/patch, exact tests, audited
pass sources, metadata selection, every command/log/observation/output, native
inputs and restoration receipt. Fresh extraction verifies and replays exactly.
No source optimization is retained; the separate best qualified PGO result
remains 14.027481 seconds and the sub-ten-second goal stays open.

The [expanded numeric-phase worker qualification](../../../docs/design/native-parallel-phase-cleanup-compile-profile.md) retains source
0c949505cc, whose entire three-file commit matches the measured patch. Eligible
general phases now run owned cleanup and machine generation in the existing
bounded pool while the coordinator prepares the vector variant. A diagnostic
records 344 submitted jobs, 48 including cleanup. Proof identities, all Module
effects, ordered publication/error precedence and metadata finish stay on the
coordinator; no unsafe Send or grammar change is introduced. Eight standard
old2/new2 complete builds improve **15.398425 to 15.130134 seconds (1.74%)**,
with both blocks faster. Object compiler time falls 1.61% and helper generation
12.68%; instructions increase 0.0352% and all adverse stages remain. A separate
same-binary serial/2-worker cohort improves **15.553768 to 15.218166 seconds
(2.16%)**, also in both blocks. All four outputs, complete metadata, 14 fixture
pairs and all 20 full executables match, including default0 and excluded real-
job gates. Four worker suites pass 36 Object comparisons and error-order tests.
The forty-fourth packet keeps source/patch/whole-commit proof, all tests,
supporting source/token audit, commands/logs, every observation/output/executable
and native inputs. Fresh extraction verifies and replays byte-identically.
Default0 remains serial; two workers are opt-in. The new source is not combined-
PGO qualified; the separate 14.027481-second d26 PGO configuration remains best.
The sub-ten-second goal stays open; no percentages or cohorts are pooled.

## Fresh PGO for expanded workers: Deflate-trained

The fresh [PGO qualification](../../../docs/design/native-parallel-pgo-compile-profile.md) on source `0c949505cc` measures complete source-to-executable builds **15.241434 to 13.596783 seconds (10.79% faster)**. A separate same-PGO-binary serial/two-worker comparison measures **13.959954 to 13.557414 seconds (2.88% faster)**. Both preserve every executable byte; the experiments remain separate.

Deflate is included in training. All seven disjoint small heldouts improve, which does not establish broad gains on unrelated substantial programs. The standard compiler remains separately reported. The forty-fifth packet includes raw/merged profiles, every adverse field, all outputs and 20 authoritative executables, the interrupted/excluded wrapper attempt and the corrected stage audit. Fresh repository extraction verifies and replays exactly. No source/language/runtime changes or sub-ten claim.

The [SSA descending-chain lookup trial](../../../docs/design/native-ssa-descending-chain-compile-trial.md) is rejected. Eight complete builds increase **15.126993 to 15.195366 seconds (+0.45%)**, with both balanced blocks slower. The separate Object cohort increases compiler time 0.78%, source CFG construction 2.55% and instructions 0.0164%. All 32 focused tests, 328 paired raw/compacted IR and machine-code cases, 14 fixture pairs, complete metadata and every whole executable byte match. The original source and standard compiler are restored; selected Deflate-trained PGO remains 13.596783 seconds. Fresh current-PGO stack diagnostics selected the candidate and remain separate from primary timings. The forty-sixth packet keeps the rejected source/patch, exact tests/audits, profiles, all commands/logs/observations/outputs/executables, native inputs and restoration receipt. Fresh repository extraction verifies and replays byte-identically. No optimization is retained and the sub-ten-second goal stays open.

The [borrowed parser event-iterator trial](../../../docs/design/native-parser-event-cursor-compile-trial.md) is rejected. Parser time falls **1.869500 to 1.842458 seconds (1.45%)**, about 27 ms, with both parser blocks faster; complete builds are **15.215777 versus 15.229858 seconds (+0.09%)**, with mixed blocks. The separate Object compiler result is flat (-0.052%); source CFG +0.84%, frontend +0.29% and compaction +2.80% remain. All 526 production document units, 1,780 complete parse/identity/accounting pairs, 14 fixture pairs, complete metadata and every executable byte match. Original source and standard compiler are restored. The forty-seventh packet preserves full syntax snapshots, patch, tests/recipes/audits, all setup failures, commands/logs/observations/outputs/executables, native inputs and restoration. Failed standalone link and two incomplete qualification setups are excluded; the full authoritative qualification passes all six stages. Fresh repository extraction verifies and replays exactly. The separate Deflate-trained PGO result remains 13.596783 seconds; no useful new complete-build gain or sub-ten-second result is claimed.

The [fused dimension declaration scan](../../../docs/design/native-dimension-declaration-scan-compile-profile.md) is retained as source `ef5e255e81`, whose entire one-file commit equals the measured patch. Eight complete builds improve **15.120963 to 14.971381 seconds (0.99%)**, with both blocks faster and every executable byte identical. Frontend improves 11.72%, Object compiler time 1.28% and instructions 0.2344%, also in both blocks. Source CFG +2.12%, RSS +0.1367%, footprint +0.1376% and all other adverse fields remain. One fresh complete declaration scan replaces separate ID, constant-kind and lifetime scans; no cache, inventory pruning, first-error, grammar or emitted-program change. All 303 core tests, no_std, 24,576 new competing-error comparisons, 14 fixture pairs, complete metadata, default0 and real344-job/48-cleanup gates pass. The forty-eighth packet keeps source/patch/whole-commit proof, original reference audit, tests, commands/logs, every observation/output/executable and native inputs. Fresh repository extraction verifies and replays exactly. New source is not PGO-qualified; the separate 13.596783-second Deflate-trained PGO result remains on earlier source. The sub-ten-second goal stays open.

The fresh [current-source PGO qualification](../../../docs/design/native-dimension-pgo-compile-profile.md) measures complete Deflate compilation **14.981386 to 13.414825 seconds (-10.46%)** on source `ef5e255e81`, with both subjects using two workers. Every complete executable byte matches. The separate same-PGO-binary serial/two-worker cohort measures 13.632845 to 13.396915 seconds (-1.73%). These experiments remain separate from the earlier 14.971381-second standard result and 13.596783-second older-source PGO result. Deflate is included in the fixed eight-program training corpus; seven small heldouts do not establish broad generalization. The forty-ninth packet preserves all profiles, diagnostic/adverse fields, observations, complete outputs and 20 complete executables. Fresh extraction verifies and replays byte-identically. No production source, grammar or runtime changes; the sub-ten-second goal remains open.

The [packed dense SSA-slot trial](../../../docs/design/native-packed-ssa-slots-compile-trial.md) is rejected and restored. Slots shrink 8 to 5 bytes and peak RSS falls about 18.45 MB, but timing is mixed: complete builds improve **14.948667 to 14.891670 seconds (-0.381%)**, while the separate Object compiler cohort slows +0.400%, with both Object blocks slower. All 31 units, 328 paired raw/compacted IR and machine-code cases, 14 fixture pairs, complete metadata, default-serial/real-worker gates and all 12 complete executable images match. The fiftieth packet retains every result, adverse field and restoration proof; fresh extraction verifies/replays byte-identically. The best qualified time remains 13.414825 seconds; no new source or performance gain is retained.

The [parser continuation payload-storage trial](../../../docs/design/native-parser-frame-payloads-compile-trial.md) is rejected/restored. Frames shrink 80 to 32 bytes; the separate standard parser improves 5.134% but complete builds slow 0.936%. Direct current-fastest8043 versus candidate with own fresh PGO measures **13.275817 to 13.478050 seconds (+1.523%)**. All 528 units, 1,976 complete parser identity/accounting pairs, both sets of 14 fixtures, full metadata and 32 qualification executables match. The fifty-first packet retains all adverse data, selection profiles, fresh raw/merged PGO profiles and restoration proof; fresh extraction verifies/replays exactly. The unchanged compiler's original primary result is 13.414825s; it repeats 13.275817s here. PGO fully counts.

The [full-LTO compiler comparison](../../../docs/design/native-fat-lto-pgo-compile-profile.md) is **rejected**. Complete fresh builds measure **13.536528 to 13.329149 seconds (-1.532%)** against the fastest qualified ThinLTO compiler, with both using exact current source, native CPU and fresh PGO. Parser time is 1.521244 versus 1.539862 seconds. All full outputs, complete metadata and 20 executable images match. The fifty-second packet retains all diagnostics, adverse fields, fixed eight-program profiles, seven small heldouts and exact Rustc LTO invocations; fresh repository extraction verifies/replays byte-identically. The full-LTO configuration is rejected; the qualified ThinLTO compiler `8043` remains selected. The sub-ten-second goal stays open.

The [parser execution census and three independent trials](../../../docs/design/native-parser-execution-trials.md) record **35.0 million continuation frames, 13.6 million checkpoints and 9.1 million rewinds** for the current 164 KB source. Identifiers succeed 10.36 times per distinct successful start and digit sequences 41.23 times; source-view bounds still need separate checking. Standalone parser means improve 2.51%, 4.63% and 2.16%, each below the predeclared 10% promotion threshold. All 526/526/527 units and complete parse bytes pass; none is promoted or full-build qualified. The fifty-third packet retains every observation, failure, source snapshot and current-PGO-library sample, with fresh extraction and byte-identical replay. Selected8043 and its original 13.414825-second primary result remain unchanged; no grammar or language change.

The [bounded lexical-reuse trials](../../../docs/design/native-parser-lexical-reuse-trials.md) measure five parser revisions, including an adverse dispatch-only result. Compact/batched reuse plus completed cache-owner release improves standalone parsing **11.86%**; the formatted frozen candidate repeats **11.80%**, with both balanced blocks faster, **529 units** and all complete parse bytes exact. Both clear the original 10% screen; fresh own-PGO complete-build qualification is separate and pending. The fifty-fourth packet retains all 44 full parse outputs, exact-context events, actual cache hits/refusals, CPU samples, type layouts and failed setups. Selected8043 and the original **13.414825-second** primary remain unchanged; no grammar or language change.

The [compact-source complete-build qualification](../../../docs/design/native-compact-source-compile-profile.md) integrates #992 with the parser/compiler work. The supplied 2,719-line source compiles through final executable linking in **6.027732 seconds**, versus 6.379101 with the prior parser (**5.51% faster**, both blocks). It reproduces the exact 55,956-byte Object (0.970× Roc/1.361× libdeflate), complete metadata and every final executable. All 552 tests and 11 complete-stream checks pass. The supplied runtime is preserved; zero eligible worker jobs means no parallel speedup is claimed. The earlier 13.414825s uses a different source. The handoff reports 81 Roc / 62 libdeflate wins, with 63 a target. Packet55 retains all profiles, failures and earlier cohorts with fresh repository verification and byte-identical replay.
