# Mech Metal Particle Field

This standalone prototype proves a mixed CPU/GPU Mech application without a
handwritten particle kernel:

- `particles.mec` defines the nonuniform initial field and all turn equations.
- `mech build` lowers the turn to f32 SPIR-V MLIR and emits the initial f32
  state image directly from the same activated program.
- The AppKit host writes four scalar controls: pointer x/y, pointer-down, and
  frame delta.
- Metal computes all 65,536 particles in place.
- The render pass reads x/y from that exact `MTLBuffer`. No particle array is
  copied to the CPU between compute and render.

## Build And Run

Requirements are macOS, LLVM MLIR 22, and `spirv-cross`.

```sh
brew install llvm spirv-cross
examples/metal-particle-field/build.sh
examples/metal-particle-field/run.sh
```

Hold the left mouse button to attract particles. Move while holding it to drag
the field, use the right mouse button to pan the camera, and scroll to zoom.

For a finite automated run and PPM capture:

```sh
MECH_FRAMES=180 MECH_CAPTURE=/tmp/mech-particles.ppm \
  examples/metal-particle-field/run.sh
```

## Data Path

```text
particles.mec
  |-- mech build --emit initial-state --> particles.initial.f32
  `-- mech build --emit mlir          --> SPIR-V --> mech_turn Metal function

CPU UI state (4 f32 values)
  --> resident Metal state buffer
      --> generated Mech compute pass
      --> generic point render pass reads the same buffer
      --> screen
```

The initial-state emitter avoids the old path that generated one Rust
assignment for every nonuniform value. At 65,536 particles that old path made
a 262,000-line temporary source file; the direct emitter writes the roughly
1 MiB upload image without invoking `rustc`. On the Apple M1 used for this
prototype, the direct emit took 2.21 seconds. It still evaluates the full
nonuniform activation vector at build time, so GPU-side initialization or a
compact initial-state expression plan remains necessary for million-particle
builds.

## Prototype Boundary

This proves the accelerator data plane, not the final host model. The AppKit
file still performs window/input/device/render plumbing and overwrites scalar
state offsets between turns. The four scalar bindings are currently identified
by declaration order in generated layout metadata. A production GPU host needs
named external bindings in bytecode or artifact metadata, automatic region
partitioning, and an explicit transaction policy for device-resident state.

The current fast path is single-buffer and non-transactional: a failed dispatch
cannot roll back particle state. The renderer shader is generic display
plumbing and contains no simulation equations.
