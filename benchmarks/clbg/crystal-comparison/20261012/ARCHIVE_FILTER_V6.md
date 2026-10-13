The v5 six-method fixture phase passed, but its metadata seal rejected the
literal archive target text of a foreign-platform symlink. No proposal or
compiler/benchmark followed. A separate common-lock diagnostic inspected all
nineteen links. Exactly eight Linux-family entries use the original archive
target `crystal-1.21.1-1./../linux`; the recorded `tarfile` data extractor
normalizes this to `linux` before creating the link. All eight actual targets
match that lexical normalization, producing self-links. Reapplying the data
filter to those already-created self-links raises ELOOP; the complete second
diagnostic captures those errors and pins the archive, original extractor,
Python library source, failure lineage and diagnostic script.

This additive archive correspondence revision compares the exact normalized
link text produced by the recorded extraction policy. It calls `normpath` on
the original archive metadata and never resolves or recreates a link. Absolute
or escaping normalized targets remain rejected. Regular file hashes, member
types, parent checks, hardlink topology and exact no-extra-file inventory are
unchanged. The original official archive digest and pinned extractor anchor
the normalization; this does not permit arbitrary semantically equivalent
replacement link text.

The six new methods retain their existing counterexamples and add the actual
normalization/self-link shape to the acceptance case. The production compiler,
language sources, flags, SDK and fifteen binaries/112 bounded outputs remain
unchanged. Every old failed metadata attempt, log and passed fixture receipt is
preserved. The new files and `private-input-archive-filter-v6.json` proposal use
fresh paths; no prior phase is resumed or overwritten.

After the six methods pass, metadata sealing again reconciles exact old proof
and emits an independently reviewable digest. Five fresh builds, 44 missing
output checks, pending generic/work proofs, canonical LOC and all full timings
retain their prior separate gates. No v6 phase has run at authoring time.
