# Two fully bound runtime orders for the 71,768-byte encoder

This diagnostic campaign compared worker `674d88602946…` against the
71,960-byte control `439a28b3b6ea…` on Apple M1. It recorded the workers,
comparator workers, input contents, call contract, and environment before
acquiring either order. Both orders completed all 81 named cases with 96
balanced blocks per case and 982,656 timed calls per order. Compressed
bytes, capacities, aliases, and independent inflation passed throughout.
[Acquisition plan](plan.json) · [Fixture digests](fixture-content.json)

| Order | Roc wins | libdeflate wins | Individual wins lost |
| --- | ---: | ---: | ---: |
| Forward | 81/81 | 61/81 | 0 |
| Reverse | 81/81 | 61/81 | 0 |

The candidate's 90% per-case intervals also support 81 Roc and 61
libdeflate wins in each order. The corresponding counts over the 74
distinct input contracts are 74 and 55.
[Full replay and raw receipts](full-replay.json)

The initial v1 per-case regression replay bounded 75 cases at 0.5%.
A separately declared duplicate-control study bounded five of the six
remaining cases. Its two byte-identical control copies varied beyond
0.5% on `seven-stored-prefix` (−0.803% to +1.371%). That case remains
unresolved despite a faster candidate point estimate in the follow-up.
About 99% of calls in this case measured zero or one 41–42 ns clock tick.
The observed control mean was 31.18 ns, where a 0.5% change is about
0.16 ns. [Recorded duration histogram](clock-quantization.json)

The runtime component's final decision is **Unresolved**, with 80 bounded
cases and zero established regressions. The opt-in recipe and complete
historical [branch experiment](../branch-on-zero/README.md) remain available.
[Duplicate-control plan](followup-plan.json) · [Final per-case replay](full-and-repeat-replay.json)

Recompute the paired-block intervals and validate the archived inputs from
the review packet directory.

```sh
python3 -B evidence/bound-71768-runtime/replay-archive.py
```

The replay verifies the runtime component. Complete promotion also
requires the artifact, build, stack, failure, and resource evidence in
the [qualification policy](../../QUALIFICATION.md).
