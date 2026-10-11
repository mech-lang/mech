# N-body scheduling and register-pressure experiments, 2026-10-11

The best measured setting gives a small repeatable improvement: 24-instruction
scheduling regions reduce fast Mech from 1.041377 s to
1.039063 s (0.9978× its previous time, about 0.2%).
Both are the same 25-LOC pure Mech source. Compact retains its existing
8-instruction setting and 20 LOC. This is an experimental result; the published
full-suite chart has not been replaced by this cohort.

Two independent 15-round cohorts confirm the small effect. Their paired geometric
ratios are 0.998038 and 0.997903;
bootstrap 95% intervals are [0.996703, 0.999426] and
[0.996186, 0.999747]. These describe the sampled rounds
on this machine, not a universal gain across systems or host conditions.

## Final matched measurements

| Program | Seconds | Sample SD, s | Time × Mech | LOC | LOC × Mech |
|---|---:|---:|---:|---:|---:|
| C++ #6 | 0.938208 | 0.002086 | 0.9029 | 202 | 8.08 |
| Chapel #3 | 0.979831 | 0.001357 | 0.9430 | 79 | 3.16 |
| Mech, 24-instruction regions | 1.039063 | 0.003159 | 1.0000 | 25 | 1.00 |
| Mech, previous 8-instruction regions | 1.041377 | 0.002336 | 1.0022 | 25 | 1.00 |
| Zig #2 | 1.045138 | 0.004051 | 1.0058 | 117 | 4.68 |
| Mech compact, 8-instruction regions | 1.090733 | 0.005469 | 1.0497 | 20 | 0.80 |
| Rust #3 | 1.124497 | 0.001941 | 1.0822 | 196 | 7.84 |

Ratios use the 25-LOC Mech candidate in this cohort. All results include fresh
process startup, initialization, 50 million full steps, both energy calculations,
output and shutdown. Compilation, validation and full-workload warmups are
excluded. Timings run sequentially in shuffled rounds on AC power with Low Power
Mode off; compiler activity and power changes are monitored. No samples were
discarded. Confirmation and replication use different shuffle seeds.

Zig's ordering relative to unchanged Mech also moved since earlier cohorts. Do
not attribute that whole ranking change to this 0.2% optimization, or splice
these raw seconds into historical results without their matched controls.

## What changed in native code

| Fast loop, per step | Previous regions 8 | Candidate regions 24 |
|---|---:|---:|
| Static instructions | 276 | 268 |
| Stack loads | 51 | 48 |
| Stack stores | 23 | 20 |
| Fused arithmetic instructions | 60 | 60 |
| Vector square roots | 5 | 5 |
| Vector divisions | 5 | 5 |

There are six fewer stack-memory instructions per step (8.1% fewer). These are
static instruction counts, not a measurement of heap memory or a complete cycle
model. All ten pair distances still execute, with two lanes per vector operation.
No approximation, work removal or specialized host kernel was introduced.
The small time improvement despite fewer spills shows that spill count alone
cannot predict the remaining performance gap.

## Trials, including failures and no-ops

- A backward scheduler minimized the live frontier from outputs and favored
  finishing a dependency branch. It gave no unique confirmed win.
- A latency scheduler tracked estimated operand completion and placed ready
  independent work between long operations. Many outputs matched the existing
  scheduler's native objects exactly.
- A final-use-first scheduler gave register retirement priority over dependency
  height. Its tested native objects matched existing scheduling choices.
- Deferred scalar extraction kept SIMD values packed until scalar use sites.
  Cranelift produced byte-identical objects for both n-body forms at each tested
  region size; it already removed this proposed overhead.
- Composite inverse-cube expansion preserved sqrt/product/divide association.
  It is a no-op for these sources because they already express those operations
  separately. Integration tests also exercised unrelated graphs where expansion
  does apply.
- Region sizes 8, 12, 16, 24, 32 and 48 were explored for the fast form. Compact
  also exercised the wider regions; its existing size 8 remained preferable.

The experiments generated 55 Mech manifest entries, including existing controls
and reproduction checks, mapping to 16 distinct objects. Only distinct relevant
objects proceeded to the second pilot. The four timing reports retain all
330 process samples and their stdout validations. Byte-identical groups are in
[equivalent-objects.json](equivalent-objects.json).

All new compiler and integration-test edits were restored exactly from their
pre-experiment snapshots. The winning object is reproduced by the original
compiler using its existing region-size option; [restored-object-verification.json](restored-object-verification.json)
records that check. There is no added production compiler complexity from these
trials. Archived prototype sources and frozen generation drivers remain available.

## Correctness and reproduction

The prototypes passed 33 fused-state and 14 map integration tests, including
perturbed initial values, unrelated recurrences, scalar/SIMD equivalence, signed
zero and complete Mandelbrot bitmaps. Each timed Mech candidate was checked
against its control across all 30 state values at 0, 1, 1,000 and 10,000 steps;
the final candidate is bit-identical. Every full-workload sample validates both
initial and final energies. The unchanged compiler has exact source hashes in
[restored-source-verification.json](restored-source-verification.json).

A supplementary full-workspace unit-test build was stopped while compiling the
unrelated full-feature math dev dependency; no result is claimed for those unit
tests. The completed native integration tests are the validation reported above.

Candidate flags:

```text
--fp-contract --algebraic-powers --pack-state-tails
--simplify-square-seeds --schedule-regions 24
```

Re-run the matched cohort with a fresh output filename:

```sh
python3 replicate_again.py --programs confirmation-programs.json --samples 15 --output /tmp/nbody-scheduling-fresh.json
```

The compiler is Cranelift AOT; Clang links the generic process runner. All
benchmark arithmetic is lowered from the unchanged Mech source. Raw data:
[pilot](pilot-50m.json), [deduplicated sweep](sweep-50m.json),
[confirmation](confirmation-50m.json), [replication](replication-50m.json).
[summary.json](summary.json) pins sources, reports and compiler snapshots;
[matched-mech.csv](matched-mech.csv) includes the timing and LOC multipliers.
