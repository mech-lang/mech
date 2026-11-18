# Select the checked default address

The checked entry folds 65 defaulted unsigned loads. It selects the original address when the unsigned bound succeeds and an aligned read-only zero otherwise. The narrow load retains its original flags and the source arithmetic keeps wrapping semantics.

The opt-in LLVM-free encoder emits **65,612 B**, saving 320 B. Encoder text is **1.137× Roc** and **1.769× libdeflate**.

Linked validation matched 162 successful observations and six checked failures with 4,784 B peak stack. The backend regression exercises the checked default with a null source address and a maximum unsigned index.

[Size comparison](size.json) · [Linked checks](linked-checks.json) · [Backend test](backend-test.log.gz)

## Full compression runs

| Order | Roc wins | libdeflate wins | Natural 4 KiB vs libdeflate | Natural 8 KiB vs libdeflate |
| --- | ---: | ---: | ---: | ---: |
| Forward | 81/81 | 62/81 | 1.07310× | 0.98008× |
| Reverse | 81/81 | 62/81 | 1.07227× | 0.98312× |

Both orders use 96 balanced blocks per case, from compression through private heap cleanup. V3 returned **Retain**.

[Replay](full-replay.json.gz) · [Forward observations](full-forward-observations.tar) · [Reverse observations](full-reverse-observations.tar)

## Fresh source through final link

Mech builds in **2.205 s**, or **0.740× Roc** and **4.588× libdeflate**, over eight balanced trials on Apple M1. Compilers and fixed deployment support are prebuilt.

The compiler patch includes the short SIMD cutoff probe. The qualified recipe uses the established 16-element cutoff. [Eight-element cutoff screen](vector-8-focus-screen.json) The Tail preparation probe reproduced the exact worker and measured 0.8 ms slower through link in the median pair. It was faster in 3/8 builds. [Preparation timing](preparation-timing.json) · [Exact replay](preparation-exact-replay.json)

[Matched measurements and phase medians](deployment.json) · [Raw builds](build-comparison.json.gz)

## Replay

Apply the compiler patch in an isolated checkout of `1e95a1634f5f06a6264f90b532dbea28c1391968`. Rebuild with the frame-copy variant at compiler optimization level 3 and the documented direct-source flags. Run replay.py with that rebuild directory and a fresh output path.

[Compiler patch](compiler.patch.gz) · [Rebuild command](../shared-continuation-returns/README.md#replay) · [Exact recipe and checksums](candidate.json)
