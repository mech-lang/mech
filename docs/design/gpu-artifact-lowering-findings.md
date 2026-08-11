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

`VariableDefine*` instructions survive as `LegacyOpaque` artifact nodes even
though downstream registers already reference the values being named. The GPU
host can safely ignore these exact dead markers, but every portable executor
should not have to rediscover that rule.

Recommended change: either elide definition-only nodes while building the
artifact or give them a first-class structural/identity classification. Do not
model them as opaque computation.

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

Values inserted into the interpreter symbol table are captured as constants
unless a source declaration establishes an input cell. The example currently
uses typed host values followed by ordinary source declarations to obtain
correct `InputDeclaration`s.

Recommended change: D1 activation should accept explicit host input bindings
with schema and shape authority. Whether a value is a constant or live input
should not depend on incidental symbol-table provenance.

### 7. Semantic operation contracts need complete coverage

`math/add` already declared pure signal/full-write/no-alias behavior, while the
equivalent subtraction and multiplication factories were `LegacyOpaque`.
This branch gives `math/sub` and `math/mul` the same declared contract.

Admission should continue to fail closed for opaque nodes. Expanding GPU
coverage should first expand authoritative operation contracts, not add an
unsafe fallback.

### 8. Live host inputs must not be serialized as artifact values

The release particle benchmark compiles at 50,000 particles but fails at
75,000 and 100,000 with `ProgramArtifact section exceeds read limit`. The host
matrices are currently materialized while source is evaluated and then captured
inside the artifact. Compile time also grows with particle count: 50.5 ms at
1,000 particles, 159.7 ms at 10,000, 321.8 ms at 25,000, and 616.6 ms at
50,000 on the measured Apple M1.

Live ingress should contribute schema, shape, and binding identity to the
artifact, not its current payload. Runtime data belongs in activation input
buffers. This is both a correctness ceiling and a bytecode-size/design issue,
not a GPU-host optimization.

## Executor and physical-plan findings

### Logical composites need physical decomposition

The semantic result is one tuple of two matrices. WGSL exposes two storage
buffers. That decomposition belongs in a GPU activated plan, which should
retain the mapping from logical output path (`result.0`, `result.1`) to physical
buffer binding.

### Binding pressure must be planned

The proof graph initially needed eight storage bindings: six inputs and two
outputs. Requesting downlevel WebGPU limits allowed only four. The native path
now requests the exact supported limit from the adapter.

For general kernels, activation should pack scalar controls into a uniform or
parameter buffer and reuse compatible arenas. Admission must compare the
planned binding count against adapter limits and report a capability diagnostic
before pipeline creation.

### Resident execution is the next performance boundary

The current native proof creates a device, pipeline, buffers, dispatch, and
readback for one call. This proves generated-kernel correctness, not steady-state
performance. A production GPU executor must retain the device, pipeline,
bind group, device buffers, and staging buffers across turns. Only changed
inputs should be uploaded, and readback should occur only for host-observed
outputs.

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
