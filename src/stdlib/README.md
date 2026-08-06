# mech-stdlib

`mech-stdlib` is the explicit, feature-selected static composition layer for
Mech distributions. It installs engine-owned intrinsics and selected standard
machine implementations into immutable runtime or source function catalogs.

The selected Cargo feature closure is the distribution: operation features
activate their owning machine, while value and shape features weak-forward only
to machines that are already selected.

## Catalog API

- `install_runtime` adds engine intrinsics and selected machine runtime
  factories to a caller-owned `FunctionCatalogBuilder`.
- `install_source` adds the corresponding source specializers and exports when
  the `source` feature is enabled.
- `build_runtime_catalog` and `build_source_catalog` construct fresh immutable
  catalogs.
- `runtime_catalog` and `source_catalog` return shared catalog values. Normal
  `std` builds cache them separately; `no_std` builds construct fresh catalogs.

`source_catalog()` always installs runtime entries before source entries. A
compiler-enabled build uses this same source catalog; there is no compiler
catalog because lowering support does not change catalog names or IDs.

## Feature profiles

The layer features are:

- `runtime` for concrete factories and kernels;
- `source` for runtime plus source specializers and export metadata;
- `compiler` for source, runtime, and bytecode lowering.

The complete profiles are `full_runtime`, `full_source`, and
`full_compiler`.

The frozen PR2 runtime artifact is the `standard-linked-dynamic-shape`
distribution. Standard profiles therefore include the dynamic shapes
`row_vectord`, `vectord`, and `matrixd`, while fixed shapes remain individually
selectable and covered by specialization fixtures. Enabling every fixed shape
would change the catalog from 9,019 to 116,603 entries and invalidate the
frozen raw-catalog digest, so frozen compatibility defines the standard shape
closure.

Smaller distributions combine a layer with selected operation, value, and
shape features. For example:

```toml
mech-stdlib = {
  version = "0.3.5",
  default-features = false,
  features = ["runtime", "f64", "math_add"],
}
```

The crate default is `full_source`. Distribution manifests should use
`default-features = false` and select a profile explicitly when dependency-graph
size or execution capability is part of their contract.

That profile links scalar-add runtime support without source specialization or
the bytecode compiler. Use `runtime_catalog()` for bytecode-only execution and
`source_catalog()` for source execution or compiler tooling.

The crate does not own execution, host integration, parsing, dynamic modules,
or dynamic-library loading. `mech-engine`, `mech-runtime`, and host-provider
crates remain independent of standard distribution composition.
