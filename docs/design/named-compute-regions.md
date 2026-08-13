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
2. the mixed executor projects unannotated/`@ cpu` sections into a parsed CPU
   program and `@ gpu`/selected `@ compute` sections into GPU programs;
3. D4 activates the CPU projection as a resident external program;
4. the GPU provider compiles the selected region and keeps its state resident through
   `wgpu`;
5. `@particles/turn <- tick` stages an at-most-once, after-commit GPU dispatch;
6. GPU completion telemetry returns as runtime host-input packets; and
7. the CPU graph writes those values through the console host.

The config uses `require-resident`, so the spike fails rather than falling back
to legacy execution. D4 gains a generic parsed-program loading entry point; it
does not contain GPU policy. Source projection and GPU lowering remain owned by
the mixed executor. The parsed-program loader is the small reusable change to
upstream into D4.

This is section-level source projection, not yet general dependency-graph
partitioning. Cross-region values, several GPU regions, and automatic placement
still require a compiler partition plan and a multi-region scheduler.
