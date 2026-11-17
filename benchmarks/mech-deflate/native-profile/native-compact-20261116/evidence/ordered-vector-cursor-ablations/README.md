# Ordered vector scorer follow-up

The two-address and lane-position variants preserve every individual peer
win in the 15-case focused screens. Each order uses 96 balanced blocks for
each case and records **15/15 wins against Roc and 12/15 against libdeflate**.

| Variant | Encoder text | 8 KiB forward vs C | 8 KiB reverse vs C |
| --- | ---: | ---: | ---: |
| Both window addresses | 71,064 B | 1.0002× | 1.0080× |
| Both addresses with lane positions deferred | 71,112 B | 1.0114× | 1.0140× |
| Lane positions deferred | 71,096 B | 1.0037× | 1.0115× |

The 71,064-byte and 71,096-byte builds both pass 162 exact checks and six
checked failures with **5,280 B** measured peak stack. Both 8 KiB timings
for both builds sit above libdeflate. The current frame-copy encoder is
71,192 B, with 81/81 Roc and 61/81 libdeflate wins in the existing full
qualification. The first promotion target is 62/81 libdeflate in both
orders while holding 81/81 Roc.

On the 8 KiB count trace, carrying either one or two addresses adds **43,020
scorer stack loads**. Deferring lane positions alone removes those reloads
and leaves **54 stack loads** and **6,984 stack stores**, the original counts.
The instruction count drops from **7,728,432 to 7,677,156** before machine
block placement. The narrower counter experiment establishes why the two
changes should be measured independently. The focused elapsed runs measure
the separate block placement. [Bound timings and counters](variants/) ·
[Full study record](summary.json)

## Compile-time work

Complete and Fueled analysis share the same discovered Tail graph. The
probe reuses it at **10** exact sites, preserving
the three artifact hashes. Those reuses account for **0.19 ms**
of measured graph work in the diagnostic pair. This is a small part of
the larger Tail derivation budget. [Graph trace](tail-inventory/result.json)

Four separate stack captures of the published compiler preserve its
bytecode, encoder and metadata. Median coordinator sample counts include
**196 SSA construction**,
**145.5 constant-phi removal**,
**85 guard closure**, and
**51.5 Tail-exit liveness**. Samples of the
worker threads and waiting are recorded separately. These are sample counts.
[Compiler profile and raw captures](compiler-sample/)

Hot SSA samples include nonlocal variable lookup and its sparse storage. The next experiment can count how many variables have one
definition and test staying inline until a second block needs storage.
