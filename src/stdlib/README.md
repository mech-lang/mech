# mech-stdlib

`mech-stdlib` is the explicit, feature-selected static composition layer for
Mech distributions. It installs engine-owned intrinsics and selected standard
machine implementations into immutable runtime or source function catalogs.

The crate does not own execution, host integration, dynamic modules, parsing,
or bytecode compilation. Call `runtime_catalog()` for bytecode-only execution
and `source_catalog()` when source specialization is required.
