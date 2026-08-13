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
create a compute boundary. Unknown placement names are rejected during parsing.

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
