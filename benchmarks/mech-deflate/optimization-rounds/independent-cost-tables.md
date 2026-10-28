# Round 09: independently constructed cost tables (source experiment)

Published pinned-source compiler baseline:
`a59117d63d2a07d3e0b5cdac45dab202114b95f7`.
Retained executable SHA-256:
`0fa38254f8f5773f49749201a07ad2032356d38b5d7f47a44f4c581a6bdc6570`.
Latest paired eight-byte result: 454.89x Roc, fresh reference 0.000285 s.

The existing permanent default-cost case requests about 44000 allocations for
544 independent updates even after consuming stores. The latest whole-program
profile remains dominated by runtime visits, accounting and data movement. This
experiment changes the application expression of independent tables, as explicitly
recommended in the supplied source audit. It is NOT a compiler-only speedup and
must not replace the pinned-source comparison or be attributed to Engine changes.

Keep the 25-component assembly and its pin unchanged. Supply a separate source
variant for the nine default/code-derived/adjusted literal, length and offset
cost loops. Construct exact 256/259/32-entry results directly using ordinary typed
comprehensions; the uniform literal default uses existing uniform construction.
Preserve first-three length entries, all arithmetic/masks/order/tie behavior,
lookup-table values and every non-table compressor function. For other input
lengths or starting positions retain the original sequential helper, so malformed
and suffix-call behavior remain available for one-dimensional lists. Both
branches must return an exact row schema: the successful sequential fallback is
copied to a row, preserving vector contents while adding disclosed fallback
allocation. Multidimensional matrices are outside this source experiment; the
pinned compiler baseline retains their separately qualified semantics. Cost
arrays in the actual compressor are one-dimensional lists throughout. Do not change tree/cache, search depth,
nice/EOF limits, path/output reservation, clear/repetition or timing boundaries.
No histogram reuse, dead-initializer removal, cache representation, writer or
header optimization is bundled into this experiment.

Use the SAME retained compiler executable for before/after; only source-derived
bytecode differs. Produce both through the maintained frontend and revision-14
codec, and retain all source/artifact hashes. Establish mathematical equality
against existing reference cases in original/decoded JIT forms, unchanged expected
values and finite execution allowances. Exercise nonzero preserved prefixes,
short/long tables, zero/late starts, short lookup arrays, overflow, input aliases,
prior publication, failure and retry. Inspect native selection/allocation evidence
so a merely different expression is not assumed faster.

Run the same eight-byte paired cohort against a fresh pinned reference, with
matching allocator and evaluate_into-only boundary, no competing compilation,
all raw rows and cleanup. Take a separate diagnostic profile. Preserve source
allocation/accounting changes explicitly; they differ by design from the faithful
source baseline. Commit all results and failures before another optimization.
Publish to draft #973, request review and cancel CI. Do not start larger inputs
or any optional compiler project while this bounded experiment is unmeasured.

Before primary timing, the direct/general matrix branch failed exact source-kind
joining. A nominal-record join has the same restriction. The variant adapts its
vector fallback to an explicit row with the existing comprehension constructs;
no compiler/type-system change is made. Original/decoded reference cost cases
and the unchanged pinned assembly test pass before complete timing. Preserve
these failed source forms as setup evidence, not a new compiler work package.
