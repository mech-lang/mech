# Why the counted Rust source is longer

This report partitions the three selected source extracts from the parent audit directory without editing them or changing the existing counter. The same Mech file supplies both chart entries. One programmer-chosen name occurrence contributes one code point; other non-trivia source retains its literal length.

## Exact partition

| Category | Mech (either strategy) | Rust textbook | Rust SIMD-4 / eight workers |
|---|---:|---:|---:|
| Setup, constants and declarations | 340 | 486 | 561 |
| EKF update and local candidate handling | 356 | 1,024 | 1,119 |
| Validation and component extraction | 383 | 263 | 396 |
| Application-written matrix helpers | 0 | 251 | 310 |
| SIMD wrapper and adapters | 0 | 0 | 1,243 |
| Batch dispatch, workers and rollback | 0 | 331 | 1,614 |
| Total | 1,079 | 2,355 | 5,243 |

Every counted token is assigned to exactly one category. `breakdown.json` records the exact selected-file line spans and hashes, and the categorized token TSV files expose each contribution. These are source-organization categories, not a minimal causal decomposition: e.g. constants inside Rust step functions count with those functions, whereas top-level Mech constants count with setup. A zero means no separate application-written function/block in the selected Mech extract; it does not mean the corresponding runtime/library operation is absent.

## Scalar-to-SIMD growth

The increase is 5,243 - 2,355 = 2,888 characters. The SIMD wrapper/adapters add 1,243, and the dispatcher grows by 1,614 - 331 = 1,283. Together these account for 2,526 / 2,888 = 87.4654% of the increase. The EKF step including local candidate handling grows by 95.

## Direct equation example

Mech selected lines 60-62 express corrected state, the correction matrix, and Joseph covariance. They cost 38 normalized characters. Rust textbook selected lines 128-146 express the same stages with arrays, loops, helper calls and output buffers; they cost 240. These excerpt subtotals exclude validation and both implementations' surrounding declarations. Rust's helper definitions are counted elsewhere, not included in the 240.

## Implementation-choice component

The five forwarding Add/Sub/Mul/Div/Neg implementations in the SIMD wrapper (selected lines 57-91) cost 480 normalized characters. They forward to operators on the inner wide::f32x4 value. They are required by this wrapper design, not evidence that Rust lacks SIMD arithmetic operators. wide 0.7.33 already implements these operators:
https://docs.rs/wide/0.7.33/wide/struct.f32x4.html

Likewise, matrix helpers are handwritten in the selected Rust application, whereas a Rust matrix-library implementation could delegate them to a library. nalgebra's official guide documents matrix operators and fixed-size matrix types:
https://www.nalgebra.rs/docs/user_guide/vectors_and_matrices/

This report does not remove those helpers, substitute a new implementation, or claim a new whole-program size after hypothetical refactoring. It explains the existing totals. The evidence supports a source-size result for these implementations, not a lower bound for Rust or a universal requirement that faster Rust must be longer.

## Reproduce

Python standard library only:

```
cd benchmarks/archive/compute/parallel-ekf/source-size-audit
python measure.py
python -m unittest -q test_measure
python breakdown.py
```

The existing counter reproduces 1,079 / 2,355 / 5,243, and its 17 tests pass. The breakdown additionally checks that the selected-file hashes match counts.json, assigns each counted token exactly once, and asserts that category sums equal the unchanged totals.
