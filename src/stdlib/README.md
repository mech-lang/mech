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

The frozen standard runtime catalog uses the `standard-linked-dynamic-shape`
distribution. Standard profiles therefore include the dynamic shapes
`row_vectord`, `vectord`, and `matrixd`, while fixed shapes remain individually
selectable and covered by specialization fixtures. The frozen catalog
contracts bind this dynamic shape closure; profiles selecting fixed shapes
have a separate exhaustive compatibility digest.

Smaller distributions combine a layer with selected operation, value, and
shape features. For example:

```toml
mech-stdlib = {
  version = "0.3.5",
  default-features = false,
  features = ["runtime", "f64", "math_add"],
}
```

The crate has no default features. Distribution manifests should use
`default-features = false` and select a profile explicitly when dependency-graph
size or execution capability is part of their contract.

That profile links scalar-add runtime support without source specialization or
the bytecode compiler. Use `runtime_catalog()` for bytecode-only execution and
`source_catalog()` for source execution or compiler tooling.

## Pure numeric text and length

The `pure_text` operation feature is included in the standard and full
operation profiles. It exports ordinary source operations through the
`number` and `sequence` modules:

```mech
+> number
+> sequence
n := number/parse-u64("42")
formatted := number/format(1.25, 9)
length := sequence/length("é👩🏽‍💻")
```

`number/parse-u64` accepts a nonempty string of ASCII decimal digits and
checks overflow. Its result is a canonical `u64`; signs, whitespace,
fractions, exponents and non-ASCII digits are errors.

`number/format` accepts plain signed and unsigned integers of 8, 16, 32, 64
or 128 bits, and `f32` or `f64`. Integer precision must be zero. Floating
precision is an integer from zero through twelve, producing fixed decimal
notation with rounding. A matrix uses one scalar precision for every
element, preserving its dimensions and element order. Precision has kind
`u64`; a bare integer literal receives that type from the call declaration.

`sequence/length` returns a canonical `u64`. For a string it counts extended
grapheme clusters; the example above has length two. For a matrix it counts
elements, including when those elements are strings or aggregates.

The core owns these operations' type schemes and pure contracts. Managed
runtime factories and resident kernels use those declarations, and native
application planning retains their `pure_text` dependency feature. Invalid
inputs leave the previous published result intact.

The crate does not own execution, host integration, parsing, dynamic modules,
or dynamic-library loading. `mech-engine`, `mech-runtime`, and host-provider
crates remain independent of standard distribution composition.

The `bits` module supplies scalar `u64` word operations through the
`integer_bits` feature, included in the standard and full profiles:
`and`, `or`, `xor`, `not`, `shift-left-wrap`, `shift-right-wrap`,
`leading-zeros`, `trailing-zeros`, `wrapping-add`, `wrapping-sub`, and
`wrapping-mul`. Binary arguments and results are `u64`; the zero-count
functions return `64` for a zero word. Shifts use the count modulo 64 and
fill vacated bits with zeros. Wrapping arithmetic reduces modulo 2^64;
ordinary arithmetic remains checked. These are word operations, without
implicit matrix broadcasting or floating-point conversion.

```mech
+> bits
word := bits/xor(305419896u64, 4294967295u64)
low-byte := bits/and(word, 255u64)
bits/trailing-zeros(word)
```
