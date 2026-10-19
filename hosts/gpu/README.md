# Mech compute backends

`mech-gpu` provides the current elementwise product lowerer and the physical
CPU and wgpu implementations of Mech's v0.4 compute subsystem. The compiler
owns source parsing and region partitioning. `mech-compute` owns the typed
neutral program and backend contracts. `mech-runtime` owns transaction
ordering and host activation.

One Mech document may contain one executable region marked `@compute`, `@cpu`,
or `@gpu`. `ProgramCompiler` returns partitioned coordinator and compute
`ProgramArtifact`s plus detached typed initializers. The product integration
then calls `mech_gpu::lower_elementwise_compute_program` to construct the
neutral `ComputeProgram` accepted by a configured `compute` host. The host
resolves a compatible backend and keeps state resident behind `ComputeSession`:

```text
Mixed Mech source
  -> ProgramCompiler mixed semantic compilation
  -> coordinator ProgramArtifact + compute ProgramArtifact + typed initializers
  -> product-owned elementwise lowering
  -> mech-compute ComputeProgram
  -> ComputeBackendRegistry
  -> ComputeSession
  -> transactional publication and telemetry
```

The stable mixed-application backends are `cpu-scalar` and `wgpu`. Stable
product selectors are `auto`, `cpu`, `gpu`, `cpu-scalar`, and `wgpu`. The
browser and native products use the same registry and session interfaces with
platform-specific adapters. An exact backend request never falls back. `@cpu`
and `@gpu` are hard class requirements; `@compute` accepts any compatible
backend.

The backend library also contains `cpu-simd`, `cpu-jit`, and fixed-shape wgpu
implementations for tests and benchmarks. The shipping mixed-application
compiler currently emits the elementwise kernel form, not the fixed-shape form
those prototypes require, so they are not stable v0.4 application backends.

Fixed-shape `cpu-simd` sessions use component-major batch storage and interpret
each operation across a cache-sized tile of independent instances. The array
executor predecodes operands, reuses temporary columns, and fuses short
sum-of-products loops without changing the source equations. Input updates are
atomic, and a failed integrity check rejects the entire candidate batch.
`FixedShapeKernel::prepare_array_cpu` exposes this executor directly;
`prepare_array_cpu_with_tile` permits an explicit tile-size limit. The older
`prepare_simd_cpu` API is retained as a four-instance-at-a-time control.


The current product scope is one configured compute host, one executable
region, fixed-shape `f32` values, persistent state, declaration defaults, live
input updates, requested output readback, and integrity rejection before
publication. Multiple regions, dynamic shapes, cross-region values, multiple
devices, and cost-based placement are deliberately rejected or deferred.

Mixed applications are source products in v0.4. Compute-region metadata and
artifacts round-trip through bytecode v1, but one root `.mecb` cannot yet carry
and activate the coordinator artifact, compute artifact, typed interface,
detached initializers, and backend metadata as a compound application package.
Ordinary resident applications continue to support source and `.mecb` input.

## Layout and publication

Compute-interface tensors are canonical row-major values. Mech's fixed matrix
kernels use column-major storage, so fixed-shape backends convert at session
ingress and output readback rather than in the steady-state turn. Scalar values
remain distinct from `1 x 1` tensors, and every update is checked for kind,
element type, rank, dimensions, layout, and element count.

Fixed-shape integrity predicates are compiled from ordinary Mech operations.
A failed candidate is not published, the previous state remains live, and the
session retains only a fault count and latest named fault. The compute host
reports backend ID, completed turns, dispatch time, fault count, and last fault
through normal runtime ingress.

The browser wgpu bridge gives each dispatched command a monotonic identity.
Every command consumed by JavaScript is either acknowledged after
`GPUQueue.onSubmittedWorkDone` or rejected when upload, encoding, submission,
or completion fails. `completed turns` advances only for acknowledgements; a
rejection is returned to the next resident dispatch as an execution error. Adapter admission and
`requestDevice` use the same required storage-buffer and workgroup limits.

## Runtime configuration

The product host uses provider and scheme `compute`:

```text
compute://<instance>/kernel/input/<name>
compute://<instance>/kernel/turn
compute://<instance>/kernel/backend
compute://<instance>/kernel/turns
compute://<instance>/kernel/dispatch-ms
compute://<instance>/kernel/fault-count
compute://<instance>/kernel/last-fault
```

`mech run` and `mech serve` always use the ordinary resident runtime.
`--backend auto|cpu|gpu|cpu-scalar|wgpu` overrides configured stable product
selection; it does not select an alternate application executor. Experimental
backend IDs may exist in feature-enabled registries, but normal mixed source
does not lower to their required kernel form.

## Validation

```text
cargo test -p mech-compute --all-features --locked
cargo test -p mech-gpu --features native,jit --locked
cargo test -p mech-wasm --features browser_project,browser_compute --locked
cargo check -p mech --features compute_backends_native --locked
```

The end-to-end particle application is under `examples/gpu-particles`.
Historical prototype notes and performance data live under
`docs/research/compute` and the separate
[benchmark repository](https://github.com/mech-lang/benchmarks/tree/main/benchmarks/archive/compute);
benchmarks are not correctness gates.

The general f64, map, collection and resource-inference regressions now live in
this crate and use local fixtures:

```sh
cargo test -p mech-gpu --locked --features native-build \
  --test native_fused_regressions --test native_map_regressions \
  --test native_sequence_admission --test native_feature_admission
```

The root CLI's native source build uses `cargo build -p mech --release --locked
--features cranelift_build`. Function-application and matrix conformance checks
are documented in
[`examples/function-application-prototype`](../../examples/function-application-prototype/README.md).
Recorded workload timings, compiler snapshots and optimization ledgers remain
in the benchmark repository.

The `mech-native-*` test tools retain legacy independent source lowering; they
are not the canonical AOT route. The public `mech build --backend cranelift`
adapter now uses maintained canonical source compilation or explicitly
source-unpaired `.mecb` decoding, then the same checked Object generator and
capability-secured resource-hosted shell. It does not fall back to the legacy
frontend or resident evaluator when lowering rejects a program. The default
`--backend resident`, `mech run`, and `mech serve` remain resident products.
Full-input execution, broader native coverage and benchmark qualification are
still unfinished; the legacy tools are not a supported second semantic
authority. See the recorded
[`follow_up_roadmap`](../../docs/validation/checked-aot-workload-progress-v1.json)
for migration context; current implemented and pending scope is detailed below.

Resource-hosted checked-native execution has one phased entry route. After
the runtime admits every original requirement and provider contract,
`prepare_hosted` executes the generated closed activation prefix and returns an
owning session before any provider read. The runtime then captures observations;
`resume_hosted_into` consumes that same session and transaction to run the fresh
turn with the original arena, finite limits and cumulative work account. Both
phases enter the same checked JIT or pinned Object CFG. The checkpoint is not
another evaluator, resident instance or permission source: the runtime alone
owns capture, effect staging and commit, and raw pure evaluation refuses
resource-bearing artifacts. Unsupported programs remain rejected. This entry
contract is not a claim that whole-benchmark qualification is complete.

The [canonical k-nucleotide source](tests/fixtures/native/knucleotide-idiomatic.mec)
expresses record selection, overlapping String windows, frequency widths and
queries in Mech, using comprehensions and general-purpose library primitives
such as `map/histogram`. Keep that algorithm in the language: packing, streaming,
fusion and parallel execution should be compiler optimizations over its canonical
semantics, not a benchmark-sized external helper. The checked native backend
now lowers scalar/String matrix comprehensions and shape-preserving unary
pattern-function lifting to generated loops, including filters and nested
generators. It uses the same artifact, current-visit proof, scalar emitter and
invocation budgets; collection helpers project elements and stage results,
never interpret body operations. Maintained numeric ranges, String grapheme
indexing/slicing, shallow matrix conversion and closed pure initialization
have historical standalone receipts for their recorded source identities;
those receipts do not qualify the current complete hosted application. The
current managed lowerer additionally supports selected shallow matrix/map
access, canonical matrix-to-Set conversion and membership, shallow
composite/enum values, String equality/concatenation, maintained sorting and sums,
and tail-recursive
FSM control. The complete k-nucleotide source uses these language-level
operations, not a workload-sized Rust helper.

Each native invocation still measures its current payloads and funds the same
maintained kernels, fresh memory plans and atomic publication. Its bounded
per-site binding cache exists only for structurally repeatable loop-body sites.
Canonical validation contracts nested loops and proves the remaining root CFG
acyclic; the consuming activation/turn session continues that same path without
replaying its prefix. Root-scope calls, including nested Match arms, therefore
use the same maintained uncached leaf directly and retain no bound/planner cache.
The full concrete once/repeated slot lane is still admitted before creation.
Repeatable sites' binding cache holds only one exact geometry at a time, never arguments,
results or provider permissions. A shape miss admits replacement work and finite
policy before retiring the old prepaid binding, then acquires its replacement
reservation before allocation; old/new binding allocations do not coexist.
Every hit and miss uses the same cumulative ledger and finite policy. The
same guarded binding may reuse immutable planning metadata only after matching
the complete fresh payload facts. Snapshot String selection may additionally
retain weak-storage-identity grapheme-index metadata, never the source payload.
Compact byte ordinals are used only after canonical grapheme segmentation
proves every grapheme has one byte, so CRLF and Unicode keep their usual meaning.
Index construction and eventual retirement are prepaid; its independently
reserved bytes and nodes remain part of every finite call admission. The
maintained selector still reads the actual current String and selector and
publishes through the same atomic output boundary. The
managed Object pins include the ABI, typed-IR and work-policy identities;
old work-policy receipts
must not be presented as qualification of changed accounting. Scalar/control
legacy identities remain separate and unchanged.

Implemented lowering and secured JIT correctness are not standalone execution
or full-input performance qualification by themselves. Clean source
`f176277d87feeae4a66167d7e11eb2b731acc2a0` passed actual secured Object-linked
execution of the complete original/decoded k-nucleotide source on small fixtures;
see [the exact-source receipt](../../docs/validation/canonical-hosted-native-linked-v1.json).
This does not qualify standard-size input, timings or the production native-build
CLI. The secured gate is
`canonical_hosted_artifact`'s ignored test
`linked::complete_original_and_decoded_knucleotide_linked_processes_are_secured_and_pinned`.
It builds and runs both complete original/decoded canonical artifacts with
ordinary CLI providers and grants, checks exact small-fixture and empty output,
denials, post-commit delivery failure, generated-input mutation rejection and
the actual runtime-only dependency closure. By default it records no timing samples.
Object emission alone does not pass this gate.

The qualifier requires an exact committed clean checkout, the pinned toolchain
and offline dependencies, no ambient Cargo configuration, and an absolute
evidence directory that does not already exist. Keep generated projects, build
outputs and evidence outside the checkout; a fresh child of a temporary
directory works. Run it serially after correctness gates, or use the required
`canonical-hosted` job in the existing native-plan workflow:

```sh
MECH_CANONICAL_HOSTED_SOURCE_COMMIT="$(git rev-parse HEAD)" \
MECH_CANONICAL_HOSTED_EVIDENCE_DIR="$(mktemp -d)/hosted" \
CARGO_TARGET_DIR="$(mktemp -d)/target" \
CARGO_BUILD_JOBS=1 \
cargo +nightly-2026-03-03 test --locked --offline -p mech-stdlib -p mech-gpu \
  --no-default-features \
  --features mech-stdlib/full_compiler,mech-stdlib/native-artifact,mech-gpu/canonical-native-program-host \
  --test canonical_hosted_artifact \
  linked::complete_original_and_decoded_knucleotide_linked_processes_are_secured_and_pinned \
  -- --ignored --exact --test-threads=1 --nocapture
```

These remain fresh stateless invocations, not persistent reactive applications.
Set `MECH_CANONICAL_HOSTED_PROFILE=release` to qualify the release Rust shell and
library code. The default is `debug`; other values are rejected. Both original
and decoded builds, including pin-mutation rejection and restored builds, use
the chosen profile. The receipt records the actual Cargo compiler profiles.
The checked Cranelift Object still uses `opt_level=none`; a release shell is not
a claim of optimized whole-program Mech code, full-size correctness or timing.

The explicitly opt-in `canonical-native-benchmark-policy` generator feature
adds `generate_linked_benchmark_program_application`, not another evaluator or
production CLI option. Its runtime-only executable requires a launch JSON
`benchmark_policy` of exactly `secured` or `trusted` and runs the same checked
Object, phased session, CLI providers, finite budgets and lifecycle in either
mode. Trusted mode bypasses capability-specific authorization decisions using
real live, in-scope, unlimited-use grant identities; subjects, revocation,
transaction scope and finite-use checks remain enforced. The experiment is not
a measurement of all safety overhead and is not safe to ship as a secured
production binary. No environment variable enables it in the production shell.
The required `canonical-policy` CI job tests actual original/decoded
same-binary controls and separately checks that the production binary rejects
both the benchmark JSON field and the legacy trusted environment bypass.
These controls alone are not full-input correctness or timing results.

That benchmark profile can use the same full-input or staged configuration
below, with a base launch containing the unchanged two grants and finite caps
but **no** `benchmark_policy` field. The driver freezes the common input,
oracle and base policy, then derives two immutable launch files differing only
in that field. Both original/decoded binaries must pass both modes' exact output,
report and native-accounting checks before any samples start. Timing alternates
policy order across rounds and reports each variant/policy separately. The
production entry still runs secured-only; it cannot select the comparison.
The required comparison CI uses release binaries and a four-process, zero-sample
small-fixture driver control, not a full-size benchmark or CI timing run.

The optional full-input phase uses those same two release executables after
all builds and pin-mutation/restoration checks finish. Add
`MECH_CANONICAL_HOSTED_PROFILE=release` and
`MECH_CANONICAL_HOSTED_FULL_INPUT_CONFIG=/absolute/full-input.json` to the
command above. With the configuration variable unset, there are no extra runs.
The configuration has this shape; replace every path and expected SHA256:

```json
{
  "schema": "mech.canonical-hosted-full-input-config.v1",
  "input": {"path": "/absolute/input.fasta", "sha256": "<64-lowercase-hex>"},
  "oracle": {"path": "/absolute/expected.stdout", "sha256": "<64-lowercase-hex>"},
  "launch": {"path": "/absolute/full.launch.json", "sha256": "<64-lowercase-hex>"},
  "deadline_seconds": 3600,
  "samples": 0
}
```

Paths must be absolute; hashes pin the exact bytes, not a benchmark-size label.
Use a separately declared finite scale launch policy: the same grants and
`max_ports` as the small gate, positive finite caps in every dimension, and
input/output acquisition caps covering the actual input/oracle bytes. The
driver does not raise small-gate caps or promise that the scale policy suffices.
It freezes byte-identical owned input/oracle/launch files and checks exact
stdout, empty stderr, route/revision/launch pins and completed shutdown for
both original and decoded programs. Actual denials are saved before failure.

`samples: 0` performs correctness only. For 1–20 samples per variant, also set
`MECH_CANONICAL_HOSTED_TIMING_SERIAL=1` and ensure no other build/test workload
overlaps timing. Both full correctness runs must pass first. Each process has
the explicit 1–3600-second deadline. Results live in
`full-input/full-input-result.json` inside the evidence directory, including
attempt/success counts if a later run fails.

To qualify several explicitly pinned sizes with the **same two already-built
release executables**, leave the single-config variable unset and instead set
`MECH_CANONICAL_HOSTED_STAGED_INPUT_CONFIG=/absolute/stages.json`:

```json
{
  "schema": "mech.canonical-hosted-staged-input-config.v1",
  "stages": [
    {"name": "medium", "configuration": {"path": "/absolute/medium.json", "sha256": "<64-lowercase-hex>"}},
    {"name": "large", "configuration": {"path": "/absolute/large.json", "sha256": "<64-lowercase-hex>"}},
    {"name": "full", "configuration": {"path": "/absolute/full.json", "sha256": "<64-lowercase-hex>"}}
  ]
}
```

Each referenced file uses the existing full-input configuration above, including
its own pinned input/oracle/finite launch, deadline and sample count. There must
be 1–3 stages with unique 1–32-character lowercase ASCII names (letters, digits,
`-`, `_`) and distinct absolute config paths, including resolved symlink targets.
All config bytes are pinned and parsed before the first process. Builds and pin
controls run once, not once per size. The two configuration variables are
mutually exclusive. Any nonzero sample count still requires
`MECH_CANONICAL_HOSTED_TIMING_SERIAL=1` and no overlapping build/test workload.

Stages run serially and stop on the first failed process, timeout or preflight
check; the driver never retries with larger caps. Both variants must pass that
stage's exact oracle qualification before its timing starts. Per-stage results
are saved under `staged-input/<name>/full-input-result.json`; aggregate
`staged-input/staged-input-result.json` retains actual attempts, confirmed/unknown
starts, successes and skipped stages even on partial failure. Earlier successful
samples do not make a partially failed staged run qualified. With neither
configuration variable set, no additional runs occur.

Integrity hashing and correctness runs deliberately warm filesystem caches.
Samples measure serial warm-cache, fresh-process wall time including CLI I/O
and report publication, excluding builds, hashing and parent validation—not
cold-cache or kernel time. This facility alone is not a full-size correctness
or timing result.

Persistent state, broader matrix/map/recursive operations, element-wise counts,
all-ten-program coverage, standard-size input and whole-program optimization
remain separate work. Release comparisons must follow exact-source
correctness/admission qualification and identify the source, input, execution
route and finite policy. The production generated shell is secured-only; the legacy
trusted-mode environment setting does not change its grants. No timings of
this language-level target are qualified yet. Older `sequence/packed-windows`
timings are separately scoped migration diagnostics, not completion of this
target.
