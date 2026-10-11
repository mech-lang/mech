# Additive Uiua spectral norm vectorization

These candidates preserve the canonical binary64 spectral norm algorithm and
all 40 matrix-vector products. They replace a tabled scalar coefficient function
with primitive outer addition and pervasive array arithmetic. The visible Uiua
source computes `1 / ((i+j)*(i+j+1)/2 + i + 1)` for every term of every product.

`sources/compact9.ua` constructs an entire coefficient array inside each product,
with coefficient functions inlined as ordinary expressions. Its triangular-sum
function consumes the current vector and derives its length at runtime.
`sources/compact.ua` keeps the same whole-array computation in separately named
functions for readability, including a live-vector index function.
`sources/fast.ua` constructs at most 128 output rows per block; it consumes the
live block record, retains the final partial block, and visits every input index.
Forward products add output indices to triangular sums. Transpose products
transpose the triangular block, add input indices, then transpose it back.
No coefficient or triangular matrix is stored between products.

The compact, readable whole-array and bounded forms have 9, 13 and 17 normalized
LOC respectively.
Every independent binding and the block data declaration occupy their own line;
ordinary pipelines are single expressions. No native benchmark kernel or FFI
helper is used. The official Uiua 0.19.1 ARM64 runtime implements primitive outer
addition and ordinary pervasive arithmetic with native numeric array operations.

Preparation and validation, from this directory:

```sh
python3 prepare.py
python3 validate.py
python3 prepare_manifest.py
```

The scripts reuse the frozen suite's pinned official runtime. Its installation
recipe is `../install_uiua_runtime.py`; release/archive URLs and hashes are
carried into the additive manifest. Runtime source excerpts pin the native
primitive table path, prefix broadcasting and the compiler's pre-evaluation
rule. The coefficient functions require live inputs, so they are not zero-arity
constant matrix bindings.

Validation acquires `/tmp/mech-clbg-timing.lock` before running any program or
reference computation. It checks complete output at `n=1,2,7,100,113,129,257`
against an independent scalar Python binary64 implementation. Separate trace
sources at `n=2,129,257` retain every original expression and check 20 forward
products, 20 transpose products, and exactly `40*N*N` coefficient elements.
For the bounded form, traced row counts also verify the 128-row limit and final
partial blocks. Trace sources are diagnostic evidence and are never the timed
application source.

This package performs no full-workload timings. It is additive evidence for a
new frozen audit; existing frozen sources, manifests, guards and measurements
remain untouched. Full candidate and fresh-control measurements are required
before chart admission.
