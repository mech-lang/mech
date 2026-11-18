# Control-flow reuse during native cleanup

Source `204656da92013ce24dabb115685a75fc930c7c46` verifies the pruned function using analyses built
in the same cleanup call when no blocks were removed. Other entries and
pruned layouts still make an independent graph comparison. After constant
phi edits, the complete IR verifier uses the dominators from the first
verification. Phi edits preserve ordered successors, branch instructions
and block layout. The standalone verifier and standalone constant-phi pass
retain their independent checks.

| Apple M1 | Mech | Roc | libdeflate |
| --- | ---: | ---: | ---: |
| Source through final link | 2.57808 s | 2.99395 s | 0.48620 s |
| Source through strip | 2.58878 s | 3.00439 s | 0.49685 s |
| Encoder text | 71,192 B | 57,696 B | 37,080 B |
| Stripped executable | 292,568 B | 191,912 B | 155,752 B |

The confirmation includes eight fresh-process orders and reverses the
initial warmup and trial rotations. It saves **33.8 ms**
through linking and **34.3 ms** in
the native phase. The first series saves a paired **13.6 ms**
through linking, including two slow candidate trials with large frontend
spikes. Both complete series reproduce the bytecode, object, metadata and
stripped executable. Arithmetic uses paired medians from each series.
[Initial plan](initial/plan.json) · [Initial observations](initial/observations.json.gz) ·
[Confirmation plan](confirmation/plan.json) · [All confirmation observations](confirmation/observations.json.gz)

Four focused checks exercise pruning, no-op and mutating phi sweeps, verify
on and off, malformed IR and stale successor order with an unchanged edge
set. The rebound worker passes 162 checks and is identical to the selected
71,192 B worker. Its existing full runtime orders are **81/81 Roc and
61/81 libdeflate** in each direction. The 62-win libdeflate threshold
remains open.
[Verifier checks](checks.json) · [Worker identity](worker-binding.json) ·
[Full runtime orders](../frame-copy-v2/README.md)

## Diagnostic cleanup census

The diagnostic compiler independently verifies each completed cleanup and
compares the exact ordered block, terminator and successor sequence across
the phi sweep. It also compares that sequence across pruning when no blocks
were removed. It emits the same selected artifacts. Its elapsed time is
outside the matched measurement.

| Measured work | Inside root cleanup, including nested calls | All functions |
| --- | ---: | ---: |
| Combined cleanups | 62 | 252 |
| Prune-sweep block visits | 256,180 | 352,749 |
| Blocks removed | 6,355 | 14,587 |
| Instructions scanned by phi collection | 503,331 | 658,463 |
| Phi parameters removed | 41,669 | 49,865 |
| First verifications using the freshly built graph | 48 | 104 |
| Post-phi verifications reusing analyses | 21 | 91 |

[Per-call counts](diagnostics/summary.json) · [Earlier call timings](../native-pass-attribution/README.md)

## Current parser and runtime targets

The separate parser census preserves the same 718,353 events, 3,967,302
parser steps and lossless tree. It identifies 138,884
eligible rejected exact terminal arms in 21,543
adjacent batches. Identifier-symbol choices account for 134,387
of those arms. Test that family with a first-byte selection plan derived
from the canonical terminals. The plan needs identical allowance charges,
rewind accounting and cache behavior. The census measures work counts.
[Parser observations](parser-choice-census/report.json)

On the identical worker, the closest missed libdeflate case is
`prefix-natural-8192`, at 1.035×
and 1.041× in the two full orders.
It needs about 3.9% less elapsed
time in the slower order. The current matchfinder and composed scorer
bodies match their archived instruction-counted atoms, including named
relocations. Start with the scorer's repeated work on this input. The
archive measured about 7.75 million scorer instructions in six calls.
[Exact worker margins and atom bindings](runtime-target.json) ·
[Prior instruction counts](../match-cache-partition-cause/README.md)

Rebuild the maintained compiler and encoder with the frame-copy recipe in
[CURRENT.md](../../CURRENT.md). The plans identify prebuilt host and
allocator inputs, compiler hashes and the exact source-through-link
boundary. At a different checkout, substitute matching inputs at their
new local paths.
