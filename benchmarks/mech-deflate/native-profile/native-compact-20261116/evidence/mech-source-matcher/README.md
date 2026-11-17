# Ordinary Mech matcher source screens

These experiments use the same Cranelift compiler and options independently
of controller windows, mapped phase layout and requested inlining from the
71,768-byte recipe. The generic reference emits 91,556 bytes. Each compiled
worker passed 162 exact calls with matching output bytes and capacities,
independent inflation, and preserved input and output aliases.
[Options and excluded layout settings](as-run/plan.json)

| Apple M1 diagnostic source | Encoder text | Both Roc orders | Both libdeflate orders | v1 |
| --- | ---: | ---: | ---: | --- |
| Generic control | 91,556 B | 12/13 each | 4/13 each | Comparison reference |
| Read each literal once | 91,172 B | 12/13 each | 4/13 each | Unresolved |
| Early run while both match roots are stale | 105,204 B | 11/13 each | 3/13 each | Reject |

Reading the current literal once supplies both observation sampling and the
cache header. It saves 384 bytes in this configuration and bounds the 4 KiB
random case at **1.696% faster** with a 90% interval from **0.349% to
2.902% faster**. The 8 KiB natural change is **−0.110%**
[−0.587%, +0.324%]. Each order preserves every individual control win.
[Source diff](builds/literal-once/source.diff.gz)
· [Results](mech-unmatched-runs/mech-ordinary-literal-once-qual/focus-replay.json)

Batching positions with stale roots keeps the cache/header and observation
updates in ordinary Mech. The prefix stops before the next block decision,
input lookahead or slide boundary. The source needs 4,023,837 parser steps.
Its diagnostic build raises the parse fuel for both variants and verifies
that the expanded-fuel control has byte-identical outputs. The default
4,000,000-step production budget continues to apply in the regular compiler.
The candidate emits 13,648 more bytes and ran **17.505% slower** on 1 MiB
random input with 90% bounds of **[+16.381%, +18.195%]**. It lost the
held-out 65,537-byte random case against Roc and the 16 KiB periodic case
against libdeflate in both orders.
[Source diff](builds/literal-run/source.diff.gz)
· [Results](mech-unmatched-runs/mech-ordinary-literal-run-qual/focus-replay.json)

The four exact-control summaries place 8 KiB natural text within roughly
3–4% of libdeflate. The stable large random-input losses generally require
28–39% less time to flip. The separate root-age simulation estimates
opportunities before collector admission and checkpoint checks. It models
upper bounds on available run positions.
[Stable losses](as-run/control-win-gaps.json)
· [Stale-root coverage model](as-run/root-coverage.json)

From the review packet directory, replay the preserved focused orders.

```sh
python3 -B evidence/mech-source-matcher/replay.py
```

Each candidate has two 24-block orders over the same 13 frozen cases.
Complete qualification requires both full orders at the runtime floors.
