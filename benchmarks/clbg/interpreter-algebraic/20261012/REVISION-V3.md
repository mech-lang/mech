The compile-input-v2 metadata phase stopped before a proposal or any compiler.
Its five temporary-file methods ran once: two passed and three rejected their
valid fixture paths because macOS exposes the temporary directory through
`/var` while resolving the include target to `/private/var`. The failed child
receipt, complete stdout/stderr and source bytes remain unchanged.

The sole fixture repair in `test_compile_inputs_v3.py` resolves the temporary
workspace root before creating its files. Production `compile_inputs` and
snapshot functions are unchanged. The original workspace root already uses
its canonical physical path. New metadata and preparation output live only in
`revisions/compile-input-v3`; no prior failed directory is resumed.

The new seal also verifies and pins the original exact proposal and retained
compiler101 report/logs, plus the v2 failed fixture receipt/logs. It requires no
v2 proposal or preparation directory to exist. These establish separate honest
failure lineage; no native/output proof exists to reuse. The previous five
method source/snapshot boundary checks remain the proposed fresh metadata gate.

The three runtime variants, general ScalarTape/ScalarTile paths, compiler and
flags, include inventory, conformance and33 actual bounded CLI cases retain the
previous reviewed scope. No v3 fixture, metadata seal, compiler or probe has
run. Root reviews the emitted new digest before any compiler preparation; full
performance runs and score admission remain unauthorized.
