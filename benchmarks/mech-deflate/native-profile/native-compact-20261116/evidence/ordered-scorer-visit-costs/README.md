# Ordered scorer address and winner selection study

Direct caller arguments emit **71,000 B** of encoder text. That is **192 B**
below the qualified build, **1.231× Roc** and **1.915× libdeflate**. The
comparison passes 162 exact output and capacity checks and all six
checked failures. It also raises measured peak stack from **5,280 to
5,296 B**, matching the previous packet-address experiment. The timing
table below covers the two separate conditional-winner variants.
[Address ABI and stack proof](address-checks/result.json)

## Winner selection

The bound scorer trace for natural 4 KiB shows a strict improvement in
3,786 of 32,015 first scalar visits
(11.8%) and
15,089 of 70,480 continued visits
(21.4%). In each variant the source
item is pure and its construction moves onto the already-proved strict
improvement edge. Ties retain the earlier item and the checked scalar
fallback still covers refused input. All linked candidates pass the same
162 checks.

| Focused scorer variant | Encoder text | 8 KiB forward vs C | 8 KiB reverse vs C |
| --- | ---: | ---: | ---: |
| Branch on first improvement | 71,048 B | 1.1113× | 1.1199× |
| Branch on all scalar improvements | 71,080 B | 1.1928× | 1.2067× |

Both lose the same three of fifteen Roc wins and four of twelve C wins in
each focused order. The lost Roc cases are **prefix-natural-16384**, **prefix-natural-32769**, **reymont-prefix-32769**. The lost libdeflate cases
are **heldout-mixed-131073**, **prefix-natural-16384**, **prefix-natural-32769**, **reymont-prefix-32769**. The compiler study rejects both variants. The full published
frame-copy worker remains the release runtime reference.

With the root-tail profile omitted and the original block order, natural
8 KiB executes
**7,728,432** scorer instructions in the control and **7,532,673** with the
first-visit branch. The branch count increases from **1,255,812** to
**1,336,680** before the separate layout pass. Instruction counts and
predicted fallthrough savings explain the work changes. The measured
compression times supply the promotion decision.

[First-visit timings](variants/ord-first-layout/focus/screen.json) ·
[All-scalar timings](variants/ord-all-layout/focus/screen.json) ·
[Source patch and disabled-build pins](probe-compiler.json) ·
[Raw compiler patch](scorer.patch.gz) · [Study record](summary.json)

## Next scorer target

The retained four-wide loop executes 24,210 groups on natural 4 KiB and
40,728 on natural 8 KiB. Each group repeats five address instructions and
one lane-constant load. Carrying the two current window addresses through
the vector loop would trade repeated index arithmetic for pointer
advances. The original initialized-window bounds, wrapping prices and
first-winner order still apply. The counts bound dynamic work and the
elapsed impact awaits a separate uninstrumented comparison.
[Exact vector instructions and group counts](vector-census.json)
