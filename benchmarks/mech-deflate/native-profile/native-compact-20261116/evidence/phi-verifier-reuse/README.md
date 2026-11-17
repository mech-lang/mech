# Reuse verification after a no-op phi sweep

The adjacent pruning and phi cleanup keep the original order. The first
pass verifies the pruned function. Constant-phi removal reports whether
it edits block parameters, aliases or branch arguments. Changed functions
receive a second verification. The standalone phi entry verifies its result.
The paired implementation keeps one ISA and owns both analyses across the
passes. [Prior count-only census](../scoped-function-replay/native-verify/README.md)

| Eight matched Apple M1 builds | Mech control | Mech verifier reuse | Roc | libdeflate |
| --- | ---: | ---: | ---: | ---: |
| Source through final link | 2.85140 s | 2.80251 s | 2.98347 s | 0.47961 s |
| Source through strip | 2.86197 s | 2.81307 s | 2.99371 s | 0.49005 s |

The paired build takes **1.40% less time** than the
control. Median native time falls from **1.46444 s**
to **1.42450 s**. Compilers and host support are
prebuilt. The exact artifacts reproduce on each build. The existing worker
was also relinked and passed 162 exact linked calls. The archived full orders
score 81/81 Roc and 61/81 libdeflate in each direction.
[Plan](source-through-link-plan.json) · [Raw builds](source-through-link-observations.json.gz)
· [Worker identity](worker.json) · [Medians and phases](summary.json)

The focused verifier checks compare edited and unchanged sweeps with separate
passes, including unreachable removal and exact final flowgraph ordering.
A malformed return produces the same verifier diagnostic after pruning.
[Focused check source](check.rs) · [Receipt](check-receipt.json)
