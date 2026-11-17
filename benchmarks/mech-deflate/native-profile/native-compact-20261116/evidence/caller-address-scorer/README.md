# Checked scorer address experiment

Both final candidates have **71,048 B** of encoder text, or **1.231×
Roc** and **1.916× libdeflate**. It saves **144 B**
against the exact 71,192 B comparison worker on Apple M1. The caller-address candidate
passed all 162 exact output and capacity checks and all six checked failure
cases. Its checked root frame grew from **3,728 B to 3,744 B**, and its
measured native stack grew from **5,280 B to 5,296 B**. That exceeds the
predeclared stack nonregression limit, so use the [current frame-copy
recipe](../../CURRENT.md) for deployment. This study measures encoder size
and compression time.

## Timing

Each focused order covers the same 15 cases in 96 balanced blocks per case.
The full orders cover all 81 cases at the same repetition count. The
compression interval includes private heap cleanup. The final two columns
give paired Mech/libdeflate time on the 8 KiB natural-text input. Wins are
counts against Roc and libdeflate, respectively.

| Scope | Order | Control wins | Candidate wins | Control time | Candidate time |
| --- | --- | ---: | ---: | ---: | ---: |
| Focus | Forward | 15/12 | 15/13 | 1.0293× | 0.9941× |
| Focus | Reverse | 15/12 | 15/12 | 1.0427× | 1.0025× |
| Full | Forward | 81/61 | 81/62 | 1.0336× | 0.9976× |
| Full | Reverse | 81/61 | 81/61 | 1.0418× | 1.0055× |

The runtime-only v2 replay returned **Retain** under the historical
61-win policy. The forward run set a high-water mark of 62/81 against
libdeflate. The reverse run recovered 61/81. Future candidates need 62/81
against libdeflate and 81/81 against Roc in both orders. Stack and checked failures
have separate receipts. The compiler source, fixtures, allocator, Roc
worker, C worker and frame-copy control are pinned in the plans.

| Full order | Gained peer cases | Lost peer cases |
| --- | --- | --- |
| Forward | libdeflate `prefix-natural-8192` | None |
| Reverse | None | None |

[Full plan](variants/caller-addresses-refined/full/plan.json) ·
[Forward raw observations](variants/caller-addresses-refined/full/forward.tar.gz) ·
[Reverse raw observations](variants/caller-addresses-refined/full/reverse.tar.gz) ·
[Full v2 replay](variants/caller-addresses-refined/full/replay.json.gz) ·
[Stack and failure checks](checked-contracts/result.json)

## How much each change contributed

| Variant | Encoder text in bytes | Natural 8 KiB forward | Natural 8 KiB reverse |
| --- | ---: | ---: | ---: |
| caller-addresses-refined | 71,048 | 0.9941× | 1.0025× |
| caller-addresses-trained | 71,048 | 1.0028× | 1.0091× |
| horizontal-layout-bound | 71,144 | 0.9995× | 1.0099× |
| horizontal-reverse | 71,096 | 1.0287× | 1.0347× |
| horizontal-reverse-layout | 71,096 | 1.0026× | 1.0045× |
| horizontal-reverse-layout-unpadded | 71,048 | 1.0005× | 1.0074× |
| horizontal-reverse-scheduled | 71,112 | 1.0042× | 1.0022× |

Each row uses a fresh balanced comparison against the exact same control.
All seven candidates preserved the focused individual peer wins in both
orders. The caller-address variant prepares the biased read-only cost base
and dense code table address after the existing bounds and work checks,
then writes them to two initialized words in the synchronous private phase
packet. The scalar fallback and checked error origins still exercise the
same contracts. The shared input slot measures 440 B in both metadata
records. The checked root frame accounts for the 16 B peak increase.
Horizontal minima and reverse low words save 96 B. The root-tail profile
was tied to the previous body and its omission saves another 48 B of text.
The unpadded ablation reaches the same 71,048 B with 5,280 B measured
peak stack, passes the 162 output and capacity checks and six checked
failure cases, and is just above 1× on natural 8 KiB in both focused orders.
[Padding attribution](root-tail-ablation.json) ·
[Unpadded stack and checked failures](unpadded-checked-contracts/result.json)

The refined block order uses four natural-text training inputs and exact
directed edge counts for the changed scorer.

A separate instruction trace on the 8 KiB case counted **7,752,576 →
7,555,494 instructions** across six scorer calls for the unlaid combination
of caller-provided addresses, horizontal minima and reverse low words.
There were **197,082 fewer instructions (2.54%)**. The refined order was
timed independently. [Count receipts](instruction-counts/) ·
[Refined layout proof](profiles/caller-addresses-refined-layout-proof.json)

## Target 63/81

The next target after the 8 KiB case is natural 4 KiB. It runs at
**1.090× / 1.097× libdeflate** in the two full orders and needs another
**8.8%** reduction at the slower margin. In the separate callback
profile, the checked scorer accounts for **47.3%** of elapsed
compression time and the tree matcher for **29.9%**. Both hot atoms
have the same bytes and relative relocations as the timed candidate. These
fractions guide the next experiment. The matched ratios above give the
runtime result. The 4 KiB scorer trace recorded 70,480 visits to the
hottest scalar loop and 3,472,745 instructions across five scorer calls.

[Ranked remaining cases](next-win.json) ·
[Phase pin and breakdown](phase-attribution/result.json) ·
[All callback observations](phase-attribution/phase-timings.json.gz)

## Reproduce the object

The input and native target are pinned to the Apple arm64 build of `176a80bde45bdc0c137ddd797e5961c42f23d56c`.
Save this evidence directory from the review branch before applying the
diagnostic compiler patch in a temporary checkout of that revision.
The patch ships separately from the maintained compiler source. Build the
opt-level-3 compiler with the [current frame-copy recipe](../../CURRENT.md).
Leave the experiment flags off for this initial build. Its bytecode, object
and metadata reproduced all three published hashes before the experiment.

Run the following from the evidence directory in the review checkout. The
emitter reuses the archived native flags and block profiles and verifies
the three candidate hashes. Its compiler argument is the compiler just
built with the diagnostic patch.

~~~sh
python3 -B reproduce.py /tmp/mech-scorer-build/compiler/release/compile_profile /tmp/mech-scorer-object
~~~

[Diagnostic compiler patch](caller-addresses.patch.gz) ·
[Compiler and disabled-artifact pins](caller-compiler.json) ·
[Candidate hash and flags](variants/caller-addresses-refined/receipt.json) ·
[Reproduced artifact hashes](portable-reproduction.json) · [Study record](summary.json)

Carrying the two addresses as private call arguments would avoid writing
extra packet words. Confirm the root frame after register allocation. A
separate runtime direction is to measure repeated distance-slot work. The layout screens alone put the closest marginal case near parity
with substantial run-order sensitivity.
