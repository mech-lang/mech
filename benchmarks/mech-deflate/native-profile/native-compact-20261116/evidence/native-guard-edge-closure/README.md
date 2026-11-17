# Incremental guard propagation on branch edges

Record the relations that read a changed guard and propagate only affected
relations after a new branch fact. Each round keeps relation order.
Other guard transfers retain their existing closure. The probe passes
17 guard tests, with a differential test covering closed and conflicting
paths. Every measured Mech trial emits the same bytecode, encoder and
metadata. The linked runtime worker matches and passes 162 exact checks.

| Apple M1, 10 balanced blocks | Median source through final link |
| --- | ---: |
| Maintained Mech | 2.53114 s |
| Probe disabled | 2.53535 s |
| Incremental guard propagation | 2.53387 s |
| Roc | 2.98024 s |
| libdeflate | 0.48064 s |

The enabled probe saves a paired **3.21 ms**
against the same binary with its switch disabled. It takes
**4.58 ms longer** against the maintained compiler.
The current compiler remains selected.

[Matched observations](matched-observations.json.gz) · [Complete plan and artifact pins](matched-plan.json) ·
[Source patch](guard-index.patch.gz) · [Tests](tests.log.gz)
