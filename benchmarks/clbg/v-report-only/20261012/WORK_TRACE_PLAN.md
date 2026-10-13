# Unscored V work-trace proposal

`source_trace.py` is authored and unexecuted. It does not compile, launch,
measure or publish programs. Root must first obtain the14 oracle,5 preparation
and8 owned-process cleanup fixture receipts, then build/execute distinct trace copies through the shared
sampler while holding `/tmp/mech-clbg-timing.lock`. Original scored source and
compiled binaries remain unchanged. Trace copies and results are always
preparation-only, full_workload=false and never_chart_eligible=true.

The instrumenter uses exact, unique source anchors and stops if a subsequent
source revision changes them. It adds only logging, a counter and a deferred
file close. It preserves the original arithmetic/map-update statements,
predicate, loop and stdout. A dedicated fresh file named by
`MECH_V_WORK_TRACE` retains complete records; the sampler's truncated stderr
capture is unsuitable for these logs. No audit helper is inserted in a scored
source. Root must pin original and trace source, instrumentation tool, input,
binary, compiler/runtime, trace and full stdout in its review receipt.

For nucleotide counting, the parent dumps all tables after computation and all
worker joins, before frequency/query output. Each actual map iterator emits
every key/count, actual distinct-key count and total windows. The independent
reader parses the THREE token, ignores semicolon comments, handles CR/LF/EOF,
normalizes case and constructs seven ordinary Python Counters over every
overlapping window. Packed keys are decoded independently. Reconciliation
requires exact equality of all seven maps, totals=max(0,sequence_length-k+1),
unique keys, ordered table summaries, lengths1/2 included and unchanged full
stdout. Growable-map backing types are also verified in the original V source;
output evidence alone cannot prove a data-structure requirement.
The independent parser recognizes only CR/LF separators and space, tab or
end-of-header after THREE. Vertical-tab/form-feed tokens are not silently
accepted as Python whitespace. This was corrected by the read-only peer review.

For pi, the actual numerator, denominator and quotient variables are logged
immediately after both source divisions. Acceptance is logged only inside the
original digit-output branch. A separate Python q/r/t LFT tracks every candidate,
requires the exact lower3q+r and upper4q+r numerators and denominator, computes
both independent integer quotients, verifies the actual acceptance branch,
digit, row padding/count and complete stdout. Endpoint-basis sources evaluate
the upper endpoint first; the compact form evaluates the lower first. No second
quotient is derived from the first. Trace counts must satisfy
actual_divisions=2*tested_candidates and accepted_digits=n.

The proposed trace scope is bounded: DNA inputs at most1MiB and pi n0..256
(normally97). These are qualification limits, not restrictions or optimizations
in measured benchmark programs. Large full inputs still need their separate
complete-output qualification and three genuine timing samples. Instrumented
times are never reused as samples.

Focused malformed-trace fixtures are needed under the mutex before execution:
missing/duplicated map, wrong packed key/high bits/window count, comments and
unterminated records, missing/reordered second division, wrong quotient or
denominator, skipped/extra acceptance, incorrect digit/tail/count, partial trace,
and stdout differences. No such fixtures or native traces have run yet.

Root's preparation pipeline owns the actual native command and validates its
normal sampler/native exits, inherited mutex/PGID, environment, memory, complete
output, unchanged pins and cleanup. A passed text reconciliation is additional
work evidence and cannot substitute for those process/source gates.
