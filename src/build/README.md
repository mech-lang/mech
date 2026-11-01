# mech-build

`mech-build` provides deterministic planning and project generation for native
Mech applications. It consumes canonical ProgramArtifact bytecode v1 plus
trusted function, host, and dependency catalogs; bytecode never selects Cargo
dependencies. Both host-free and hosted executables use the resident runtime.
Instruction-only bytecode and mixed legacy/artifact execution authority are
rejected before project generation.

The public build-plan contract is versioned and the crate covers engine, hosted,
and live native-application vertical slices.

## Execution boundary

Here, a native application is a platform executable packaging the canonical
artifact and resident runtime. It does not mean that Mech operations have been
lowered to machine code. Host-free and hosted projects differ in their typed
host adapters, not their language semantics or runtime authority.

This crate's contract is not a claim that every CLI build route is unified.
The opt-in `mech build --backend cranelift` still uses the older source-based
sequence/control/grid compiler in `mech-gpu::native_build`; it does not call
this artifact project generator. Its accepted programs and host configuration
are not interchangeable with the resident route. Sharing syntax does not make
independent semantic lowering equivalent.

The required architecture is one canonical source compiler and executable
artifact, with resident, JIT and Object/AOT backends sharing semantic,
capability-admission and atomic-publication contracts. The canonical native
backend is an incomplete step toward that architecture. Migrate the older
compiler's capabilities onto the artifact path, qualify resident/JIT/linked
AOT agreement, then cut over the CLI and remove alternate source lowering.
Unsupported backend operations must fail explicitly, without reparsing source
or silently falling back. This migration remains implementation work, not a
documentation-only correction.
