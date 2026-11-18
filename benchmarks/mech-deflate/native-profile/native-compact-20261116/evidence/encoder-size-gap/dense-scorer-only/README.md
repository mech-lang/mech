# One dense scorer with a checked fallback

The LLVM-free opt-in configuration emits **66,240 B**, a reduction of **1,408 B** from the qualified 67,648 B build. Encoder text is **1.148× Roc** and **1.786× libdeflate**.

The scorer uses its proved immutable code table for both small and large admitted inputs. The visit-count threshold covered preparation of the mutable lookup cache. The dense body needs no such preparation. Cursor, capacity and cost bounds still guard the call. The original checked source continuation handles refusal before the synchronous packet is written. The compiler omits 1,328 B of standalone sparse scorer text. The checked entry shrinks by 80 B.

[Exact size comparison](comparison.json) · [Compiler patch](compiler.patch.gz)

The lowerer verified the same three checked controller groups and five continuation families after mapping the changed graph. The linked checks cover 162 successful observations and six checked failures. Peak stack is 4,640 B, compared with 4,752 B before this work.

[Controller map](layout_controller-map.json) · [Continuation map](continuation-recovered.json) · [Linked checks](linked-checks.json)

The original 320-byte overlap fixture called the sparse scorer 4 times. The new worker enters the dense scorer the same number of times. Separate instrumentation counts the actual route on six fixtures.

[Instrumented call counts](route-counts.json)

## Compression timing

Each case uses 96 balanced blocks. The timing boundary includes private heap cleanup.

| Scope and order | Roc wins | libdeflate wins | Natural 8 KiB vs libdeflate | 320 overlap vs libdeflate |
| --- | ---: | ---: | ---: | ---: |
| Focus forward | 15/15 | 13/15 | 0.99605× | 0.99956× |
| Focus reverse | 15/15 | 13/15 | 0.99157× | 0.99605× |
| Full forward | 81/81 | 62/81 | 0.98954× | 0.99850× |
| Full reverse | 81/81 | 61/81 | 1.00140× | 0.99973× |

V3 returned **Reject** for the complete comparisons.

[Full replay](full-replay.json.gz) · [Forward observations](full-forward-observations.tar) · [Reverse observations](full-reverse-observations.tar)

## Fresh source through final link

Apple M1 ran 12 balanced fresh builds per arm. Each uses the original source and includes final executable linking with fixed support prebuilt.

| Build | Source to link | Stripped executable |
| --- | ---: | ---: |
| Qualified matcher layout | 2.296 s | 276,056 B |
| Dense scorer with existing guard placement | 2.301 s | 276,056 B |
| Dense scorer with early admission | 2.302 s | 276,056 B |
| Dense scorer with early admission and single-body preparation | 2.295 s | 276,056 B |
| Roc | 2.987 s | 191,912 B |
| libdeflate | 0.489 s | 155,752 B |

Two compiler recipes reproduce the same early-admission worker. The table includes both. The selected recipe’s paired time is 0.768× Roc and 4.699× libdeflate.

[Matched timings and phase breakdown](deployment.json)

## Reproduce the encoder

Apply the compiler patch in an isolated checkout of 9b5de6f46819660b1e6b41ddd38003e8cb8041d9. Use the frame-copy rebuild with compiler optimization level 3, direct source, preselected root inlining and skipped refused general SSA. Pass that rebuild directory to replay.py with a fresh output directory. The replay verifies the original canonical bytecode, encoder object and metadata.

[Baseline compiler instructions](../shared-continuation-returns/README.md#replay) · [Candidate flags and exact hashes](candidate.json) · [Verified local replay](verified-replay.json)
