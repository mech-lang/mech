# Handoff: EKF source comparison and poster evidence

## Task

Continue the Mech/Rust **application source-size comparison**, and use the
actual source excerpts and chart to explain it on the robotics poster. This is
not a request to develop or report a new throughput comparison. Work in this
isolated branch; production compiler/runtime changes and merges are outside
this handoff.

Branch: `codex/ekf-source-size-audit` in `mech-lang/mech`.
Base: `d7c535bcccbde027db9dff3e6dc1bb77ed7be05b`, the archived benchmark revision
named in the source audit, not the current v0.4 cutover stack.
Directory: `benchmarks/archive/compute/parallel-ekf/source-size-audit/`.

## Current four chart values

| Normalized source characters | Textbook | Four-wide SIMD / eight workers |
|---|---:|---:|
| Mech | 1,079 | 1,079 |
| Rust | 2,355 | 5,243 |

There are three distinct counted source files, not four. Both Mech bars
reference the exact same `selected/ekf.mec`. The SIMD Rust is the original
archived application extract. Textbook Rust has the explicit
`evidence/textbook-contract.patch` for block-start checkpoint, complete rollback,
terminal rejection, and corresponding fault metadata. Do not describe the
2,355 count as the unmodified archived scalar baseline.

The earlier 1,062 / 2,579 / 3,628 counts used different rewritten sources and
are superseded. Do not combine them with this audit. Do not silently replace
the SIMD source with a Rayon rewrite or a different matrix-library program.
Alternative implementations are valid new comparison cases, but must receive
their own provenance, source boundary, counts, and labels.

## What explains the difference

Read `proof/README.md` and `proof/breakdown.json`.

- Rust grows by 2,888 normalized characters from textbook to SIMD.
- SIMD adapters add 1,243; the dispatcher grows by 1,283. Together these
  explain 2,526 / 2,888 = 87.4654% of the growth.
- The EKF step including candidate handling grows by only 95 characters.
- The exact correction-equation excerpts cost 38 in Mech and 240 in textbook
  Rust; surrounding helpers are counted separately, not in that excerpt.
- Five Rust arithmetic trait-forwarding implementations cost 480. They are
  a choice of this application's V4 wrapper design, not a universal Rust cost.

These categories partition the existing source, rather than proving minimal
necessary implementation sizes. Zero Mech application characters for a
separate helper/dispatcher do not mean the library/runtime contains no such
implementation. The result is specific to the selected programs.

## Counting rules that must stay visible

Count one Unicode code point per programmer-chosen name occurrence, in both
languages. Thus `control-jacobian` contributes 1 on every occurrence. Numeric
literals keep their complete spellings: `0f32` contributes 4; `0.0_f32`
contributes 7. Keywords, external library APIs, trait-required method names,
punctuation, attributes, and literal contents retain their actual lengths.
Comments and whitespace outside literals contribute zero.

The metric covers application-authored numerical code, constants, initialization,
validation, SIMD adapters, packing, dispatch, and publication support. It
excludes synthetic benchmark inputs, timings, command-line parsing, warmup,
checksums, reporting, tests, and general library/compiler/runtime internals.
Both selected checked and unchecked branches are retained. The classifier is
explicit and source-specific, not a general Rust/Mech semantic parser.

The Rust contract correction uses block-start rollback. The generic Mech
scalar per-turn publication API is not thereby a block-atomic API; source
identity and execution-interface identity must remain distinct.

## Reproduce before changing the presentation

From the repository root:

```sh
cd benchmarks/archive/compute/parallel-ekf/source-size-audit
python3 measure.py
python3 -m unittest -v test_measure
python3 breakdown.py
# Optional chart regeneration (requires matplotlib):
python3 plot.py
```

The first three commands use Python's standard library. `measure.py` verifies
original Git blob hashes, rebuilds the extracts and patch, and emits token
audits. `breakdown.py` checks extract hashes and category totals and
reproduces the code-excerpt subtotals. The existing 17 tests protect counting
and source-preservation behavior; they are not runtime qualification claims.

## Poster requirements

Use Mech yellow `#F4C430` and Rust tan `#DEA584` for the source chart. Keep
academic prose direct; explain the actual source differences instead of
claiming all faster Rust programs must be larger.

Keep the exact title:

> A Rust-Native Embeddable Reactive Numerical Language for Heterogeneous Computing in Robotics

Keep the authors:

> Corey Montella, Aung Kant Ko, Steven McPhillimey

Preserve Mech and Lehigh branding. The previous image-generated poster has
invented Rust-like code, a four-component state, and an incorrect covariance
update. Do **not** reuse that code panel. Use exact excerpts from
`selected/ekf.mec`: the actual state has three components and the source uses
the Joseph covariance update. Draw chart values from `counts.json`, not an
image generator's interpretation.

Useful next presentation: show the real Mech correction expressions alongside
the corresponding Rust excerpt, and pair the four-bar chart with the measured
SIMD-adapter/dispatcher breakdown. Preserve the implementation-specific scope
and the textbook publication-correction note in the caption or linked audit.
