# mech-build

`mech-build` provides deterministic planning and project generation for native
Mech applications. It consumes canonical ProgramArtifact bytecode v1 plus
trusted function, host, and dependency catalogs; bytecode never selects Cargo
dependencies. Both host-free and hosted executables use the resident runtime.
Instruction-only bytecode and mixed legacy/artifact execution authority are
rejected before project generation.

The public build-plan contract is frozen and the crate covers engine, hosted,
and live native-application vertical slices.
