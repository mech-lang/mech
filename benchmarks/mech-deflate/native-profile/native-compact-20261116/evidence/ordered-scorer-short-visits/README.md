# Short scalar visits and four-lane entry

The scalar arm takes two visits at once after the existing entry proof
admits both, then runs the existing scalar tail for an odd remainder.
It compares the second cost strictly with the first and applies the same
strict comparison to the incoming best. The vector arms use the same
four-lane code with entry minima of four, eight, or twelve admitted
elements. All arms use the same supplied compressor source.

The disabled probe reproduces the maintained compiler's bytecode, encoder
object, and metadata. With the previous scorer settings and placement it
also reproduces the 71,096 B experimental worker exactly. Each placed
worker has the same checked root bytes and relocations. The matched
linked checks cover 162 observations and six checked failures.
[Compiler patch and build](compiler/) · [Linked checks](checks/)

Peak stack by arm is vector-4 5,280 B, vector-8 5,280 B, vector-12 5,280 B, paired-scalar 5,280 B. The checked root and
loop shapes used in timing come from the placed objects. Instruction
counts are separate from timing. The grouped scalar arm increased the
number of stack loads. Raw counts include each variant before layout
and the vector arms after layout.
[Machine-readable results](summary.json)

| Arm | Order | Encoder text | Wins Roc/C | Overlap 320 vs C | Natural 4 KiB vs C | Natural 8 KiB vs C |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| vector-4 | focus forward | 71,096 B | 15/12 | 0.95933× | 1.15283× | 1.03972× |
| vector-4 | focus reverse | 71,096 B | 15/12 | 0.98559× | 1.16212× | 1.04812× |
| vector-8 | focus forward | 71,096 B | 15/13 | 0.99400× | 1.09598× | 0.99593× |
| vector-8 | focus reverse | 71,096 B | 15/12 | 0.98740× | 1.10057× | 1.00499× |
| vector-12 | focus forward | 71,096 B | 15/13 | 0.98860× | 1.08195× | 0.99454× |
| vector-12 | focus reverse | 71,096 B | 15/13 | 0.98703× | 1.08885× | 0.99855× |
| paired-scalar | focus forward | 71,368 B | 15/12 | 0.98957× | 1.08648× | 1.00079× |
| paired-scalar | focus reverse | 71,368 B | 15/12 | 0.98865× | 1.09323× | 1.01115× |

The 12-visit threshold brought natural 4 KiB to **1.08195× and
1.08885×** libdeflate. Its natural 8 KiB was marginally slower than
the same-order 16-visit scorer. None of the changes added a focused peer
win over that control.

In a separate, predeclared duplicate-control check, both slots of the
maintained worker won overlap in both orders. The exact prior scorer won
overlap in both A/B orders and hit natural 8 KiB at **0.99733× and
1.01345×** libdeflate. Those acquisitions neither establish a
candidate-specific overlap loss nor explain the earlier reverse miss.
[Duplicate-control observations](fixed-worker-diagnostic/)

The 71,192 B frame-copy encoder remains the maintained build. The next runtime mechanism is the repeated bound reload.

On natural 8 KiB, the 16- and 12-visit placed scorers each execute
43,020 reloads from `[sp, #0x70]` across six calls. Both stores of that
spill slot receive the same packet field at `[x1, #0x58]`. The hot
reload feeds the bounds comparison for the literal-indexed cost load.
This gives a specific lifetime to shorten or specialize before
reevaluating runtime margin. [Register spill census](spill-census.json)

The runtime gates require both complete benchmark orders to confirm
81/81 Roc and at least 62/81 libdeflate, with 63/81 as the next target.
[Policy](../../qualification-policy-v3.json)
