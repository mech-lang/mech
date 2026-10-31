# First-violation call budget checks: measured result

Base **545e17bf065028512c93a2bdc8677fbc3728a3fa**; production **299c70f4ce21eb3cba94dcb47e4d37106cc14b29**; correctness/code-generation packet **3a6953921**; measured checkpoint **e6a1749ee2e1dcb14254546d1f0685edd800ff6e**. IR/work/ABI/bytecode remain125/93/36/14. No production/source change follows the frozen default build. Core349,Engine227,component210,selected scalar8,bits3 = **797 unique scoped Rust passes** before timing; all12 architecture checks pass. Production rebuild1017.391s, same selected operation/value profiles, no catalog expansion.

**Roc = 1×: 67.315086207× → 65.708479310×.** Mech **19.5213750ms → 19.0554590ms**, fresh common reference **0.290000ms** (20000 observations). **2.386697% less elapsed; 1.024451× speedup.** All four pairs improve. This is a modest gain; the representative C-equivalent assembly is not a large end-to-end improvement. Previous67.879005245× used its own0.286ms reference; this is a new paired experiment, not a continuous normalized series.

|Pair|Order|Before ms|After ms|After / before|
|---|---|---:|---:|---:|
|1|before → after|19.551167|18.944375|0.968963899|
|2|after → before|19.528458|19.058334|0.975926210|
|3|before → after|19.302875|18.982917|0.983424334|
|4|after → before|19.593167|19.124208|0.976065176|

Four fixed alternating-order process pairs,three primary observations per control per process,twelve/control. All40 process observations remain:24 primary,eight warm-ups,eight excluded profile-tagged. The separate210-repetition sampled process contributes no primary observation. No compiler/test/audit/other benchmark overlaps protected primary or its sample. All observations retain exact ten raw bytes,independent inflation,capacity13 and final zero bytes/nodes. Output backing changes and its copies remain timed. Same pinned mimalloc2.1.7 and evaluate_into-only harness; JIT,setup,seed,clear,rebind,verification anddrop remain excluded. Existing diagnostic NoLimits stays.

**Logical work is identical:222049095608.** Prices,allowances,source operations,all25 components,algorithm/tables,262144-entry U16 tree,1500513-word U32 cache,depth300,nice258,near-EOF rules,source allocation sites andtiming boundaries stay unchanged. Mech performs compression itself. Largest complete input remains eight bytes;no confidence interval orlarger-input throughput claim. Frozen before SHA7c39895dc9af8de0b6d33c77eb0003edb34ccd98b6cfbe526e25995a9ec2093e;after SHA1f9977d1fcf16eb83821be3ff14d689dc1f88d05174f87d6721cc42e1c2f5826. Exact source/artifact/reference/allocator/harness/library hashes and allraw observations accompany [the manifest](final-primary/manifest.json).

The earlier [reference-only attempt](ABORTED-TIMING.md) stopped before spawning Mech because copied executables lacked execute mode. Its reference observations and modes remain separately committed. The corrected experiment preserves execute modes, verifies byte-identical qualified compiler/libraries,and uses a fresh reference. No failed Mech samples are replaced;none existed.

Fresh profile partitions1315 disjoint exclusive ancestry weights: {'remaining': 186, 'other_invocation_helper': 202, 'selected_library_call': 280, 'ordinary_control_visit': 266, 'buffer_frame_actions': 381}. The complete budget evaluator has18 exclusive weights versus58 in the prior recorded profile. Frame-input admission has141 inclusive weights. The earlier profile is separately sampled,not a paired profile. Shares are not removable elapsed-time guarantees. Raw stacks and the partition script remain. Larger remaining frame/collection/visit costs need a broader compiler improvement; no second optimization is mixed into this result.

This timing/profile packet is committed immediately before protected scalar guards oranother compiler change. The1× goal remains active.
