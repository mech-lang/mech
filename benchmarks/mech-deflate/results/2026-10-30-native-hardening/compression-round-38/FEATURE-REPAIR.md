# Separate inherited feature repair

Repair commit: `dc77024f29f373f0938695531ab2b7c975bb1e7e`.
Ownership candidate: `f64c9e65f26e6fc47cd16c0ea419643712196fbd`.
Clean published parent: `ba198160dc51162da49d962866fe4a19e0aa051c`.

The new reduced-profile checks found two inherited errors. Both were reproduced
on a clean, sparse source checkout of the published parent using the actual Rust
compiler, locked/offline Cargo, one job, incremental off and repository-wide
warnings-as-errors:

- `cargo +nightly-2026-03-03 check -p mech-engine --no-default-features --features native-artifact,sequence_buffers --locked --offline`
  fails with E0599: the borrowed unsigned view calls
  `supports_unsigned_matrix_length`, whose implementation belongs to `pure_text`.
- The same command with `--features native-artifact` fails with E0599: the uniform
  collection candidate calls `primitive`, unavailable without the typed selected
  library vocabulary.

The borrowed view now queries length only with its owning feature, and replace/
append only with theirs. The original full-profile short-circuit order is kept.
The uniform collection candidate conservatively declines when the base profile
has no typed selected-library vocabulary. It never infers factory semantics from
an operation name, exposes a missing private vocabulary, or activates extra
features. Existing validation, value kinds and warnings remain intact.

Actual Cargo checks now pass independently for all four configurations:
`native-artifact`; `native-artifact,sequence_buffers`;
`native-artifact,pure_text`; `native-artifact,map_ops`. These are compilation
checks, not claimed executed test counts. The ownership regression, actual
component/runtime execution and performance evidence are separate.

Setup failures are retained: the initial new sparse checkout had not populated
its index/files, corrected before the clean-parent runs; an experimental attempt
to expose the primitive getter without its unavailable backing type failed
locally and was reverted. No such getter change is in the repair. The first
probe build also needed the established Cargo environment and the arena's
explicit lifetime. These are diagnostic setup errors, not compressor failures.
