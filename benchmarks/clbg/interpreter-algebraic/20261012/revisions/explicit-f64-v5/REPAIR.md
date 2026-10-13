This additive candidate preserves module-bridge-v4's terminal failure and its
successful strict preparation. The v4 reduction compiler reported E0689 twice:
the receivers of algebraic_add were ambiguous `let mut sum = 0.0` bindings.
Broad uses the same two declarations and has not yet been compiled by v4.

Each alternative source adds exactly two explicit f64 annotations. The sealer
requires the complete source to equal the original with only those replacements.
Strict source, recurrence, operation ordering, coefficients, integer/error paths,
runtime settings and compiler remain unchanged. Every actual source revision
will have its own fresh project and bounded conformance/output preparation.

guarded_prepare_v5.py preserves the previous proposals, failures, actual strict
binary/conformance/codegen proof and passed compile-input fixtures. Both sealing
and preparation acquire the shared timing mutex before hashes or execution.
The first proposed command is metadata-only --seal; root must review its actual
proposal digest before --prepare. Zero full performance runs are authorized.
