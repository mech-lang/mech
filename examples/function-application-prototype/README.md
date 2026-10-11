# Function application and matrix AOT

This is an opt-in front end for the v0.4 native AOT path. It normalizes application
syntax into ordinary canonical source, then uses typed graph lowering and native
kernels. Canonical parser/formatter integration for the surface syntax is still
separate work; the interpreter is unchanged.

* `value |> f` applies a unary function.
* `value |> f(_, bound)` fills each standalone `_` with the same left value,
  evaluated once even when the stage repeats the hole.
* Stages associate left to right. Bracketed multiline calls and continuation
  pipeline stages are supported, including standalone effect pipelines.
* `<[T]>` denotes a vector of elements of kind `T`, with any length.
* `f(x<T>) => <U> := expression` defines a typed expression-bodied named function.
  Multiple typed inputs and existing statement-body functions remain supported.

Source functions can capture immutable global bindings defined before use, including
scalars, vectors, text and lookup tables. Captures use lexical scope and remain
fixed while the function is invoked, including nested iteration and record mapping.
The compiler tracks capture dependencies in its graph. Small environments use
existing argument slots; larger environments use generic runtime storage without
changing the normal operation ABI. Function bodies and iteration remain AOT code.
IO inside iteration callbacks and undefined/forward captures are rejected.

The named function/application syntax uses hygienic intermediate bindings and
retains original physical lines. Arbitrary pipelines nested inside conditional or
call-argument expressions are rejected until evaluation-order-safe lowering exists.
The root pipeline input may itself be a complete parenthesized pipeline.

The working ordinary-matrix spectral-norm example has nine nonblank physical lines:

```
mech build examples/function-application-prototype/spectralnorm-matrix.mec --backend cranelift --no-config --offline -o /tmp/spectralnorm-matrix
/tmp/spectralnorm-matrix 100
```

Expected output: `1.274219991` followed by a newline. The earlier seven-line
`spectralnorm.mec` shows the lower-level matrix application interface; current
benchmark sources use the nine-line ordinary-matrix form instead.

Checks:

```
python3 examples/function-application-prototype/verify.py
python3 examples/function-application-prototype/verify_matrix.py
python3 examples/function-application-prototype/verify_capture.py
python3 benchmarks/clbg/verify_function_application.py --help
```

Canonical benchmark sources have been migrated where the feature helps. Preserved
before-sources, executables, input pins and comparison reports live under
`benchmarks/clbg/function-application-audit/20261009T171437Z`. Historical timing
reports and chart data are retained as historical observations.

## Existing matrix forms in native AOT

`spectralnorm-matrix-proposed.mec` now compiles through native AOT using ordinary
ranges, matrix comprehensions, transpose, matrix multiplication, and dot product.
The filename retains the prototype's history. There is no new matrix-construction
syntax, benchmark executor, or special coefficient formula.

The admitted matrix producer is a pure scalar expression over two nested,
zero-based exclusive `u64` ranges. Comprehensions concatenate horizontally:
the inner comprehension generates a row over `j`, its transpose makes a column,
and the outer comprehension joins those columns over `i`. The final transpose
puts `i` on rows and `j` on columns. A seed comprehension is transposed to a column
for matrix–vector multiplication. Rectangular shapes are supported.

The compiler snapshots the declared dimensions, compiles the Mech entry expression
as a native scalar graph, and fuses it into matrix–vector multiplication. Transpose
swaps dimensions and indices; it allocates no coefficient array. Shape guards
check vector lengths. The coefficient formula retains its ordinary arithmetic:
`1.0 / denominator` followed by multiplication is not rewritten to vector division.

Eager checked-arithmetic failures remain observable at matrix construction.
A native unsigned bounds proof validates the whole coefficient domain once; if
that proof is inconclusive, a checked scalar traversal validates entries in source
generator order. Validation remains before subsequent IO, including when iteration
runs zero times. This is allocation elimination, not a new lazy matrix semantic.

The native typed producer retains declared zero-length axes, such as `0 × n`
and `n × 0`, and the floating-point element kind. Legacy resident empty constructors
currently collapse dimensions to `0 × 0`; this compiled shape behavior is an
explicit extension, not a claim that those resident constructors have changed.

The current slice excludes nonzero/stepped/inclusive generators, filters, varying
vector-comprehension bodies, matrix materialization, matrix–matrix multiplication,
and captured scalar values inside coefficient functions. Immutable dimensions
can be captured by the iteration function. Unsupported forms receive diagnostics.

Run `python3 examples/function-application-prototype/verify_matrix.py` for independent
scalar/SIMD, rectangular, zero-axis, tail, capture, shape, and eager-error checks.
See `docs/design/native-matrix-application.mec` for the lowering contract.
