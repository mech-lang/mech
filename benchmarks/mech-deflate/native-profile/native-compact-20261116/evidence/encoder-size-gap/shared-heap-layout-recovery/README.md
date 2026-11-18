# Apple M1 shared heap setup and matcher layout

The encoder retains all eight checked heap-call stubs and emits **64,652 B**
of text. That saves **16 B** from the 64,668 B screen and **384 B** from the
previously qualified 65,036 B worker. The only changed native function
relative to the 64,668 B worker is the 16-byte matcher. Its new block order
uses the existing checked AArch64 layout pass. The six training inputs omit
the original failing case. [Matcher profile](matcher-layout.json) ·
[Training and held-out estimates](prediction.json) · [Checked heap ABI](abi-proof.json)

| Full order | Roc wins | libdeflate wins | Distinct wins against Roc / libdeflate | Natural 32,769 B case vs Roc |
| --- | ---: | ---: | ---: | ---: |
| Forward | 81/81 | 62/81 | 74/74 · 56/74 | 0.969× |
| Reverse | 81/81 | 62/81 | 74/74 · 56/74 | 0.967× |

The policy V3 replay retains the exact worker. Both full orders preserve all
peer wins from the matched 65,036 B reference. The reference verification
checks 162 exact outputs, independent inflation, borrowed aliases, six
checked failure cases and private cleanup. Peak native stack is 4,784 B.
[Full result](result.json) · [Forward observations](forward-observations.tar) ·
[Reverse observations](reverse-observations.tar) · [Linked checks](linked-checks.json)

The 64,668 B worker also beats Roc on the disputed case in the focused
cohort. Its 64,652 B successor takes 1.91% more time on that case in the
forward focused order and 0.65% more in reverse. The source proof and full
sweeps establish the smaller worker's peer wins. The focused intervals
record the per-case tradeoff. [Focused results](focus-result.json)

Build the two shared heap options from the checkout with the named M1
recipes. The frame-copies recipe remains the maintained control.
[Source-built commands](../../README.md#source-built-m1-reference-encoders)
