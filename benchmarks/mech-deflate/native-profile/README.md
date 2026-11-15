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
