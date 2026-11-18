# Current native cleanup and preparation costs

Four balanced capture pairs at `7a0b3127ce2ec163d3989f706b7ab819901ec0c2` reproduce the selected
frame-copy bytecode, encoder object and metadata. The spans cover the
coordinator. The outer native span is 1419.5 ms beside a 1418.3 ms
uninstrumented native phase. Writing the trace after the outer span inflates
the process's own reported native phase. Use the inner span for attribution
and the separate ordinary build for elapsed comparisons.
[Plan](plan.json) · [All observations](observations.json.gz)

Root cleanup takes 513.0 ms. Its initial
cleanup records a reduction from 17,952 blocks and 47,660 instructions to
11,896 and 21,025, including 1,702 constant branches and 21,221 local phi
parameter removals. The current observations give aggregate changes and
individual analysis call counts. Per-call visited and changed nodes need
a narrower capture.

| Root cleanup work | Exclusive ms at measured boundaries | Calls |
| --- | ---: | ---: |
| Unreachable blocks and constant phis, including internal verifiers | 151.4 | 20 |
| Explicit verifier | 58.6 | 21 |
| Identity route forwarding | 39.8 | 2 |
| Alias resolution | 24.4 | 25 |
| Threaded loop edges | 23.2 | 1 |
| Dominator analysis called by cleanup | 21.7 | 37 |
| Local phi simplification | 20.0 | 5 |
| CFG analysis called by cleanup | 11.4 | 40 |

The earlier exclusive trace measured 67.7 ms rebuilding CFG and
dominators inside 245.4 ms of root verification.
That earlier trace bounds graph reuse on that revision. The current tail
layout derivation takes 88.8 ms, guard derivation takes 77.7 ms, and the
all-path initialization proof takes 3.8 ms.
[Current call timings](call-review.json) ·
[Earlier raw trace](../native-exclusive-work/shared/native-exclusive.json.gz)

## Selected native experiment

Measure which successive root checkpoints preserve entry, block layout,
terminator identities and ordered successors. Record visited blocks and
instructions, changed blocks and phi parameters, and the current nested
analysis cost. Try reusing previously verified graphs across one bounded
span with those dependencies unchanged. Invalidate when a change is
uncertain. Keep the complete IR verifier after edits and preserve the
first and standalone checks. Compare exact artifacts and elapsed wall time
in a separate matched run. Park after two flat or adverse iterations.
