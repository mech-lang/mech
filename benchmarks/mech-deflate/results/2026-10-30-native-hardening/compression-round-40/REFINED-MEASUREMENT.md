# Matched comparison of both selections

Retain the initial four-pair cohort and all counters/profiles. Compare the
frozen published baseline, first valid candidate and regrouping refinement in
four NEW independently prepared triplets, with one fresh shared 20,000-call
reference. The same three-way rotating-order configuration from round 39 is
used: 16 blocks ×31 calls per version per preparation, 5952 primary calls,
5976 including warm-up/final, 1984 primary per version. Blocks are descriptive
within four preparations. The fixed historical 0.300 ms ruler is also reported.

All programs are ready before timing; only the requested worker runs. No builds,
tests, audits, profiles or instrumented diagnostics overlap primary. The same
linked full compressor, tiny setup JIT helpers outside timing, pinned source/
bytecode/allocator, real buffers, exact raw/inflate/input-alias/output-capacity/
reuse checks, stable per-compiler work, and final zero cleanup remain. Worker
bodies differ only in their same-build image/identity bindings. Immutable
executable/Object/image hard links avoid redundant disk copies and are hash
checked against frozen binding receipts before execution.

Commit reference, each triplet and summary immediately. Separate CPU, allocation,
code-size and profile diagnostics follow; none supplies primary elapsed values.
No new allowance, algorithm change, larger-input result or parity claim.
