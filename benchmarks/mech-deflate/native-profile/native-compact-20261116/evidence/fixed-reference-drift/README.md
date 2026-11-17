# Fixed-worker periodic drift diagnostic

Both Mech slots use the exact 71,768-byte reference worker. A predeclared
eleven-case set preserves case indices, input identities, actor orders,
warmup and validation. Both serial orders completed all eleven cases with 24
balanced blocks apiece and 13,152 recorded calls per order. All affected
periodic cases returned to their historical timing ranges and were peer
wins in both slots in both orders.
[Plan](plan.json) · [Forward results](fixed-reference-drift-forward/summary.json)
· [Reverse results](fixed-reference-drift-reverse/summary.json)
· [Planned and actual block counts](acquisition-reconciliation.json)

| Affected periodic size | Forward Mech / libdeflate | Reverse Mech / libdeflate |
| --- | ---: | ---: |
| 1024 | 0.9805× | 0.9817× |
| 4096 | 0.9331× | 0.9113× |
| 8192 | 0.8813× | 0.8873× |
| 16384 | 0.8653× | 0.8537× |

In the archived outlier, the fourth 1 KiB block group shows the same Mech
binary at 283.84 µs against 44.24 µs in the first group. Roc and libdeflate
also slow in the fourth group. The 4 KiB and 16 KiB runs have much larger
Mech excursions, while the end of the 32 KiB run recovers for all three.
[Archived observations by block quarter](archived-by-run-and-quarter.json)
· [Original drift qualification](../frame-copies-71192-runtime/README.md)

The current run records load average. macOS returned errors for thermal
and CPU power telemetry. The archived episode has no matching telemetry. Its
precise cause remains open. This is a partial diagnostic with duplicate
controls. The two archived eligible full orders remain the fixed-worker
evidence for the v2 runtime gate.
