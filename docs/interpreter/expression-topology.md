# Expression evaluation topology

`mech-interpreter` keeps expression dispatch in a small façade and assigns each
evaluation responsibility to a named module. The subsystem forbids unsafe code;
container outputs decoded from bytecode are checked before they are used.

## Ownership

- `expressions/mod.rs` owns the `Environment` type, the `expression`
  dispatcher, module declarations, and deliberate compatibility re-exports.
- `environment.rs` owns deferred expression-solve scope state.
- `registration.rs` owns initialized function registration and ordered batch
  registration.
- `variables.rs` owns variable lookup, addressed identifiers, environment
  fallback, and kind-cast registration.
- `ranges.rs` owns range evaluation and range-function registration.
- `comprehensions.rs` owns qualifier expansion, generator extraction, value
  detachment, set and matrix comprehension functions, compilers, and
  descriptors.
- `subscripts/mod.rs` owns slice/subscript dispatch and shared formula/range
  index conversion.
- `subscripts/dot.rs` owns named, integer, tuple, record, table, and swizzle
  access.
- `subscripts/brace.rs` owns brace formula/range access.
- `subscripts/bracket.rs` owns bracket, matrix, range, scalar, and all-index
  access combinations.
- `subscripts/string.rs` owns string access compile mode, liveness tracking,
  and direct-versus-live source/index decisions.
- `formulas.rs` owns factor and term evaluation, unary operators, formula
  compiler selection, and intermediate function registration.
- `functions.rs` owns expression-level function-call argument evaluation and
  dispatch. `crate::functions` continues to own function definitions, frames,
  execution, and the registry.
- `matches.rs` owns arm selection, guards, output-kind validation, enum
  exhaustiveness, wildcard coalescing, and option-matrix fallback.
- `errors.rs` owns expression-specific structured errors.
- `tests/` owns the behavior-grouped private expression tests described in the
  [core and interpreter test topology](../testing/core-interpreter-test-topology.md).

## Compatibility boundaries

Existing public expression items remain available through
`mech_interpreter::expressions::<name>` and the crate-root re-exports.
`mech_interpreter::functions::function_call` is retained as a compatibility
re-export even though expression-level call lowering now lives in
`expressions/functions.rs`.

Production modules import their dependencies explicitly. Cross-module helpers
use the narrowest visibility that permits sibling coordination; they are not
promoted into the public API.

## Safety boundary

The expression directory declares `#![forbid(unsafe_code)]`. Set-comprehension
bytecode construction explicitly matches `Value::Set` and returns
`SetComprehensionOutputKindMismatch` for any other output kind. The expression
subsystem therefore no longer requires an unsafe-boundary allowlist entry.
