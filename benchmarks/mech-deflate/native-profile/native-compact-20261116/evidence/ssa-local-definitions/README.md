# SSA lookup and definition storage

One change returns a known local SSA definition before entering
the predecessor lookup state machine. The other keeps a single sparse
definition inline until a second block needs storage. Each passes 15 SSA
tests and reproduces the bytecode, encoder and metadata across the
campaign. Both candidates link the exact runtime worker and pass 162 checks.
[Source patches and tests](build/)

| Apple M1, ten matched blocks | Median source through final link | Source compiler CPU |
| --- | ---: | ---: |
| Maintained Mech | 2.54902 s | 2.51716 s |
| Cached SSA definition return | 2.54234 s | 2.51069 s |
| Inline single-definition storage | 2.54442 s | 2.50871 s |
| Roc | 2.99543 s | 2.95097 s |
| libdeflate | 0.48343 s | 0.44538 s |

| Mech variant | Paired elapsed saving | 90% interval | Paired source CPU saving | Median native phase |
| --- | ---: | ---: | ---: | ---: |
| Cached SSA definition return | 9.66 ms | -7.44 to 10.88 ms | 5.22 ms | 1386.6 ms |
| Inline single-definition storage | 5.88 ms | 0.86 to 17.41 ms | 10.68 ms | 1386.0 ms |

The maintained native phase median was 1383.2 ms.
The review decision is **Retain**.
The earlier local-return campaign overlapped active rendering. Its paired
elapsed interval was -150.05 to 188.87 ms.
[Matched plan](matched-plan.json) · [Raw measurements](matched-observations.json.gz) ·
[Earlier run](contended-observations.json.gz)
