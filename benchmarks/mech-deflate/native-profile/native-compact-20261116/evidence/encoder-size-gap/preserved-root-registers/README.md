# Qualified 63,804 B preserved-register encoder

**The smaller result is selected:** the source-built opt-in recipe is `m1-preserved-root-registers`. It emits **63,804 B**, saving **640 B** from the qualified 64,444 B predecessor. Code size is **1.105865× Roc** and **1.720712× libdeflate**. The checked entry shrinks **47,428 → 46,788 B**. Peak active stack rises **4,656 → 4,848 B**, within the unchanged 5,280 B ceiling. The 64,444 B recipe remains available as an earlier qualified alternative.

Both complete M1 orders record **81/81 Roc and 62/81 libdeflate point wins**, with **81 and 62 precision-supported wins in each order**. These cover **74/74 and 56/74 distinct contracts**. The unchanged v3 gate returns **Retain**, with no rejected or unresolved peer cases. The direct 64,444 B comparison has **no established slowdowns and seven uncertain cases**; their full intervals remain in the replay. This is a peer-qualified size gain, not a claim that every case improved.

[Full decision](full-screen.json) · [All per-case intervals](full-replay.json.gz) · [Raw forward observations](full-forward-observations.tar) · [Raw reverse observations](full-reverse-observations.tar)

## Complete build timing

| Same-session Apple M1 build | Encoder text | Source to link | Source to strip | Source CPU | Source peak RSS |
| --- | ---: | ---: | ---: | ---: | ---: |
| 64,444 B control | 64,444 B | 2.237373 s | 2.248259 s | 2.256837 s | 393.2 MB |
| Selected preserved registers | 63,804 B | 2.239043 s | 2.249887 s | 2.258899 s | 393.2 MB |
| Roc | 57,696 B | 2.984684 s | 2.995503 s | 2.940046 s | 395.8 MB |
| libdeflate | 37,080 B | 0.480820 s | 0.491652 s | 0.443041 s | 76.1 MB |

Eight balanced fresh builds per actor use one prebuilt compiler and the same prebuilt deployment support. The selected build takes **0.750372× Roc** and **4.654599× libdeflate** source-to-link time. Its paired median cost is **+1.194 ms** versus 64,444 B, with **3/8 faster** and a paired-trial bootstrap 90% interval of **−6.170 to +12.180 ms**. These data establish neither a compile-time gain nor a slowdown. The size gain is kept. Paired source CPU cost is **+2.852 ms** and native-phase cost **+0.122 ms**.

Stripped deployment remains **276,072 B**; the 640 B code saving does not cross a linked-file size boundary. Remaining elapsed gaps are **0.239043 s to 2.0 s**, **1.239043 s to 1.0 s**, and **1.758223 s to this cohort's C build**.

Compiler phase medians are **0.707869 s parsing**, **0.220838 s source semantics**, **0.006840 s artifact creation**, **0.002494 s encoding**, and **1.232968 s native emission**. Catalog join wait is **0.000019 s**, with catalog CPU still running during parsing. Link takes **0.035739 s**; strip takes **0.010787 s**. Median per-trial source time outside explicit scopes is **0.028061 s**. Independently calculated medians need not add.

[Complete measurements](deployment.json) · [Every build timing](build-timings.csv) · [Raw build acquisition](build-comparison.json.gz) · [Inputs and balanced orders](build-plan.json)

## Source and correctness

Compiler source is `6dc64cff731a7388fb1239945b3aa4eaf2ace0ce`, compiler SHA256 `bf4f2ead7fa0f00e9b765cd7ffdaf4f3f3d82ff5ecbf8b9fb06a2282515623a8`. The option reverses only the existing preferred/nonpreferred integer register sets for eligible Apple AArch64 memory-backed checked roots. The allocatable union, pinned/scratch exclusions, call clobbers, ABI preservation and allocation checker are unchanged. It defaults off outside this named recipe. The original Mech program and canonical bytecode are unchanged; no LLVM dependency is added.

All **five focused native/validation tests** pass, including 18 opaque results live across imported calls under both preferences and optimization modes, outputs, sentinels and resetting the function attribute. Linked checks pass **162 exact output/decompression/alias observations** and **six injected allocation-failure cases**, with matching origins and cleanup. The named source command additionally passes **12 exact recovery calls**.

From the benchmark packet directory in a full PR #995 checkout:

```sh
python3 -B rebuild.py /tmp/mech-m1-preserved-root-registers \
  --variant m1-preserved-root-registers --mimalloc-object /path/to/mimalloc.o
```

The complete runtime qualification belongs to fixed-support worker `5afce59aa83c03df81c693d78e9a65f17df1c88c64a7e04c490c58ecc4fffba0`. The freshly rebuilt host-support worker is `2e0f926c0799786bbd27f8006f9ea9d1cc87c42af9f55975d1159465145b7c48`; its runtime wins are not measured by the rebuild command. Source checks, worker identities and fixed deployment receipts are distinct.

[Recipe, source and profiles](candidate.json) · [Named rebuild receipt](source-rebuild.json) · [Successes](source-successes.json) · [Failures](source-failures.json) · [Recoveries](source-recoveries.json) · [Linked contracts](linked-checks.json) · [Original plan](plan.md)

## Controller architecture remains incomplete

The original brief already had separate numeric kernels. Existing work shares **three controller bodies and six continuation bodies across 53 sites**. The new **46,788 B checked entry remains large**; this allocation change does not complete the planned small direct controller and bounded continuation regions.

The preceding 64,444 B root census finds 11,857 instructions, including 2,106 direct-SP `ldr`, 857 `str`, and 2,084 `mov`. Its IR has seven explicit stack slots totaling 560 B, while its native frame is 3,104 B. These static counts explain the allocation hypothesis; they do not attribute runtime. Original disassembly, IR and region reports are archived. The next structural pilot should extract one substantial checked fallback outside an inner hot loop, retaining exact live-in/out, ownership, partial-work resume and error contracts and total active-stack limits.

The earlier focused `Unresolved` label meant complete timing orders were still required. Both full orders are now complete and return `Retain`. Earlier harness setup errors and all raw focus data remain beside the full evidence. [Initial focused decision](focus-screen.json) · [Validation logs](validation-logs.tar.gz)
