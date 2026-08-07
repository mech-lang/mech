# mech-build

`mech-build` provides deterministic planning and project generation for native
Mech applications. It consumes official Mech bytecode plus trusted function,
host, and dependency catalogs; bytecode never selects Cargo dependencies.

The public build-plan contract is frozen and the crate covers engine, hosted,
and live native-application vertical slices.
