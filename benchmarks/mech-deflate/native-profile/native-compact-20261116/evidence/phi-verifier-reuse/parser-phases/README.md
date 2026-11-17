# Reported parser intervals on Apple M1

| Four balanced source builds | Median reported parsing time |
| --- | ---: |
| Mech | 1.02029 s |
| Roc | 3 ms |

Roc reports `Parsing (summed)` as 3 ms in every capture. Its compiler
sums module tokenization and parsing under type checking. All runs use
`--jobs=1`, `--no-cache`, and the same source as the matched build.
The Mech timer surrounds `parse_canonical_document`, the strict-clean
check, and the syntax cast. That entry creates the lossless syntax tree
and the syntax and restart indexes. The reported intervals differ by
approximately 340×.

Both compiler binaries, source files and generated artifacts match the
ordinary compiler comparison. These data come from separate diagnostic
runs with Roc's `--timings` enabled.
[Plan](plan.json) · [All four phase observations](summary.json)
