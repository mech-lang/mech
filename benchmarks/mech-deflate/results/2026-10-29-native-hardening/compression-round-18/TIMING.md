# Opaque siblings in private frames: paired eight-byte timing

Reviewed base d48231ef0413b05e455576217d10898b3544e7fa. Production compiler **1d2ef606f0cb576eee343b4806fd1c75e69a7b7a**; baseline986601834c and test-packaging correction8b04906ed. Prior executable is the retained round17 control (production1388b4da98174ca3493ca65cdafee3b44f876f7f). Existing draft #973; parent eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b. This packet is committed before further optimization.

| Complete eight-byte compression | Before | After |
| --- | ---: | ---: |
| Median evaluate_into |33.103167ms|32.011084ms|
| Multiple of same fresh reference (Roc=1×) |114.941552×|111.149597×|

Observed reduction 3.299029%; speedup 1.034116×. Reference median0.288000ms from20000 observations;3 primary observations per Mech control. Prior round17 after was113.747852× under its own fresh reference. Warm-ups and the separate125-repetition profile are excluded. These small samples do not establish a confidence interval or larger-input throughput.

All observations preserve exact ten-byte raw output, independent inflation, capacity13 and final zero bytes/nodes. Output backing still changes, and copies remain timed. Same pinned mimalloc2.1.7; no compiler/test/benchmark competes with primary. The pinned source/bytecode,25 components, algorithms/tables,262144-entry U16 tree,1500513-word U32 cache,depth300/nice258,near-EOF behavior,6 Huffman initialization and source allocation sites/timing boundaries remain unchanged. Mech performs compression itself; no larger full-compression input ran and no allowance changed.

IR120 admits opaque canonical tuple/record siblings in the existing managed-owner frame lanes. Complete-component use checks still reject a child consumed by another pack, whole canonical escapes and unsupported producers. This does not flatten nested private frames. Current schema/shape/owner/selector, finite resource admission, cleanup and atomic publication remain existing runtime operations. Work policy93 prices, ABI35 and bytecode14 remain unchanged. Reached route selection changes total work from303735923752 to308968619891 (delta5232696139); this is not an equal-debit claim.

The3 new permanent regressions pass before timing, covering U64/U8 small/real capacities, original/decoded artifacts and image rederivation, fresh scalar compound bits, assertion-before-arithmetic order, overflow/division, unchanged prior publication/aliases, cleanup and retry. The release build passes in450.902s on nightly2026-03-03,aarch64mac,opt3,jobs1,locked/offline. Production remained frozen during the build. The initial unsafe audit caught test evidence packaging; the correction shares the approved image helper and compresses the historical source without changing its original hash. The corrected12 static checks pass without widening the allowlist.

Broader existing component, Engine/scalar/Object/image and capacity witnesses follow in the closure; no further semantic optimization precedes qualification and review. Retained unresolved scalar observations: round10 decoded-U641.0526036; round17 initial original zero-step loop1.052454987, whose fixed balanced follow-up did not reproduce>5% regression. Neither observation is erased. The1× objective remains active. CI is cancelled by user direction, not green.
