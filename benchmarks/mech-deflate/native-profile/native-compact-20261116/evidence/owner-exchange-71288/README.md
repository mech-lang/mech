# Checked owner exchange

The optional controller change shares a nullable retain-and-release helper
across 26 structurally matched sites with distinct checked exits. Its
controller keeps the original equal-owner fast path and forwards the
existing checked release status and origin. The new helper retains the
incoming owner before retiring the outgoing owner. It uses the existing
release provider.

| Apple M1 diagnostic | Unpadded control | Shared owner exchange |
| --- | ---: | ---: |
| Encoder text | 71,752 B | 71,288 B |
| Encoder text / Roc | 1.244× | 1.236× |
| Encoder text / libdeflate | 1.935× | 1.923× |
| Object file | 159,368 B | 158,936 B |
| Linked text | 474,676 B | 474,212 B |
| Linked executable file | 956,056 B | 956,088 B |
| Checked root frame | 3,728 B | 3,712 B |
| Measured native stack peak | 5,280 B | 5,264 B |

The exact 71,768-byte published reference was rebuilt separately. Its
padding is disabled on both size-comparison workers so the owner flag is
the sole compiler-option difference in that pair. All 162 exact-output
checks and all six pinned allocation-failure outcomes match, including
aliases, cleanup, failure status and source origin. Two targeted Cranelift
JIT tests pass, including equal pointers, nulls, wrapping increment and a
negative continuation check.
[Exact source and artifact receipts](backend-work-hardened/worker-checks/variants.json)
· [Checked failures and stack](backend-work-hardened/linked-proofs/result.json)
· [Prototype code](as-run/source.patch.gz) · [Admission checks](as-run/guard-hardening.patch.gz)

Both 19-case orders record **19/19 Roc and 13/19 libdeflate** wins with
every individual control win preserved. The v1 result is **Reject**
because empty input slowed **1.187%**, with a 90% paired interval of
**[+0.630%, +1.728%]**. The adverse forward order for 16 KiB natural text
and the unresolved 4 KiB random case also remain in the record.
[Raw replay and per-case intervals](backend-work-hardened/source-owner-exchange-71288-focus/focus-replay.json)

The empty-input path uses the same terminal instructions and allocation
in both links. The identical timer import stub crosses an instruction-cache
line in the smaller executable. An 80-byte layout ablation preserves
the original control's import-stub address residues and emits **71,368 B**.
It passes 162 exact checks and preserves all 19/13 focused peer wins.
The observed empty-input difference is **+0.831%** with a 90% interval
of **[+0.059%, +1.623%]**. The original and layout-aligned intervals overlap.
The layout comparison is **Unresolved** under v1, with no established
slowdown and three case-level timing uncertainties.
[Linked path proof and reproduction](layout/DOSSIER.md)
· [Layout replay](backend-work-hardened/source-owner-exchange-71368-focus/focus-replay.json)

Recompute both focused experiments from the review packet directory.

```sh
python3 -B evidence/owner-exchange-71288/replay.py
```
