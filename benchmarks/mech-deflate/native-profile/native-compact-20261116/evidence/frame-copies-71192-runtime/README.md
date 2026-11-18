# Full runtime orders for the compiler-emitted 71,192-byte encoder

This campaign compared the Cranelift frame-copy change emitted by source
commit `fa27c2c4c7595db6337497a2a00c6fd3869063d3` with the existing Mech
control on Apple M1. The encoder text shrank from 71,768 to 71,192 bytes,
or 0.992× the control, 1.234× Roc, and 1.920× libdeflate. Matched builds from
source through final link took 3.82900 s for the control and 3.82835 s for
the candidate. Both used the same prebuilt compiler and fixed link inputs.
[Source-emission proof](../shared-copy-tails/source-proof/exact-source-identity.json)
· [Matched build receipt](../shared-copy-tails/source-proof/summary.json)

| Mech actor | Encoder text | Object SHA-256 | Worker SHA-256 |
| --- | ---: | --- | --- |
| Control | 71,768 B | `7bf9df1aa610…` | `674d88602946…` |
| Source-emitted frame copies | 71,192 B | `96696eeed083…` | `ddefe57e371b…` |

The archive replays 81 named cases over 74 distinct input contracts. Each
order contains 96 balanced blocks per case and 982,656 timed calls. Both
orders record independent inflation, preserved input and output aliases,
matching output capacities, and exact reference bytes on all 81 cases.
[Plan](plan.json) · [Artifact hashes](as-run/artifact-verification.json)
· [Forward completion](source-frame-copies-71192-full-forward/complete.json)
· [Reverse completion](source-frame-copies-71192-full-reverse/complete.json)

| Order | Roc wins control / candidate | Roc precision control / candidate | libdeflate wins control / candidate | libdeflate precision control / candidate |
| --- | ---: | ---: | ---: | ---: |
| Forward | 81 / 81 | 80 / 79 | 57 / 57 | 57 / 57 |
| Reverse | 81 / 81 | 81 / 81 | 60 / 61 | 60 / 61 |

The v1 floors are 81 Roc and 61 libdeflate point wins per order. The forward
order misses the libdeflate floor by four for both Mech workers. In reverse,
the candidate meets that floor and gains `reymont-prefix-16384`. The candidate
keeps every control point win in each order. The unsupported forward Roc
wins are `prefix-periodic-4096` for both workers and `prefix-periodic-16384`
for the candidate. The v1 runtime decision is
**Reject**, with 69 bounded cases, 11 unresolved cases, and one regression.
For `prefix-periodic-16384`, the candidate is +10.468% slower than the control
with a 90% interval of [+1.723%, +19.681%]. The forward estimate is +21.725%
[+3.243%, +43.494%] and the reverse estimate is +0.252% [−0.038%, +0.555%].
[Full replay and raw observation receipts](full-replay.json)

## Same-worker drift across campaigns

The 71,768-byte control in this campaign is byte-for-byte the earlier
71,768-byte candidate with 81 Roc and 61 libdeflate wins in both orders.
The object, worker, metadata, runtime receipt, and both comparator workers
match their earlier hashes. Both campaigns also pin the same run inventory,
fixture contents, environment, call contract, timing script, and policy.
The earlier and current Roc pins record the same metadata hash. The timer
rehashes the Roc executable, and the file at the recorded Roc metadata path
was unavailable for an independent rehash during archive preparation.
[Earlier full replay](../bound-71768-runtime/full-replay.json)
· [Checked hashes and timing diagnostic](baseline-drift.json)

| Forward case | Earlier 71,768 × libdeflate | Current 71,768 × libdeflate | Current 71,192 × libdeflate |
| --- | ---: | ---: | ---: |
| `prefix-periodic-1024` | 0.986 | 1.421 | 1.374 |
| `prefix-periodic-4096` | 0.934 | 3.517 | 3.683 |
| `prefix-periodic-8192` | 0.901 | 1.537 | 1.435 |
| `prefix-periodic-16384` | 0.859 | 1.844 | 2.245 |

These four consecutive cases account for the entire forward drop in the
unchanged worker's libdeflate win count. The reverse order's single control
loss is `reymont-prefix-16384`, which went from 0.942× to 1.027× libdeflate
for the identical worker. The candidate won at 0.956×. The wall-time
diagnostic uses all 96 blocks for each of the changed wins in both orders.
At forward 4 KiB, the control's median block mean went from 94.33 to
245.67 µs. The candidate measured 385.60 µs. The largest block means were
6.17 ms for the control and 5.55 ms for the candidate. At least two actors
exceeded twice their historical comparison median in 38 of 96 blocks.
That count reached 61 at forward 16 KiB and was zero for all four periodic
cases in reverse. Reverse `reymont-prefix-16384` had 26 such blocks,
including 6 in which all four actors crossed that threshold. The source
of the shifts remains unidentified. The v1 replay includes every acquired
block.
[Cross-campaign comparison](baseline-drift.json)

## Earlier focused evidence

The two historical 17-case orders recorded 17 Roc wins and 13 libdeflate
wins each. Their candidate worker and object match this campaign. The
prototype's formatted JSON and source-emitted compact JSON have separate
hashes and identical contents after serialization. The original focused
`prefix-random-4096` result remains unresolved under v1 after its adverse
reverse estimate of +1.459% [+0.838%, +2.106%]. The separate duplicate-control
follow-up measured +0.093% [+0.064%, +0.123%]. The two full orders here bound
that case at −0.154% [−0.555%, +0.264%] in their own replay.
[Focused replay](../shared-copy-tails/focus-replay.json)
· [Duplicate-control replay](../shared-copy-tails/repeatability/combined-replay.json)
· [Metadata reconciliation](../shared-copy-tails/source-proof/metadata-format.json)

From the review packet directory, recompute the unmodified v1 intervals and
the same-worker diagnostic using the archived observations.

```sh
python3 -B evidence/frame-copies-71192-runtime/replay-archive.py
```

The command confirms the recorded v1 decision. [Portable replay log](portable-replay.log)
