# R262: retest packed scalar choice after native outlining

**Reject this candidate and retain repaired R261.** The same idea can behave
differently after other compiler changes. This retest removes the old stack-frame
growth, but complete compression still becomes 2.870005% slower across all 81
cases. The retained compressor executable and source algorithm do not change.

Published parent: `56f7a4dd0bcffdce20be6ac4e5b62dd8fa1fb87d`.
PR base: `92f75db84e2523693648084bea16cd52ad60777a`.
The exact-head review completed in comment 6773197869 before this experiment.
The experimental source starts from R261's repaired 825-file closure, modifies
two existing files and reverses exactly to that baseline. IR/work/ABI/bytecode
remain 215/107/39/14; the R255 backend and dependencies remain pinned.

## Why repeat this old experiment?

R187/R188 packed a U32 minimum and its winning index into one U64. That reduced
the inner loop but increased broader costs in a multi-megabyte generated entry.
The current path computation occupies a 2048-byte native phase with a 288-byte
frame. That architecture change makes a new measurement worthwhile.

This is R187 packing alone. Current I32 scalar-cost evaluation, vector emission,
thresholds and selected-item evaluation remain unchanged. Existing admission
proves `cursor < stop <= U32MAX`; cost occupies high32 and index+1 occupies low32.
Low zero preserves the incoming item on equal costs. Increasing indices preserve
the earliest winner, including a preceding vector-prefix winner. Current source
checks, funding, refusal paths and terminal semantics remain intact. Selection
uses the generic ordered-reduction proof, without compressor names or input-family
tests. The private diagnostic flag is `MECH_PACK_SCALAR_CHOICE=1`.

## Complete-call measurements

Two fresh balanced cohorts use all six actor orders, 48 total blocks per case,
matched 200-ms warmups, level 12 and runtime input. Ratios are calculated inside
each balanced block. All observations, contrary cases and original gates remain.

| Input | Bytes | R261 / Roc | Candidate / Roc | Candidate / R261 |
|---|---:|---:|---:|---:|
| Eight literals | 8 | 0.422043x | 0.436717x | 1.034767x |
| Natural text | 32768 | 1.838396x | 1.846710x | 1.004522x |
| Random | 32768 | 1.827117x | 1.896943x | 1.038216x |
| Periodic | 32768 | 1.094608x | 1.138223x | 1.039846x |
| Repeated | 32768 | 1.059643x | 1.098062x | 1.036256x |
| Natural text | 1048576 | 1.906580x | 1.907909x | 1.000697x |
| Random | 1048576 | 2.168785x | 2.254190x | 1.039379x |
| Periodic | 1048576 | 1.268420x | 1.332019x | 1.050141x |
| Repeated | 1048576 | 1.225070x | 1.280785x | 1.045479x |

The unchanged 1.05 gates pass 76/81 pooled-median and 77/81 block-median cases.
Pooled misses: eight-literals-prefix, repeated-1024, random-262144,
periodic-262144 and repeated-262144. Block misses: random-64 and the three
262144-byte cases. No allowance is increased. Fresh reference denominators do
not replace historical tables or imply a change to the retained compiler.

Complete compression, scratch construction, output transfer and private cleanup
remain inside the timer; prepared input, validation and external output-slot
lifetime stay outside on both sides. Source algorithms, tables, depth 300,
nice length 258, EOF limits, allocation sites, real tree/cache/path capacities,
output clear/reuse and pinned allocator remain unchanged. The compact timing
entry still omits the additional ordinary managed admission/accounting/
publication/recovery contract.

## Why fewer inner-loop instructions did not help

The actual continuing scalar route shrinks **14 to 12 instructions**. The eight
changed path definitions grow 2048 to 2064 bytes each, with frames still 288
bytes. Among 341 root/numeric definitions, 333 retain identical Object bytes;
root code/frame and linked worker size remain unchanged. Unrelated linked
addresses move, so no fixed-placement attribution is claimed.

The empty-match outer route grows **39 to 44 instructions**. Register allocation
adds current cursor and invariant-home stack traffic even though that route
never enters the scalar minimum loop. The historical unchanged-source current-cache
census derives 2,991,660 such visits for random 1 MiB. Five extra instructions
expose 14,958,300 instructions; the fresh whole-call increase is 15,227,313. This
supports the transport explanation without claiming a complete event partition
or a fresh R262 census.

Natural 1 MiB instructions increase 2.0086%; random increases 1.5414%; periodic
increases 1.2013%. The smaller continuing loop is outweighed by other emitted
work. Eight literals have no eligible scalar visits, nearly equal instructions
and an improved separate cycle cohort despite worse primary elapsed; its cause
remains unproved. Every contrary measurement is preserved.

All 81 allocation, requested-byte, initialized-copy, zeroing and private-peak
families remain equal. Final nonreturned private bytes remain zero. Hardware and
heap cohorts are separate from primary elapsed and from CPU sampling.

Eighteen fresh captures cover R261, candidate and Roc on six bounded inputs,
using each exact current image. On natural 1 MiB the path phase has 57.93% of
R261 running samples and 57.99% of candidate samples; on random it rises from
18.34% to 21.14%. These are CPU-time sample shares, not instruction counts or
causal elapsed budgets. Root source attribution remains explicitly unknown.

## Qualification and retained limits

- 716 candidate native tests pass with seven unchanged inherited exclusions.
  Ten focused tests pass, including 12,288 compiled-helper boundary/tie cases.
- 160 original/decoded path calls agree with the unchanged independent oracle
  and hash-pinned prior resident receipts. Current arguments, invalid windows,
  errors, aliases, unchanged publication, cleanup and retry remain covered.
- 324 complete original/decoded smokes pass all 81 exact streams, independent
  decompression and alias/cleanup checks. Both enabled images match exactly.
  Disabled compilation restores the complete R261 Object and metadata exactly.
- Test roots use opt-level 1; the performance compiler library uses opt-level 3,
  with warnings-as-errors and the same six native features. Three tracked
  architecture/catalog checks and changed-module formatting pass.
- The unchanged maintained 680-test matrix is inherited from R261. Full CI,
  fresh full linked/AOT integration and the full 6.6-MB input are not claimed.
- An initial cohort-A setup mistook copied profiler-duration estimates for
  completed resume state and executed zero timing observations. That directory
  and receipt remain. Valid cohort B and counters continued untouched; A then
  executed all 81 cases fresh. Actual order is B, CPU, heap, A. Every final
  cohort has all 81 observation files; no valid observation was discarded.
- One candidate-profile Boolean label was inherited incorrectly from the
  baseline helper; its correct worker/Object hashes and all observations remain.
  A sidecar records the clarification. R261's prose also mislabeled test-root
  optimization level; original receipts were always correct and remain immutable.

No introduced functional failure remains. The introduced performance regressions
and gate misses prevent adoption. Retain R261 and investigate a current redundant
narrow-extension instruction without changing loop live ranges before another
compiler experiment.

## Evidence and replay

[Full source, reversible patch, all 81 rows, profiles, counters and commands](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r262-scalar-choice-retest-b842a1a42f30.tar.gz)
contains 1,907 hash-verified files and five offline reports, with native binaries,
bytecode, full compressor metadata/IR/maps and raw trace/XML/stacks excluded.
SHA-256: `b842a1a42f307c1ec2b253b8d3be957d335aeb2137ffe5723960bbc7c1b04c6b`.

```sh
python3 benchmarks/mech-deflate/experiment.py replay-round \
  --index benchmarks/mech-deflate/experiments.json --round 262 \
  --output /tmp/mech-r262-review
```

Replay verifies the complete archive, reconstructs the compiler in both
directions and reproduces five reports. It does not compile or compress.
