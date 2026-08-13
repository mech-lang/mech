# Named compute regions

Mech programs may use an underlined Mechdown section as an explicit numeric
compilation boundary. The application remains one `.mec` document:

```mech
+> math

1. Input and UI
-------------------------------------------------------------------------------

@mouse := window://mouse{:read(position)}
mouse-position := @mouse/position

particle update @ compute
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
- `@ compute` creates a backend-neutral boundary;
- `@ cpu` requires the CPU executor;
- `@ gpu` requires a GPU executor.

An unannotated Mechdown section remains documentation structure and does not
create a compute boundary. It follows the ordinary CPU runtime path; no
`@ cpu` annotation is required. Unknown placement names are rejected during
parsing.

## Compiler contract

While interpreting source, the engine records the exact reactive-plan node
range produced by each named section. Compilation translates those plan-node
identities into semantic `ProgramArtifact` node identities. Variable-definition
markers that disappear during semantic lowering do not become region nodes.

Region metadata is part of bytecode v1 rather than process-local compiler
state. The `ArtifactComputeRegions` section stores each region's name,
placement, and sorted semantic node IDs. Decoding validates names, placement
tags, node bounds, ordering, deduplication, and non-overlap.

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

## Native mixed-host spike

`examples/gpu-particles/particles.mec` exercises a narrow end-to-end boundary:

1. the ordinary unannotated graph reads a real timer host;
2. the CLI extracts `particle-field @ gpu` from the same parsed document;
3. the GPU provider compiles that region and keeps its state resident through
   `wgpu`;
4. `@particles/turn <- tick` stages an after-commit GPU dispatch;
5. GPU completion telemetry returns as runtime host-input packets; and
6. the ordinary graph writes those values through the console host.

The spike uses the transactional legacy route for the CPU graph because D4's
resident finalizer does not yet exclude GPU-owned nodes when finalizing the CPU
artifact. It proves a real source, transaction, host, compiler, GPU, and ingress
path without claiming the general mixed-region scheduler is complete.
