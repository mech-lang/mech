# GPU artifact lowering findings

Status: implementation findings from the first source-to-`ProgramArtifact`-to-WGSL slice.

This report separates limitations of the initial GPU host from issues that may
require changes to bytecode v1, `ProgramArtifact`, activation, executor, or
value design. Bytecode v1 is allowed to evolve before Mech 1.0.

## Conclusions

- `ProgramArtifact` is the correct semantic admission boundary. The GPU host
  did not need D1 activation internals.
- Existing schema bodies and compile-time matrix dimensions are sufficient for
  fixed-shape `f32` kernel admission. No value or shape redesign is indicated
  by this slice.
- GPU-specific physical layout belongs after `ProgramArtifact`, alongside
  activation planning. Logical tuple output became two storage buffers without
  changing the semantic artifact.
- The artifact must preserve canonical operation semantics independently from
  the selected runtime factory. This is the largest unresolved issue found.

## Findings that should affect D1 or bytecode work

### 1. Preserve canonical operation identity

Arithmetic nodes currently reach `ProgramArtifact` as specialized runtime
factory names such as `runtime/MulMDS<f32>`, not `math/mul`. A portable host
therefore has to recognize implementation-specific name prefixes.

Recommended change: carry both identities through compiler metadata and
bytecode v1:

- semantic operation: `math/mul`
- selected implementation: `MulMDS<f32>`

`ProgramArtifact::NodeDeclaration.operation` should be semantic authority.
Runtime/native linkage metadata should select the implementation separately.
The GPU prefix mapping in this branch is an explicit temporary compatibility
adapter, not the desired contract.

### 2. Return materialization must have instruction roles

`compile_bytecode` resolved the return value after closing every plan node. A
direct tuple return emitted `CompositePack` at that point, leaving the
instruction without `CompiledInstructionRole`; artifact compilation then
failed with `MissingInstructionRole`.

This branch fixes the issue by materializing the return inside a combinational
source-node boundary. This behavior should remain covered when D1 changes the
source-to-artifact path.

### 3. Named computed composites can lose dependency edges

With `output := (next-positions, next-velocities)`, the current compiler
represented `output` as a new artifact input snapshot. The artifact no longer
connected that logical output to the two arithmetic results. Returning the
tuple expression directly emits `core/composite-pack` and preserves the graph.

Recommended change: variable definition/alias normalization must preserve the
producer of a computed value. A named computed output must not become an
independent input solely because source syntax used `:=`.

### 4. Elide or classify variable-definition markers

`VariableDefine*` instructions previously survived as `LegacyOpaque` artifact
nodes even though downstream registers already referenced the values being
named. This branch now elides the markers during semantic artifact construction
while retaining their executable bytecode and symbol metadata.

This also allows dead evaluated snapshots used only by those markers to be
removed from the artifact constant store. Definition-only markers should remain
absent from future artifact designs rather than modeled as opaque computation.

### 5. Typed `f32` literal conversion is outside the runtime catalog closure

Source such as `1.0<f32>` creates
`ConvertScalarToScalarBasic<f64,f32>`. Its compiler-emitted runtime identity is
not installed in the source catalog, so `compile_program_product` fails with
`UnknownRuntimeFunction` before GPU admission.

The particle example avoids this by receiving native `f32` values from its
host, which is also the intended large-particle ingress path. The compiler bug
still needs a general fix: every compiler-emitted executable identity must be
in the artifact/catalog closure, or constant conversions must be folded before
runtime operation resolution.

### 6. Input authority needs an explicit host-ingress contract

Values inserted into the interpreter symbol table are otherwise captured as
constants. This branch adds an explicit compilation input set: the configured
host names its inputs, and matching immutable source declarations provide their
schemas while becoming `InputDeclaration`s instead of serialized constants.
Missing, ambiguous, computed, or mutable matches fail compilation.

This removes the name-matching magic from execution, but the API is still a
prototype compiler option. The durable design should make external input
bindings, schema authority, and initializer policy first-class build or
activation metadata. Whether a value is a constant or live input must not
depend on incidental symbol-table provenance.

### 7. Semantic operation contracts need complete coverage

`math/add` already declared pure signal/full-write/no-alias behavior, while the
equivalent subtraction and multiplication factories were `LegacyOpaque`.
This branch gives `math/sub` and `math/mul` the same declared contract.

Admission should continue to fail closed for opaque nodes. Expanding GPU
coverage should first expand authoritative operation contracts, not add an
unsafe fallback.

### 8. Live host inputs must not be serialized as artifact values

The initial release particle benchmark compiled at 50,000 particles but failed
at 75,000 and 100,000 with `ProgramArtifact section exceeds read limit`.
Compiler-generated definition markers retained evaluated matrix snapshots even
though no semantic node consumed them. Artifact construction now decodes
compiler-owned constants without a bytecode file round trip, elides those
markers, and compacts constants to the IDs actually referenced by the semantic
graph. The two-million-particle artifact has no constants and compiles without
raising bytecode read limits.

Large live ingress is still evaluated by the legacy interpreter before artifact
construction. That makes artifact plus WGSL compilation take 20.47 seconds at
two million particles on the measured Apple M1. Live ingress should contribute
schema, shape, and binding identity directly to compilation; payload allocation
and initialization belong in activation input buffers. This remaining issue is
compile-time work, not resident GPU execution.

The browser proof makes the same limitation visible at the API boundary. Its
compiler export currently allocates two zero-filled `2 x 2,000,000` matrices
solely to establish input schemas and state initializers before producing the
GPU manifest. The generated program and resident execution are representative,
but this compile-time payload construction is not a desirable bytecode or
executor contract. A shape-only external initializer would remove the browser
pause and avoid transient copies without changing Mech source semantics.

## Executor and physical-plan findings

### Large state initialization must be parametric and deferred

The managed CPU/GPU example now defines its particle state entirely in Mech,
rather than receiving precomputed position and velocity matrices from the
benchmark host. That exposed the remaining representation problem directly:
source compilation eagerly evaluates the particle range and initialization
expressions before constructing the artifact. At 250,000 particles the
artifact exceeds a section read limit; at two million particles bytecode
finalization exceeds the file read limit. Neither configured executor has run
at that point.

Raising those limits is not the intended fix. The artifact needs a shape
parameter and a deferred initializer region:

- `particle-count` determines state schemas and launch extents;
- an initializer graph computes initial state once during activation;
- the chosen executor allocates and initializes resident buffers;
- the recurring region consumes those buffers on each pulse;
- externally supplied matrices use bindings and uploads, not serialized
  payload constants.

Build-time shape specialization is enough for the first implementation. Shape
polymorphism and a specialization cache can follow. `InitializerReference`
therefore needs a computed-plan form with dependencies and shape parameters;
its constant-only form cannot represent this program at useful scale. This is
tracked in [issue #753](https://github.com/mech-lang/mech/issues/753).

### Managed mixed-runtime benchmark

`mixed_runtime_benchmark` executes the exact regular CPU graph in
`examples/mixed-cpu-gpu-particles/app.mec` and the exact configured kernel in
`kernel.mec`. It specializes only the `particle-count` declaration. Each pulse
enters through a runtime input driver, advances and commits the CPU graph, and
delivers the configured kernel effect. GPU results synchronize once after the
measured batch and perform no particle readback. The benchmark rejects a lane
unless its compiled element count and completed dispatch count match the
request.

On an Apple M1 using Metal, three 100,000-particle runs with five warmup and
100 measured turns produced these completed steady-state rates:

| Executor | ms/turn | Million particle-turns/s |
| --- | ---: | ---: |
| Fused CPU, median | 29.236 | 3.420 |
| Metal, median | 0.111 | 902.778 |

The median completed-throughput speedup is 264x. This is a batched throughput
measurement, not per-turn GPU latency: the CPU executor completes every turn
synchronously, while Metal queues the measured turns and the final
synchronization accounts for their completion. The first synchronized pulse
was approximately 2 ms in two of the three runs. At 4,096 particles, the full
managed path measured 1.246 ms/turn on CPU and 0.112 ms/turn on Metal, an 11.1x
throughput speedup.

The direct executor correctness benchmark remains separate. At 4,096 particles
and 100 resident turns, CPU/GPU one-turn maximum absolute error was 5.96e-8 and
the sampled resident recurrence error was 1.788e-7.

### Logical composites need physical decomposition

The semantic result is one tuple of two matrices. WGSL exposes two storage
buffers. That decomposition belongs in a GPU activated plan, which should
retain the mapping from logical output path (`result.0`, `result.1`) to physical
buffer binding.

The browser host now consumes this mapping through public binding roles and
cell-slot identities. This is enough for it to allocate ping-pong state buffers
and bind the logical position and velocity outputs to a separate render
pipeline; it does not inspect generated WGSL or rely on fixed binding numbers.

### Binding pressure must be planned

The proof graph initially needed eight storage bindings: six inputs and two
outputs. Requesting downlevel WebGPU limits allowed only four. The native path
now requests the exact supported limit from the adapter.

For general kernels, activation should pack scalar controls into a uniform or
parameter buffer and reuse compatible arenas. Admission must compare the
planned binding count against adapter limits and report a capability diagnostic
before pipeline creation.

### Resident execution removes per-turn setup and observation

The native proof now includes a resident session that retains the device,
pipeline, bind groups, and two particle state-buffer sets. A host-provided
feedback map connects `result.0` to `positions` and `result.1` to `velocities`.
Turns alternate bind groups without host copies; output staging and mapping
occur only when the host requests readback.

On the measured Apple M1, two million particles ran 120 turns at 1.703 ms per
turn (1.174 billion particle-turns per second). One final full readback of both
matrices took 15.852 ms. CPU/GPU one-turn output matched exactly and sampled
120-turn recurrence error was 5.96e-8.

This is executor machinery, not bytecode machinery. D1's
`ProgramArtifact -> ActivatedPlan -> ReactiveInstance` separation is compatible
with a resident GPU plan.

## Initial host capability set

The first host admits only:

- scalar `f32` and fixed-size `f32` matrices;
- pure signal nodes;
- full-write, no-alias outputs;
- element-wise `math/add`, `math/sub`, and `math/mul`;
- scalar broadcasting;
- structural tuple packing at the logical output.

It rejects state, dynamic shapes, effects, integrity constraints, opaque
compute contracts, matrix constants, and unknown operations with structured,
node-specific diagnostics. Those rejections are capability limits, not evidence
that the semantic model must change.
