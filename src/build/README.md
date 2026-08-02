# mech-build

`mech-build` provides deterministic planning and project generation for native
Mech applications. It consumes official Mech bytecode plus trusted function,
host, and dependency catalogs; bytecode never selects Cargo dependencies.

Phase 1 freezes the public build-plan contract and proves a deliberately small
set of native application vertical slices.
