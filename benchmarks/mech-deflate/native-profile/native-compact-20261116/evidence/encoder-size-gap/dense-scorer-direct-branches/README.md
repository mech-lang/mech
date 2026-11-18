# Direct branches in the checked dense scorer

The LLVM-free opt-in recipe emits **66,192 B** of encoder text. It saves **1,456 B** from the qualified 67,648 B build, and emits **1.147× Roc** and **1.785× libdeflate** code.

The layout bypasses branch-only blocks and lets the scalar reduction jump directly to the loop header. The dense scorer also selects compare-and-branch instructions for integer comparisons against zero. The lowerer keeps the original immutable table and checked continuation for inputs outside the dense admission bounds.

The linked worker matched **162 successful observations and six checked failures**. Peak stack was **4,640 B**. The vendored backend unit suite passed 212 tests.

[Compiler source patch](compiler.patch.gz) · [Linked validation](linked-checks.json) · [Backend test log](backend-tests.log)

## Compression timing

Both full orders use 96 balanced blocks per case. Timing runs through private heap cleanup.

| Order | Roc wins | libdeflate wins | Natural 8 KiB vs libdeflate | 320 overlap vs libdeflate |
| --- | ---: | ---: | ---: | ---: |
| Forward | 81/81 | 62/81 | 0.98198× | 0.98941× |
| Reverse | 81/81 | 62/81 | 0.99134× | 0.99078× |

V3 returned **Retain**.

[Full replay](full-replay.json.gz) · [Forward observations](full-forward-observations.tar) · [Reverse observations](full-reverse-observations.tar)

## Executed and emitted instructions

| Case | Instructions before | Instructions after | Unconditional jumps before | After |
| --- | ---: | ---: | ---: | ---: |
| 320-overlap | 36,372 | 33,772 | 1,308 | 1,292 |
| prefix-natural-8192 | 7,454,886 | 7,205,976 | 163,872 | 20,202 |

A separate instrumented replay measured these counts. The timing worker is uninstrumented.

[Exact branch counts](branch-census.json) · [Training and held-out checks](scorer-prediction.json)

## Fresh builds through final link

Apple M1 ran 8 balanced trials per build. Each compiler and fixed deployment support are prebuilt.

| Build | Fresh source through link | Stripped executable |
| --- | ---: | ---: |
| Qualified Mech | 2.279 s | 276,056 B |
| Smaller Mech | 2.274 s | 276,056 B |
| Roc | 2.986 s | 191,912 B |
| libdeflate | 0.481 s | 155,752 B |

Paired build time is **0.762× Roc** and **4.721× libdeflate**.

[Matched build and phase timings](deployment.json)

## Source replay

Apply the compiler patch in an isolated checkout of `648a7359293d43c5d4d5288e5a80a3f382b6a4fc`. Rebuild using the frame-copy variant, compiler optimization level 3, direct source, preselected root inlining and skipped refused general SSA. Run replay.py with the baseline rebuild directory and a fresh output path. The replay checks the encoder object, metadata and original canonical bytecode.

[Baseline rebuild command](../shared-continuation-returns/README.md#replay) · [Recipe flags and hashes](candidate.json) · [Verified replay](verified-replay.json)
