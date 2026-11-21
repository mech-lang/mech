# Current allocator spill-pair retest

**Retain the source-built `m1-register-spill-pairs` recipe: 63,756 B, 81/81 Roc and 62/81 libdeflate in both complete orders.** It saves 48 B against the 63,804 B selected encoder. All prior individual peer wins are preserved, including precision support for all 81/62 wins. This recovers a historical optimization that the former policy rejected for slowdowns on cases that still won.

The original Mech source and canonical bytecode remain unchanged. The live generic allocator feature already exists in this PR's compiler: `MECH_COMPACT_PAIR_REGISTER_SPILLS=1`. It emits paired ARM64 register spill/reload instructions. The current combination includes scoped phase packets, matcher spill forwarding, preserved checked-root registers and the sparse guard scheduler. No LLVM dependency is added. This is a new qualified recipe and evidence, not another codec-source rewrite.

## Complete builds and size

| Actor | Encoder text | Source through final link | Through strip | Stripped executable |
| --- | ---: | ---: | ---: | ---: |
| Prior selected, same compiler | 63,804 B | 2.221099 s | 2.232103 s | 276,072 B |
| Current spill pairs | 63,756 B | 2.219958 s | 2.231146 s | 276,072 B |
| Roc | 57,696 B | 2.984375 s | 2.995337 s | 191,912 B |
| libdeflate | 37,080 B | 0.481603 s | 0.492534 s | 155,752 B |

Eight balanced fresh-process builds per actor include final deployment linking. Fixed deployment support is prebuilt for every actor. Strip is reported separately. The current paired complete-build ratios are **0.743225× Roc**, **4.609545× libdeflate**, and **0.999683× the same-compiler reference**. The paired median difference is **0.704 ms saved**, with 4/8 faster; treat build time as effectively unchanged. Do not combine medians or effects from earlier cohorts.

Current phase medians are **0.709814 s parsing**, **0.217974 s source semantics**, **0.006830 s artifact creation**, **0.002543 s encoding** and **1.219665 s native emission**. Catalog join wait is 23 microseconds; catalog work runs during parsing. Final link is **0.035493 s**, strip **0.010865 s**. Independent medians need not add. Compiler-building time is outside this source-to-executable boundary.

Encoder text is **1.719417× C** and **1.105033× Roc**. The checked root stays 46,788 B / 3,296 B frame; peak active stack stays **4,848 B**, below the unchanged 5,280 B cap. The stripped executable remains 276,072 B. The encoder still exceeds C by **26,676 B**.

## Runtime and current losses

All 162 output/inflation/alias observations, six checked failures and 12 successful recoveries pass. The normal and symbolic allocation-checked compiler modes produce identical object, metadata and bytecode, both before and after valid layout restoration. Each complete order has 81/81 Roc and 62/81 C point wins and precision-supported wins. The [required old fixture wins](required-wins.json) are all retained; counts alone are not used as a substitute.

`random-1024` slows 0.735% / 0.900%, and `prefix-random-1024` slows 0.569% / 0.865%, forward / reverse. Both retain their peer wins. Policy v3 reports these slowdowns and retains the candidate. It also records uncertain case comparisons in the full replay. There is **no 63rd C win**.

Natural 4 KiB is **0.99444× / 0.99205×** the current reference time and still **1.05396× / 1.06722× C**. Large-random 1 MiB is **0.99795× / 1.00063×** the reference time; the raw instruction reduction does not establish a large runtime improvement. The full [case ratios](runtime-case-ratios.json), raw orders and replay include every case.

## What pairing changes

On the exact normal workers' untimed collector counter clones, 1 MiB random input changes **245,026,411 → 241,346,097** executed instructions and **25,947,639 → 22,267,325** stack-load instructions: 3,680,314 fewer instructions and 14.2% fewer stack-load instructions. Branches remain 45,024,643 and stack-store instructions remain 9,698,800. Paired loads still read both values; this is instruction-count reduction, not a 14.2% reduction in bytes read. The clones are never used for latency.

The initial strict scorer graph rebase failed. Inspection established that one paired instruction reduced executable code by four bytes while the aligned literal pool stayed fixed, leaving one extra unreachable zero padding word. The final rebase requires identical ordered non-padding control graphs and proves no branch or fallthrough into that zero tail. It includes all actual padding words in the new source-defined layout. Both the failed attempt and the amended preparation proof are retained. The other two profile graphs match directly. Shipping artifacts are emitted by the compiler; diagnostic counters alone alter cloned instructions.

## Reproduce and verify

Build this checkout's compiler and encoder with:

```sh
python3 rebuild.py /tmp/mech-m1-register-spill-pairs \
  --variant m1-register-spill-pairs --mimalloc-object /path/to/mimalloc.o
```

Or, from this evidence directory, use a compiler built from this PR:

```sh
python3 reproduce.py /path/to/compile_profile /tmp/mech-register-pair-replay
python3 verify.py
```

The replay regenerates normal and allocation-checked artifacts and checks every archived object/metadata/bytecode identity. It makes no runtime claim for a different linked support worker. Full raw orders, matched complete builds, actual linked checks, exact probe-object provenance, counters and as-run scripts are archived alongside this document.

Source base: `388e1087c401cd3b9f87a31b10652372ab720096`. Compiler SHA: `6711160f915f00bb783831fc14f0144ba7949c06ea9df264edd727a62d44e715`. Fixed-support worker SHA: `c2ffb9aa77320e29c4dc9caeb647afbc130166587a551f168ea5963c3373f63b`. Object SHA: `348c298469e92fdff8571c098f460c62253160869293bc4c0fa55dba85f757d0`.

## Continue

Use the [19-mechanism history audit and all-loss profiles](../../history-and-profiles-20261119/README.md) to trace remaining collector word/root/link checks and live ranges. Those profiles bind the prior 63,804 B worker; the fresh complete-order ratios here bind the new worker. Keep natural 4 KiB as the closest next-win check, and repair its pointer/countdown exit transport as a secondary experiment. Small slowdowns without peer losses are allowed. The larger 1× libdeflate text and complete-build goal remains unfinished.
