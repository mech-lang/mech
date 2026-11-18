# Use 16-byte equality groups in the compact matcher

The checked match collector tests 16-byte groups. A difference uses the existing scalar loop at that group’s starting position. A matching group advances within the proved immutable window. The unchanged scalar loop handles the short remainder and reports the exact first difference.

Encoder text is **65,276 B**, a reduction of **240 B** on Apple M1. That is **1.131× Roc** and **1.760× libdeflate**. The worker matched 162 success observations and six checked failures, with 4,784 B of peak stack. [Size](size.json) · [Linked checks](linked-checks.json)

A separate JIT test exercised both equality widths on misaligned inputs, tail lengths, and first mismatch positions. It runs the pure equality loop with no saved prefix. [Test output](isolated-equality-tests.log) · [Fixture source](equality-fixture.rs.gz)

| Full runtime order | Roc wins | libdeflate wins | Natural 4 KiB vs libdeflate | Natural 8 KiB vs libdeflate | 320-byte overlap vs libdeflate |
| --- | ---: | ---: | ---: | ---: | ---: |
| Forward | 81/81 | 62/81 | 1.06126× | 0.97188× | 0.98378× |
| Reverse | 81/81 | 62/81 | 1.06467× | 0.97778× | 0.98247× |

Both runtime orders use 96 balanced blocks per case and include private heap cleanup. The release compiler reproduced the measured worker, encoder, canonical bytecode and metadata byte for byte. [V3 qualification](full-screen.json) · [Forward observations](full-forward-observations.tar) · [Reverse observations](full-reverse-observations.tar)

Fresh MechDeflate source through final executable link takes **2.184 s** in eight paired trials. That is **0.736× Roc** and **4.573× libdeflate**. The compiler binaries and common deployment support are prebuilt. [Build measurements](deployment.json) · [Raw observations](build-comparison.json.gz)

Apply the compiler patch to the pinned parent, build the compiler, and run replay.py. The source patch includes the existing opt-in compiler components required to reproduce this encoder. [Pinned source](source.json) · [Compiler command](compiler-build.json) · [Profiles and flags](candidate.json) · [Verified replay](verified-replay.json)
