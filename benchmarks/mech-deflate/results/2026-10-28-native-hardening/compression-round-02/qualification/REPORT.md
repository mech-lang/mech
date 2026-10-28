# Round 02 qualification follow-up

The production implementation is `335de1c16743343fa6745d7ed2e638f6f1626fa9`.
Timing receipts were committed as `0c2ed38dd037bd23aed8fc2f5d78a42b7d3f562b`.
The subsequent changes correct a new source test and move its separate engine
test module to the repository's required flat layout. Neither changes the
numeric implementation measured in the parent report.

| Command receipt | Executed outcome |
| --- | --- |
| `../round02-single-projection-unit.json` | 3 engine tests pass; GPU filter selects 0 and is not counted |
| `round02-program-regressions.json` | 54 pass, 1 newly added test fails before runtime, 323 filtered |
| `round02-matrix-source-selection.json` | Alternative matrix source still rejects before runtime; 1 fails, 377 filtered |
| `round02-word-source-recovery.json` | Corrected scalar-source test passes, 377 filtered |
| `round02-static.json` | Introduced nested test-file layout fails existing guard |
| `round02-static-fixed.json` | All 12 architecture/catalog/layout/inventory/warning/boundary/format commands pass |

There are **58 distinct passing tests**: three engine tests, 54 other program
tests, and the corrected program test. The initial 55-test invocation did not
pass as a whole. The passing linked test is
`linked_single_word_update_preserves_publication_and_recovers`; it covers
valid → assertion rejection → valid, assertion-before-arithmetic ordering,
unchanged prior publication and cleanup. Existing component expectations and
resource allowances are unchanged. No test was ignored within the selected
invocations. Tests outside the recorded filters, large inputs and full corpus
were not run here.

The release feature profile is `mech-gpu/canonical-native-program` with no
default features. Engine units use the combined `-p mech-engine -p mech-gpu`
profile; integration uses `-p mech-gpu`. Commands use `--locked --offline
--release -j1` and `--test-threads=1`. JSON receipts retain full commands,
environment, working revision, dirty paths, duration and exit status. Repository
`-D warnings` remains enabled. Raw logs are preserved byte-for-byte in gzip.

The introduced source fixture incorrectly used a free-shaped matrix for a
scalar assignment. The corrected fixture uses the compressor's actual scalar
write, a rectangular destination, immutable aliases and original/decoded
artifacts. It checks success, invalid index, zero copied-byte allowance,
unchanged publication after both failures, successful retry and complete
cleanup. The engine tests separately exercise one-cell packed source data.

Four caller-form reproducers and their normal source/compiler diagnostic are
retained. Free-shaped matrix sources reject with
`source-semantics/incompatible-assignment-kind`; a fixed one-cell source passes
source semantics but rejects native lowering with
`WitnessTypeMismatch { node: NodeId(1) }`; a matrix selector/free-shaped source
rejects with `source-semantics/incompatible-annotation-shape`. These forms are
not qualified native caller support and are not used by the compressor's
scalar stores. The source and lowering stages are unchanged by this runtime-only
optimization. No clean-parent rebuild of these reproducers is claimed, so their
historical first occurrence remains unclassified. They remain a separate
source/native-lowering follow-up, not an introduced runtime failure.

The initial engine test location violated the existing file/directory rule.
The flat file is byte-identical to the test that passed all three engine tests;
its Rust logical module is unchanged, as recorded in
`test-module-relocation.json`. Only its `cfg(test)` path changes. The full
static suite passes after relocation. The optimized engine unit binary was not
rebuilt solely for this byte-identical move.

The earlier CI run 40676927741 received ordinary and force-cancel requests.
An all-jobs query (`jobs?per_page=100`) confirms all 35 jobs completed, with no
active job. CI is not awaited and is not claimed green. The published stack
and inherited qualifications in the main hardening report remain in force.
