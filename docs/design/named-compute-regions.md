# Named compute regions

Mech programs may use an underlined Mechdown section as an explicit numeric
compilation boundary. The application remains one `.mec` document:

```mech
+> math

1. Input and UI
-------------------------------------------------------------------------------

@mouse := window://mouse{:read(position)}
mouse-position := @mouse/position

particle update @compute
-------------------------------------------------------------------------------

offset := positions - mouse-position
acceleration := (0f32 - offset) * attraction
next-velocities := velocities + acceleration * dt
next-positions := positions + next-velocities * dt

positions = next-positions
velocities = next-velocities

2. Display
-------------------------------------------------------------------------------

@window/particles <- positions
```

The heading is structured source metadata:

- `particle update` is the stable region name;
- `@compute` creates a backend-neutral boundary;
- `@cpu` requires the CPU executor;
- `@gpu` requires a GPU executor.

An unannotated Mechdown section remains documentation structure and does not
create a compute boundary. It follows the ordinary CPU runtime path; no
`@cpu` annotation is required. Unknown annotations are rejected during
semantic compilation.

## Compiler contract

While interpreting source, the engine records the exact reactive-plan node
range produced by each named section. Compilation translates those plan-node
identities into semantic `ProgramArtifact` node identities. Variable-definition
markers that disappear during semantic lowering do not become region nodes.

Region metadata is part of bytecode v1 rather than process-local compiler
state. The `ArtifactComputeRegions` section stores each region's name,
placement, and sorted semantic node IDs. Decoding validates names, placement
tags, node bounds, ordering, deduplication, and non-overlap.

That round trip applies to compute-region artifact metadata. A mixed
application is a compound product containing the ordinary coordinator,
compute-region artifact, typed interface, detached initializers, and placement
metadata. v0.4 loads that product from source. It does not yet package or
activate the compound product from one root `.mecb` file. Ordinary resident
applications remain loadable from source or bytecode.

The executor derives live inputs, outputs, residence, and transfer boundaries
from graph dependencies. Mech source never contains buffer handles, pointers,
bind groups, or explicit upload/readback operations.

## Placement behavior

`compute` means the configured executor may choose CPU or GPU. `cpu` and `gpu`
are hard requirements; selecting an incompatible executor is an error rather
than an implicit fallback.

The current GPU provider can lower one named GPU region into one fused WGSL
program. Placement planning supports multiple named regions and reports their
boundaries, but executing several CPU/GPU regions in one reactive turn still
requires the mixed multi-kernel scheduler. The provider rejects that case
instead of fusing across explicit boundaries.

The stable v0.4 product backends are `cpu-scalar` and `wgpu`. `auto`, `cpu`,
`gpu`, `cpu-scalar`, and `wgpu` are stable application selectors. Native
`mech run` also accepts `cpu-simd` and `cpu-jit`; those requests select the
compiler's fixed-shape kernel emission before backend resolution and fail
explicitly when the region cannot be represented by that form. Browser
`serve` keeps the stable selector set. Automatic dual-form planning remains a
separate optimization: `auto` still emits the portable elementwise form first.

## Mixed runtime integration

`examples/gpu-particles/particles.mec` exercises a narrow end-to-end boundary:

1. the ordinary unannotated graph reads the configured pointer host;
2. `ProgramCompiler` performs mixed semantic compilation, partitions the
   selected named compute section, and preserves its placement and dependency
   metadata in immutable `ProgramArtifact`s;
3. the v0.4 runtime compiler returns an activation-only artifact product, so
   browser and native hosts do not retain a duplicate bytecode container;
4. the product integration selects a lowering from the compiler-owned compute
   artifact: `lower_elementwise_compute_program` for the portable path, or
   `ComputeLowerer::compile_broadcast` for an explicitly requested native
   fixed-shape backend;
5. `mech-compute` owns the resulting typed interface, neutral compute IR,
   placement contracts, and backend registry contracts;
6. the concrete backend bundle implements CPU and wgpu compilation and keeps
   compute state resident behind a `ComputeSession`;
7. `ComputeHostFactory` adapts that session to the resident runtime, while
   configuration grants define live inputs independently from detached values
   used to establish types and shapes during planning;
8. committed writes to `compute://particles/kernel` dispatch only after the
   ordinary CPU transaction commits; and
9. pointer input, compute telemetry, and rendering remain host-owned I/O rather
   than handwritten simulation code.

Concrete backends do not own compiler planning state or public `LegacyValue`s.
Short-lived compiler planning and source partitioning are confined behind
`ProgramCompiler`; only immutable artifact products and detached typed
initialization values cross into product lowering. The current elementwise
lowerer is in the concrete backend bundle, `mech-compute` defines its neutral
output contract, concrete backends own physical CPU/wgpu execution, and the
resident runtime retains transaction ordering, capabilities, effects, and
publication.

This remains section-level source projection, not yet general dependency-graph
partitioning. Live host-dependent CPU regions also require complete resident
operation coverage; static source initialization is not a substitute for live
resident execution. Cross-region values, several GPU regions, cost-based
automatic placement, and activation-time state initializers still require a
compiler partition plan and a multi-region scheduler.
