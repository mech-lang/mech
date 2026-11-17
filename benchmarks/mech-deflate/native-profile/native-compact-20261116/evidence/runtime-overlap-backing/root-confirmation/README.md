# Exact bottom-tested scorer repeat

This run binds the published experimental worker without the 32-page
threshold change. The prior full pair recorded 62 libdeflate wins forward
and 61 in reverse. Both Mech slots missed the 320-byte case in that reverse
order. This repeat preserves the 81 cases, checked worker, peer binaries,
96 balanced blocks per case, and forward and reverse case orders.
[Earlier full pair](../../ordered-vector-latch/README.md) · [Plan](plan.json)

| Order | Wins vs Roc | Wins vs libdeflate | Natural 8 KiB vs C | Overlap 320 vs C | Historical misses |
| --- | ---: | ---: | ---: | ---: | --- |
| Forward | 81/81 | 62/81 | 0.99261× | 0.98823× | All retained |
| Reverse | 81/81 | 60/81 | 1.00477× | 1.01830× | 320-overlap, prefix-natural-8192 |

The v3 decision is **Reject**. [Full replay](replay.json.gz)
