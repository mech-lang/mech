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

The v0.4 runtime compiler now accepts detached typed planning values separately
from an explicit set of live input names. Planning values establish types and
shapes; they do not silently become ports. In the mixed browser project, the
configured `compute://` write grants are the authority that selects live input
declarations. The compiler returns their declaration-time values as detached
snapshots and drops all compiler cells before activation.

This is sufficient for the one-region proof. A general activation plan should
carry the same input schema, shape, and capability authority directly rather
than reconstructing it in each compute host.

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

The runtime compiler no longer exposes or retains a legacy program to do this.
It creates one short-lived planning graph, captures explicitly live input
initializers as detached typed values, finalizes the immutable artifact, and
drops the graph. Artifact construction also avoids a bytecode serialization
round trip.

The remaining browser startup cost is the eager evaluation of the particle
program's source-defined million-element state initializers. That work is
reported once as source-to-artifact compilation and is separate from resident
turn throughput. A shape-aware or GPU-side initializer plan would remove the
pause and transient CPU buffers without changing Mech source semantics.

### 9. Snapshot and executor matrix layouts require an explicit boundary

Mech runtime matrices and numeric kernels use column-major component order.
Canonical `ProgramArtifact` matrix snapshots are serialized in row-major order.
The generic fixed-matrix lowerer initially consumed snapshot elements as though
they were already runtime buffers. A rectangular projection matrix therefore
lost its second row, and every accelerated EKF backend agreed on the same wrong
measurement update.

The fixed-matrix executor now converts constants and state initializers to
column-major order once during admission. Source evaluation, scalar lowering,
SIMD, Cranelift, and native GPU results are compared independently, and a
rectangular matrix regression prevents symmetric covariance matrices from
hiding future layout errors. No conversion occurs in the steady-state turn
loop. The element-wise particle executor deliberately retains
artifact-linearized row-major buffers because it performs no matrix algebra.
Detached runtime matrix input initializers cross the inverse boundary once
when the browser compute host creates artifact-linearized GPU input buffers.

Recommended follow-up: activated physical plans should describe matrix layout
per buffer. Cross-region transfers must not infer layout from the selected
backend or from an untyped `Vec<f32>`.

## Executor and physical-plan findings

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

## Generic fixed-shape batch follow-up

The parallel EKF proof extends the experiment without introducing an EKF
operation. Its source compiles into 108 ordinary typed artifact nodes. A
backend scalarization pass expands fixed-size matrix multiply, transpose, dot,
matrix construction, scalar broadcasting, arithmetic, and trigonometry into a
single invocation-local register program. One invocation executes one complete
filter, and the physical executor maps that program over an independent outer
batch.

An independent test evaluates the high-level source in a short-lived compiler
planning session, executes one turn through the scalarized CPU IR, and compares
the state vector and covariance matrix. The native benchmark then compares
that same scalar IR with generated WGSL over four turns. No operation name in
the artifact or WGSL contains `ekf`.

This follow-up exposed four additional design requirements:

1. **Fixed-shape scalarization belongs after semantic compilation.** Matrix
   source should remain matrix source. Expanding a 3x3 product is a backend
   choice informed by shapes and target cost, not a language rewrite or a
   special EKF intrinsic.
2. **The outer parallel axis is an activation property.** The follow-up spike
   now receives actual arrays produced by the ordinary section of the same
   Mech document. `compile_broadcast` derives one common extent
   from artifact inner shapes and activation lengths; singleton inputs
   broadcast, while missing, fractional, zero, and conflicting extents fail
   admission. No compiler API receives a filter count. A compact artifact form
   for literal user-function broadcast remains future compiler work.
3. **Canonical operation identity and contracts remain blockers.** Several
   typed concatenate, dot, and trigonometric nodes still carry legacy opaque
   contracts. The prototype admits an audited set of exact runtime operation
   families and validates their schemas and arity. Production admission should
   use declared semantic contracts, not runtime factory-name compatibility.
4. **Submission policy is part of the physical plan.** On the measured Apple
   M1, the source-driven 100,000-filter run reached 61.060 million
   EKF-turns/s with one command submission per turn and a median 329.650
   million when 120 dependent turns were recorded in one submission. The
   scheduler needs an explicit way to batch or retain device work while
   preserving observation and effect boundaries.
5. **The broadcast axis is also a viable CPU vector axis.** A four-lane
   `f32` executor now evaluates the exact same scalarized instruction stream
   and reaches 4.416 million EKF-turns/s versus 1.222 million for the scalar
   evaluator. This 3.61x speedup requires neither source changes nor an
   EKF-specific kernel. A future CPU backend can combine wider vectors, native
   code generation, and multicore partitioning independently.

The generic CPU number reported by this proof is a scalar IR evaluator. It is
not evidence about retained Mech, AOT code generation, SIMD, or raw Rust. A
future CPU backend can consume the same scalarized region and choose native
code generation or fixed-size vectorization independently from GPU placement.
