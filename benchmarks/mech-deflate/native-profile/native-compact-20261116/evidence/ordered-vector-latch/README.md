# Bottom-tested ordered reduction

The vector path already proves that at least one complete four-wide group
is available. Test the next group's index at the loop backedge and leave
the stable earliest-index reduction and scalar tail in place. The second
probe also defers adding each lane's offset until the last group finishes.

The emitted control matches the previous unlaid control. Both laid-out
workers pass all 162 exact linked output and capacity checks. Each focused
order covers 15 cases with 96 balanced blocks per case.

| Worker | Order | Encoder text | Natural 4 KiB vs C | Natural 8 KiB vs C | Focus wins Roc/C |
| --- | --- | ---: | ---: | ---: | ---: |
| latch-test-layout | forward | 71,048 B | 1.08725× | 0.99605× | 15/13 |
| latch-test-layout | reverse | 71,048 B | 1.09565× | 0.99967× | 15/13 |
| latch-test-deferred-layout | forward | 71,096 B | 1.09897× | 1.00459× | 15/12 |
| latch-test-deferred-layout | reverse | 71,096 B | 1.09917× | 1.00482× | 15/12 |
| latch-kept-root-layout | forward | 71,096 B | 1.08658× | 0.99409× | 15/13 |
| latch-kept-root-layout | reverse | 71,096 B | 1.09316× | 0.99960× | 15/13 |
| latch-both-cursors-layout | forward | 71,112 B | 1.08559× | 0.99566× | 15/13 |
| latch-both-cursors-layout | reverse | 71,112 B | 1.09297× | 1.00184× | 15/9 |

## Complete timing orders

`latch-test-layout`

The candidate wins 81/81 against Roc in both full orders.
It wins **61/81 and 61/81 against libdeflate**, with natural 8 KiB at
**0.99779× and 1.00436×**.

`latch-kept-root-layout`

The candidate wins 81/81 against Roc in both full orders.
It wins **62/81 and 61/81 against libdeflate**, with natural 8 KiB at
**0.99286× and 0.99866×**.

The first full run misses periodic 1 KiB forward, and the matched control
also misses that case. Natural 8 KiB wins forward and stays just over
parity in reverse. Keeping the qualified root layout restores the published
root bytes and preserves the placed scorer. This version wins natural
8 KiB both ways and reaches 62 libdeflate wins forward. Both Mech workers
miss `320-overlap` in its reverse full order. Its 24-block quarters contain
different absolute compression times across actors. Both complete cohorts
fall short of the v3 floor. [Overlap timing quarters](overlap-timing-quarters.json)

Before block placement, the two 8 KiB count traces both make
**43,074** stack loads in the scorer. The latch reduces
control-flow work and changes allocation enough to add 43,020 stack loads.
The paired elapsed observations measure both effects together.

## Natural 4 KiB runtime profile

In separate warm compression profiles, the ordered scorer accounts for
**49.8%** of the samples within the native encoder and match collection
accounts for **26.6%**. The capture classifies validation and waiting
outside those shares. The paired elapsed results above measure the full runtime
boundary. [Stack captures and plan](runtime-sample/result.json)

[Results and exact identities](summary.json) · [Compiler patch](compiler.patch.gz)
