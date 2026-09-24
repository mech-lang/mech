# EKF source-size audit and agent handoff

Start with [HANDOFF.md](HANDOFF.md) for the next agent's task.

![EKF application source size](ekf-source-size-reconciled.svg)

[Why Rust is longer](proof/README.md) · [Character breakdown](proof/breakdown.json) · [Four chart values](counts.csv) · [Exact selections and provenance](manifest.json)

Counted source: [Mech, both columns](selected/ekf.mec) · [Rust textbook](selected/rust_textbook.rs) · [Rust SIMD-4/eight-worker](selected/rust_simd.rs).

The following records preserve the reconciled source comparison. The original
benchmark source paths are unchanged by this branch.

## Reconciled comparison

| Language | Textbook | Four-wide SIMD / eight workers |
|---|---:|---:|
| Mech | 1,079 | 1,079 |
| Rust | 2,355 | 5,243 |

Unit: normalized source characters (Unicode code points).

## What was kept and what was changed

The eight-worker pair reproduces the supplied audit exactly, including its
four-character-per-name sensitivity check. The selected Mech source is the
benchmark's full `ekf-kernel-taichi-comparable.mec` fixture. The selected Rust
SIMD path retains its own V4 adapters, matrix helpers, scoped-thread dispatch,
checkpoint, fault metadata, initialization, divisibility precondition, and
block rollback. It has not been replaced by a shorter Rayon implementation.
Both Mech rows reference the same selected source and the same SHA-256.

The Rust textbook row is NEWLY publication-normalized source, derived from the
archived `minimal/rust_scalar.rs`. It is not the original unchecked or lane-local
scalar count. Only the following parts change:

- A block-start checkpoint and complete state/covariance rollback are added.
- A failed candidate terminates the block instead of continuing and retaining
  updates to other filters.
- The existing three validation predicates return corresponding fault categories
  (finite = 1, positive diagonal = 2, covariance symmetry = 3).
- Fault metadata carries the turn, instance, and constraint, as in the SIMD path.

`evidence/textbook-contract.patch` contains every change to that scalar extract.
The numerical body, hand-written matrix helpers, reset function, and initial
allocation statements remain byte-for-byte unchanged. The source checks verify
those invariants. Positive diagonal validation is not a positive-definiteness
proof. Floating-point operation association and diagnostic ordering need not
be identical between these source implementations.

This is a source comparison. Neither the chart nor this package assigns a
throughput to the normalized textbook source. The metric describes these
specific implementations; it does not mean all Rust implementations must grow
by this amount when optimized.

## Application contract and counting boundary

Inputs are supplied arrays of velocity, angular velocity, and bearing, together
with population and block length. The comparison includes constants, initial
state, f32 bearing-only EKF prediction/correction, Joseph covariance update,
finite/positive-diagonal/symmetry predicates, and application-owned execution
and publication support. The publication-normalized Rust scalar application
and the original optimized Rust path both restore the complete population to
the block-start checkpoint when any checked update fails; the caller observes
only the completed block. The Mech application remains the same declarative
program: generic backend selection and block execution/publication are runtime
responsibilities, not an added hand-written Mech EKF implementation. The Mech
scalar `dispatch_turns` interface, which publishes per turn, must not be relabeled
as the audited fused block interface merely because its source input is the same.

Excluded on both sides: synthetic benchmark inputs, command-line parsing,
timers, warmup, checksums, printing, tests, and general-purpose language/library/
compiler/runtime internals. The unused Rust nonfused dispatcher is excluded.
Initial allocation and population validation are retained even though they
come from the harness's `main`. Configuration parameters such as block length
and worker count are supplied at the application boundary, as in the original
audit. Both checked and unchecked branches in the selected path stay counted.

These files are source-counting extracts, including initialization statements
extracted from main; they are not presented as complete standalone projects.

## Name and whitespace normalization

Each programmer-chosen name occurrence contributes one code point, regardless
of original spelling length. This includes custom type, field, function, local,
parameter, and constraint names. All language keywords, primitive types,
library API names, trait-required method names, punctuation, numbers, and
literal contents retain their actual lengths. Rust attributes are retained.
Whitespace outside literals and comments are excluded. Document underlines
are excluded by selection; the compute directive and its section name are kept.

The multiword semantic section name `EKF step` counts as one chosen name, but
`@compute` retains its literal spelling. The classifier distinguishes custom
V4 methods from same-named scalar/library methods and the scope binding from
`std::thread::scope`. It is an explicit source-specific classifier, not a
claimed general-purpose parser. The files in `normalized/` contain an audit
stream and a per-token TSV; the audit stream is not executable source.

## Sensitivity to identifier width

| Selected source | Width 1 | Width 4 | Original names, no trivia |
|---|---:|---:|---:|
| Mech | 1,079 | 1,628 | 2,913 |
| Rust textbook, publication-normalized | 2,355 | 3,390 | 4,142 |
| Rust SIMD / eight workers | 5,243 | 7,196 | 8,711 |

## Provenance and exact selection

Repository: https://github.com/mech-lang/mech
Revision: d7c535bcccbde027db9dff3e6dc1bb77ed7be05b

Original Mech: `hosts/gpu/fixtures/ekf-kernel-taichi-comparable.mec`
Blob: `6658d76c331b52303b64d9e94fdf328e4ed2b709`
Selection: lines 1, 16, 19–103.

Original SIMD Rust: `benchmarks/archive/compute/parallel-ekf/minimal/rust_simd.rs`
Blob: `b00fa105b195e8231a2efb8667d7adc4ca684d99`
Selection: instrumentation-free imports `use std::slice; use wide::f32x4;`,
then lines 3–245, 259–272, 325–459, 476–479, and 481–483.

Original textbook Rust: `benchmarks/archive/compute/parallel-ekf/minimal/rust_scalar.rs`
Blob: `5a5fcd7d35786adee9766109ccb454de4f03aca6`
Initial selection: lines 2–17, 96–248, 23–26, in that order.
Then apply the explicit publication/fault correction in `prepare_sources.py`.

All three complete originals match their Git blob hashes. `measure.py` verifies
the originals, regenerates the selections and patch, and recomputes all totals.

## Reproduce

Python 3.10 or newer, standard library only for counting:

```
python measure.py
python -m unittest -v test_measure
```

For the chart, install matplotlib and run:

```
python plot.py
```

`counts.csv` contains the four chart values. `counts.json` contains independent
source totals, sensitivity counts, and SHA-256 digests. The graph reads this
file directly. PNG and PDF are rendered artwork; SVG contains editable text.

## Reproduce the explanation

After the counting commands above, run:

```sh
python breakdown.py
```

This writes `proof/breakdown.json`, `proof/breakdown.csv`, the exact code excerpts,
and per-token category logs. Each counted token belongs to exactly one category.
`normalized/` and the large categorized token logs are generated locally rather
than checked in. All reproduction paths are relative to this audit directory;
no ChatGPT sandbox or downloaded attachment is required.

The checked-in SVG is the existing chart from the reconciled package. `plot.py`
regenerates SVG, PNG, and PDF using matplotlib. No generated poster image is
included: its invented code panel is not an authoritative Mech example.
