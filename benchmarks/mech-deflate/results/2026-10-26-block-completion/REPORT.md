# Full block completion diagnostic

The collector's source algorithm is unchanged from the previous published
checkpoint. This run selects a separate 1 GiB managed account and one-trillion
work/comparison allowance, with 16384 compilation blocks allowed. Actual lowered
blocks remain 4916, and temporary call storage remains 128 MiB. Existing
component caps and the fixed-budget/full-block witnesses remain unchanged.

The compiler/runtime code checkpoint is `4ea406e00f66a098b1e36d9b2e8a0b2474ee33d9`
above architecture cleanup #939. Subsequent commits contain documentation and
reference-provenance checks; no compiler/runtime implementation changed for
these diagnostics. The test driver was locally modified to expose the explicit
larger account. The Rust host is unoptimized, generated code uses `speed`, and
source compilation/reference checking are outside the measured evaluation.
Evaluation includes collector initialization and publication. These are not
complete-compression timings or a controlled language-performance comparison.

## Eight-byte blocks

One executed Rust qualification passes: 1 passed, 0 failed, 270 filtered, in
106.56 seconds (127.94 seconds including compilation). It runs both eight-literal
and eight-overlap cases through original and decoded artifacts and compares
all scalar fields and every cell of all six buffers with the independently
regenerated reference. Input aliases remain shared and dropping published values
returns managed usage to zero.

| Case | Form | Evaluation seconds | Allocation requests | Cumulative requested bytes | Work |
| --- | --- | ---: | ---: | ---: | ---: |
| Literals | Original | 13.794647667 | 360175 | 778588745 | 17462558139 |
| Literals | Decoded | 13.772060208 | 360174 | 778588681 | 17462558139 |
| Overlap | Original | 15.556565625 | 413361 | 618486838 | 16388393557 |
| Overlap | Decoded | 15.556170500 | 413361 | 618486838 | 16388393557 |

Each retains 6615228 managed bytes after publication versus 42780 before.
Cumulative allocation requests are not peak usage or measured copied bytes.
The earlier smaller-account trace used a different compiler foundation and
account allowance, so no causal speedup is inferred from comparing their costs.

## Full 320-byte block

The original artifact completes all 320 positions and exactly matches the full
reference, including cache pointer 324, all tree/cache cells and counter arrays.
Measured evaluation is **663.475494208 seconds**, with 2601119 allocations and
16781118735 cumulative requested bytes. Work is 286379914988, comparison work
15207850276, and visits 976780. Retained managed bytes are 43092 before and
6615540 after publication. Input aliases and complete cleanup pass.

The decoded artifact also completes all 320 positions with exact reference
agreement in **663.621672875 seconds**. It records 2601118 allocation requests,
16781118671 cumulative requested bytes, and identical work, comparisons, visits
and retained memory. Alias preservation and final cleanup pass in both forms.
The terminal Rust test result is **1 passed, 0 failed, 0 ignored, 271 filtered**
in 1374.91 seconds of test execution (1396.24 seconds including setup). The
command receipt records starting HEAD `7ba38a870660706efd47d081f2e25ce046c30ce5`
and its exact dirty test/provenance paths; those changes were subsequently
committed as `cce7e60b6ff9f43c93d028eec761ac48a78c7273` and
`49ffc5e7ac2c9a499e54edab6b9810f02beb17a0`. Formatting passes on the latter
clean revision. Earlier partial observations remain labeled historical evidence.

The original run contains one one-second external sampling observation. Its
host has stripped symbols, so
the anonymous sample is not evidence identifying a particular compiler cost.
At the observation point, `ps` reports 647408 KiB RSS; the sampler separately
reports 53.7 MiB physical footprint and 163.5 MiB peak physical footprint. These
OS metrics are distinct from managed account capacity and cumulative allocator
requests. No peak-managed-memory measurement is claimed.

The source pins and all expected case values are unchanged. Reference validation
now covers all 14 tracked package modules; see the adjacent reference-imports
report for its reviewed fix, 3 passing tests, corpus equivalence and resolved
sandbox/cache denial.

## Remaining work

This qualifies a first final block, not a complete compressor. Larger-alphabet
sampling, window slides, distribution changes, rewinding and carryover still
need execution evidence. The next semantic requirement is generic reserve,
append and clear retaining capacity for U64 path nodes and U8 output. The pinned
parser reserves 305257 U64 slots and appends path nodes without initializing a
whole matrix; its output reserves once and appends bytes. Core canonical
sequences currently use fixed boxed storage, while the existing private typed
collection builder retains a Vec only inside comprehension construction. There
is no maintained source operation exposing the required buffer lifecycle yet.
A minimal source witness and acceptance test precede extending that path.

Near-optimal selection, actual-size Huffman integration, block/bit writing,
compress_into orchestration, full raw-output/decompression correctness, and the
full input measurement remain outstanding. No optional performance tuning is
included in this diagnostic policy change.
