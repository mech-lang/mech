# Share checked byte reads

A shared continuation replaces 15 equivalent defaulted byte reads. The checked entry shrinks by 416 B and the helper adds 156 B. The LLVM-free encoder emits **65,932 B**, down **260 B** from the qualified build, at **1.143× Roc** and **1.778× libdeflate**.

The linked worker matched 162 successful observations and six checked failures. Peak stack was 4,720 B. Canonical bytecode is identical.

[Exact size comparison](size.json) · [Linked checks](linked-checks.json)

## Compression timing

Both full orders time compression through private heap cleanup, using 96 balanced blocks per case.

| Order | Roc wins | libdeflate wins | Natural 8 KiB vs libdeflate | 320-overlap vs libdeflate |
| --- | ---: | ---: | ---: | ---: |
| Forward | 81/81 | 62/81 | 0.97386× | 0.99538× |
| Reverse | 81/81 | 62/81 | 0.97602× | 0.99482× |

V3 returned **Retain**.

[Full replay](full-replay.json.gz) · [Forward observations](full-forward-observations.tar) · [Reverse observations](full-reverse-observations.tar)

## Fresh source through final link

Apple M1 ran eight balanced trials with each compiler and fixed deployment support prebuilt.

| Build | Source through final link | Stripped executable |
| --- | ---: | ---: |
| Qualified Mech | 2.195 s | 276,056 B |
| Shared checked byte reads | 2.198 s | 276,056 B |
| Roc | 2.981 s | 191,912 B |
| libdeflate | 0.479 s | 155,752 B |

Paired build time is **0.737× Roc**, **4.587× libdeflate**, and **1.001×** the same-session Mech control.

[Build and phase timings](deployment.json) · [Raw build trials](build-comparison.json.gz)

## Replay

Use the compiler patch and rebuild command from the parser replay, then run this directory’s replay.py with the same rebuild directory and a fresh output path. The recipe selects the additional checked continuation and verifies the encoder object, metadata and canonical bytecode.

[Compiler source and rebuild instructions](../../sealed-grapheme-and-statement-replay/README.md#replay) · [Recipe flags and hashes](candidate.json) · [Verified replay](verified-replay.json)
