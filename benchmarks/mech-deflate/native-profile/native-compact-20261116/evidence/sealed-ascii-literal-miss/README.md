# Sealed ASCII literal mismatches

The ordinary compiler bypasses a fresh scanner and its continuation frames
when a sealed contiguous ASCII source settles an exact single-character
literal mismatch. The existing replay path performs the original rule
exit and rewind. The cache receives the same entry in the same order and
the parser consumes the same allowance. Other grapheme shapes and short
polls use the existing scanner. All **537 syntax tests** passed.

Eight balanced, fresh standalone parser pairs saved **28.5 ms**
at a paired **0.9664×** the immediate control. Every trial
reproduced the exact syntax tree, 718,353 events, and 3,967,302 parser steps.
[Standalone observations and plan](standalone/result.json) · [Syntax tests](standalone/tests.json)

## Fresh process through final deployment link

| Apple M1 | Through final link | Through strip |
| --- | ---: | ---: |
| Immediate reference | 2.57145 s | 2.58230 s |
| Mech | 2.54104 s | 2.55172 s |
| Roc | 2.98847 s | 2.99894 s |
| libdeflate | 0.48068 s | 0.49114 s |

Mech uses **0.850× Roc** and **5.290× libdeflate**
build time. The parser shortcut's paired complete-build change is
**29.3 ms saved** in the median, with 8 of eight pairs faster.
The parser phase medians were **813.4
ms** for the current control and **785.8 ms**
with the shortcut. Compilers, allocator and fixed deployment support are
prebuilt. The trial protocol holds the M1, clang/assembler boundary, source
and output hashes fixed.

All pairs reproduce the encoder bytes, metadata, bytecode and stripped
deployment executable. The linked runtime worker is identical and passes
162 checks. Its prior full compression scores are **81/81 Roc** and
**61/81 libdeflate** in both orders. The next runtime floor remains
**62/81 libdeflate**, with **63/81** as the target.

[Matched plan](matched-plan.json) · [All raw timing trials](matched-observations.json.gz) ·
[Worker verification](worker-qualification.json) · [Compiler pin](build.json) ·
[Machine-readable result](summary.json)
