# Native compression: current status

## R135: retire historical temporary state at each lexical traversal

Update base: `6f009e8db6b02c5aa5d4c182d009dfdb560b0bc2`.
Control runtime: `61b7d0b51a1ef17daf97992844c5f413b3203400` (R134).
Measured compiler: **`5a0d926a2ead208466bc8aa13a4dad301aaeb5ef`**.
Test-first witness: `dfb9a1cd1e`. IR/work/ABI/MBC: **164/107/39/14**.
Stack base remains `eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`.

The historical roughly11× random1KB result and558–674× first32,769B results
used different inputs. The larger case crosses the first32KiB history boundary
and exposed additional compiler costs. Those figures were not a same-input
regression. Preserve both input size and data pattern beside every multiple.

Fresh instruction-address profiles of the real R134 binary found2188/2535
timer samples at stack memory instructions and191 at stack-offset constants;
only2 sampled divisions. The real collector registers3590 SSA words. Complete
boundary snapshots kept historical local temporaries live across root visits,
even when the selected source defines those values afresh before any read.

Preparation now reuses canonical current-traversal liveness. A candidate must
have an immutable scope owned by a selected loop, all sealed definitions inside
the selected region, all read roles inside that region, and no live input at the
root header. Only proved scalar, private-frame scalar and scalar-aggregate lanes
receive a fresh SSA constant at each root header. Current carriers, captures,
arrays, private ownership, fuel and counters retain their existing transport.
No canonical data or buffer is zero-filled; source operations and their original
checks/charges still execute. Images independently reconstruct and compare the
list; old IR163 is rejected. Public bytecode remains revision14.

The collector proves **3004 of3590 words** fresh. Generated text falls
**39,089,980→29,384,768B** and entry stack **248,400→126,720B**. Object SHA256:
`f1ada5f63545dbd1ff94e58d9773bc0f565bd1d55ce0f7413f624ea6164ecbac`.
This is not complete exit-state narrowing or physical word-bank compaction.
The earlier R131 terminal-failure-only experiment remains rejected; this patch
changes the repeated SSA dependencies instead of replaying that experiment.

| Complete evaluate_into | R134 control | R135 | Control→current / Roc |
|---|---:|---:|---:|
|8B|1.6710835 ms|1.5669375 ms|5.842949×→5.478802×|
|Random1KB|3.478187 ms|2.962771 ms|11.219958×→9.557326×|
|Repeated1KB|5.000271 ms|4.056729 ms|15.601470×→12.657501×|
|Random32,769B|70.0845835 ms|54.861896 ms|66.810852×→52.299234×|
|Repeated32,769B|110.4644375 ms|83.071521 ms|83.243736×→62.600995×|

Fresh reference medians are0.286 /0.310 /0.3205 /1.049 /1.327ms. Elapsed improves
**6.232244% /14.818525% /18.869817% /21.720451% /24.797950%**. Every paired block
improves:32/32 at8B across2 prepared pairs,8/8 for each other case. Counts/version
are992 at8B,128 at each1KB and16 at each32,769B. Individual raw observations are
retained; improving blocks do not imply every observation wins. Each primary
cohort was committed immediately. Do not compound independent rounds.

| Separate hardware cohort | R134 instructions | R135 instructions | R134 cycles | R135 cycles |
|---|---:|---:|---:|---:|
|8B|16,941,035.5|15,800,852|5,347,441|4,927,631|
|Random1KB|58,743,876.5|37,665,902|11,081,454|9,259,969.5|
|Repeated1KB|84,217,912.5|62,561,263.5|15,839,959|12,744,920|
|Random32,769B|1,454,127,249|814,977,088.5|226,114,265.5|174,928,244|
|Repeated32,769B|2,258,279,496.5|1,617,196,133|352,322,997|266,563,954|

Hardware uses32 calls/version small and4 large. Large instructions fall
**43.954211% /28.388132%**, cycles **22.637237% /24.341029%**. Small instructions
fall6.730306% /35.881143% /25.715015%. Separate heap cohorts use4 calls/version
small and2 large. All five allocation counts, requested bytes, reallocations and
incremental live usable peaks are unchanged. Large peaks stay
**15,314,280 /15,313,264B**. Smaller machine stack is not a measured heap saving.

Fresh2-second actual-executable samples put **94.740796% /94.506949%** of CPU
time in generated code. A separate raw-PC capture finds2158/2494 timer samples
at stack memory instructions and145 at stack-offset constants;1 at division.
The optimized nondebug reference diagnostic finds245/2356 at stack instructions
and1419 at other memory instructions. These are time-based sampled opcodes,
not retired-instruction partitions; the remaining Mech stack traffic is still
the next evidence-backed target. The diagnostic reference wrapper runs4096
complete calls instead of256, preserving all14 compressor sources. Two inspected
function opcode streams agree with the actual primary reference binary; no
whole-binary identity claim is made. Earlier debug-wrapper profiles remain as
historical evidence and are superseded for this comparison. Primary timings
always use the original retained optimized reference executables.

Validation: **544** compatible GPU tests with the same7 inherited exclusions;
**119** native CI contracts inventoried and executed; unchanged **236** components,
**8** connected and **1** path case; **132** workflow tests; **15** architecture/
catalog checks, runtime-only metadata, formatting and warnings as errors. GPU
opt3 and unit-root opt1 use nightly2026-03-03 aarch64 macOS and frozen dependency
libraries. New controls cover current Boolean/numeric arguments, nested/repeated
visits, private writes, finite errors, unselected invalid children, unchanged
publication and aliases, cleanup/retry and forged/reordered/missing image proof.
The clean parent passes the semantic control and fails the2 physical/proof
expectations; all3 focused tests pass after the patch. No fresh full/minimal/
no_std dependency or linked40/scalar12/frame8 suite is claimed.

The initial worker link failed with environmental errno28 after successful
image emission. Verified lossless compression of completed historical builds
freed3.54GB; the link-only retry passes all5 exact complete-call witnesses.
Current controls and libraries were retained. Initial diagnostic cache permission
failures were corrected with authorized execution; all failed receipts remain.
No introduced correctness failure remains. Hosted CI is cancelled by standing
instruction, with no green hosted-CI claim; latest-head review remains pending.

Pinned source/bytecode, algorithm/tables, depth300/nice258, near-EOF rules, source
allocation sites,262144U16/1500513U32/305257U64 scratch, allocator, input aliases,
output13/1029/32804 clear/reuse and evaluate_into boundaries are unchanged.
All cases match exact pinned raw bytes and independently inflate. Large logical
work stays129,288,746,789 /270,676,915,001; final managed bytes/nodes are zero.
No full-corpus timing was attempted. Parity, broader exit/entry liveness,
physical compaction and separate source variants remain unfinished.

Reproduction commands, raw counters/timings/profiles, source requirements,
clean-parent failures, qualification and failure dispositions are indexed in
`benchmarks/mech-deflate/experiments.json`, round135. Diagnostic archives exclude
native binaries, libraries, Objects and bytecode. All **12** packets
(13,342,925B) and their receipts are published with server SHA256 verification.

## R134: keep total unsigned collection bodies in native code

Update base: `3b22758fe95e36cc58882077c926efd88b28ec7b`.
Retained control runtime: `9d23729e5f48466f5b606bb00cec3a19be106469` (R133).
Measured compiler: **`61b7d0b51a1ef17daf97992844c5f413b3203400`**.
Test-first witness: `2dcca9a5eaece0303f2e2c031bea5e2c2064b8d7`;
exact unsafe-boundary audit contracts: `6c8719573a`.
IR/work/ABI/MBC: **163/107/39/14**. Stack base remains
`eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`.

The earlier roughly11× random1KB result and558–674× first32,769B results were
**different inputs**. The larger input reaches the original32KiB history slide,
which runs a pointwise function over262,144 U16 entries. R133 removed a linear
source-membership scan. This patch removes per-element host boundaries in that
map. Neither comparison is a same-input regression from11× to500×.

A separate exact callback census established1,310,721 ordinary source visits and
524,288 element/push helpers in this one rebase. Its scalar arithmetic was already
native. The generic compiler now derives an acyclic, total unsigned scalar CFG
from the canonical collection, captures, definitions, external uses and existing
scalar admission group. Unsigned ranges and selected comparison edges establish
checked subtraction and narrowing; redefinitions kill dependent facts and joins
intersect them. Bodies with reachable failure or unsupported operations retain
the ordinary path. No source function name, field name or compressor site is a
selection rule. Existing shared scalar instructions, SSA words and terminators
emit the body; checked machine operations remain as defense.

At the original completed initializer, current source ownership, full original
work, component limits, memory/nodes and exact output geometry are checked once.
Insufficient funding declines before changes and keeps ordinary error precedence.
One allocation replaces geometric physical growth, preserving its final capacity,
source allocation site, original conservative peak and original logical debits.
The immutable source stays owned; the unique unsigned sink has length zero until
the synchronous total kernel succeeds. The ordinary final header, collection
finish, shape checks and canonical publication remain. Current captures reload
for each fresh invocation; no permission or argument cache crosses visits.
The image independently reconstructs and compares the complete proof. OldIR162
and ABI38 identities remain rejected. Public bytecode stays revision14.

| Complete evaluate_into | R133 control | R134 | Control→current / Roc |
|---|---:|---:|---:|
|8B|1.666250 ms|1.683312 ms|5.725945×→5.784577×|
|Random1KB|3.501771 ms|3.497042 ms|11.314284×→11.299005×|
|Repeated1KB|5.0222085 ms|4.9931875 ms|15.596921×→15.506793×|
|Random32,769B|175.503479 ms|71.0640625 ms|167.305509×→67.744578×|
|Repeated32,769B|213.886229 ms|110.3121875 ms|160.937719×→83.003903×|

Fresh reference medians are0.291 /0.3095 /0.322 /1.049 /1.329ms. Large random
improves **59.508459%**, repeated **48.424829%**, with16 observations/version and
8/8 improving paired blocks each. Each1KB version has128 observations; random
improves6 blocks with2 adverse and repeated7 with1 adverse. The eight-byte control
has992 observations/version across2 pairs: **1.023976% slower**, with25 adverse
and7 improving blocks. All adverse data remain. Every primary cohort was committed
immediately. Fresh reference variation changes reported multiples even where Mech
time barely moves; independent cohorts are not compounded.

| Separate hardware cohort | R133 instructions | R134 instructions | R133 cycles | R134 cycles |
|---|---:|---:|---:|---:|
|8B|16,808,015.5|16,940,821|5,272,424.5|5,217,365.5|
|Random1KB|58,608,768.5|58,745,051|11,152,852.5|11,131,206|
|Repeated1KB|84,141,946.5|84,217,093|15,813,304|15,756,970.5|
|Random32,769B|3,346,383,215|1,453,970,149.5|560,741,793.5|226,231,835.5|
|Repeated32,769B|4,152,832,313|2,258,330,693|683,870,299.5|352,947,516.5|

Hardware uses32 calls/version small and4 large. Large instructions fall
**56.550997% /45.619507%**, cycles **59.654900% /48.389700%**. Small instructions
increase0.790132% /0.232529% /0.089309%; cycles improve1.044282% /0.194089% /
0.356241%. The eight-byte elapsed and cycle observations disagree; its cause is
not established. No per-function instruction partition or fresh large reference
instruction/heap cohort is claimed.

Separate heap cohorts use4 calls/version small and2 large. Small requests,
requested bytes and live peak are unchanged. Large requests fall5263→5247 and
3428→3412; requested bytes fall16,547,794→16,023,514 and16,245,514→15,721,234.
That is16 requests and524,280 requested bytes saved in each. Peak live usable heap
stays **15,314,280 /15,313,264B**. Capacity requests are not copied/live byte counts.

The isolated exact after-census confirms **zero** per-element element/push helpers:
random visits1,322,760→12,040 and total helpers530,220→5,934; repeated visits
1,336,935→26,215 and helpers538,070→13,784. Prepare and finish each execute once.
Each diagnostic uses2 agreeing complete calls; its instrumented elapsed, heap and
hardware results are excluded. Actual ARM64 Object inspection finds2 source-map
kernels with36 static instructions each and **no calls**. Generated text is
39,089,980B (−5,088); entry stack248,400B (−288). Object SHA256:
`8dfac0c1600c6f9297c569e4b6c04722d2338aa2c1f89869877bd30a2634b139`.

Fresh isolated2-second actual-executable samples put **95.030120% /96.029120%**
in generated code, including the new named kernels. Remaining callback/cleanup
subtrees total4.743976% /3.772336%; outside/unrecognized samples are0.225904% /
0.198544%. These are disjoint CPU-time weights, not retired-instruction shares.
This shifts the next investigation toward the remaining generated entry's state
transport and repeated instructions; it does not establish a function instruction
ledger or complete the broader physical-state brief.

All source/bytecode pins remain
`331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9` /
`a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`.
Algorithms, tables, depth300/nice258, near-EOF limits, source allocation sites,
262144U16/1500513U32/305257U64 capacities, input aliases, pinned mimalloc2.1.7,
output capacities13/1029/32804 with clear/reuse and evaluate_into boundaries stay
fixed. Exact pinned raw bytes and independent inflate pass at8B/1KB/32,769B.
Large logical work remains129,288,746,789 /270,676,915,001. Final managed bytes/nodes
are zero. No full-corpus or full Reymont timing was attempted.

Validation: **541** compatible GPU tests with the same7 inherited profile
exclusions; **116** native CI contracts inventoried and actually executed;
unchanged **236** components, **8** connected cases and **1** path mechanism;
**132** workflow tests; **15** architecture/catalog checks, runtime-only metadata,
formatting and warnings as errors. GPU/runtime opt3 and unit-root opt1 use
nightly2026-03-03 aarch64 macOS and frozen Core/Engine/stdlib dependencies. No new
full/minimal/no_std dependency or linked40/scalar12/frame8 suite is claimed.
The new5 tests cover all unsigned widths, empty/power-of-two/odd extents, changing
numeric/Boolean captures, original/decoded/restored images, ordinary work equality,
fallible subtraction/narrowing, live aliases, publication preservation, finite
boundary denial and recovery, and forged/missing proof rejection. The clean
parent completes the value check then fails the structural witness with6 element
callbacks instead of0. The compiler candidate passes it.

All introduced implementation/build issues are corrected. The first draft
witness incorrectly requested unsupported mixed-argument automatic lifting;
its valid explicit comprehension replaces only the test. Sealed-value and offset
import compile errors are archived. The architecture audit caught2 missing exact
sink-boundary entries; the narrow contracts were added without weakening it.
A copied linked harness still included the old descriptor; strict rejection
caught it, and only that path was corrected. Old inventory/config assumptions
in local scripts were fixed before their dependent timings. Failed attempts and
all actual reruns are retained. No introduced correctness failure remains; the
small elapsed/instruction costs above remain explicit performance limitations.
Ordinary hosted CI cancellation is required by standing user direction; no hosted
success or clean latest-head review is claimed. Final external status belongs to
the publication receipt. The PR remains draft.

Raw diagnostics are hash-listed external release assets; native binaries,
libraries, Objects and bytecode stay local. The accepted broader exit/entry
liveness, storage compaction, separate source variants and allocation-lifetime
brief remains unfinished. This checkpoint completes the measured generic unsigned
map path; it is not a parity claim.

## R133: remove a linear scan at every cold source visit

Update base: `6dcab422ee0ea364ec713060147a3e45686a9d9d`.
Control compiler: `f6f47a06a98c0aeb97cd596250676209897e9648` (R132).
Measured runtime: **`9d23729e5f48466f5b606bb00cec3a19be106469`**.
IR/work/ABI/MBC remain **162/107/38/14**. Stack base remains
`eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`.

The newly authorized **32,769-byte** random and repeated cases cross the first
32KiB history-window boundary and execute the original262,144-entry U16 rebase.
R132's first larger primary baselines were **674.336449× /557.604602× Roc**;
its smaller random1KB cohort was **11.481581×**. Those are different inputs,
not a same-input optimization regression. The small cohort hid a real cold-path
performance cliff. Both larger cases already agreed exactly with the reference.

Fresh R132 CPU samples put **77.753465% /72.992701%** exclusively in
`Invocation::numeric_source_prepaid`. Actual ARM64 hot PCs at offsets160/164
sit inside a SIMD linear scan of the collector's **4,574** source sites.
The fallback inventory, like the separate interval/prepaid inventory, already
comes from a `BTreeSet` and is reconstructed independently on image load.
The runtime now binary-searches that fallback inventory. It retains the same
choice of inventory, current region, deepest paid activation and ancestor walk;
no permission, charge, admission, source visit or current-account check is skipped.
The comparator is ordinary `usize` identity; no new cache or allocation is added.

Tests were strengthened first in `130c60dd66ef9a23c341a8d2939ead800427db45`.
Both complete-source and interval images reject reordered and duplicated
membership; complete-source images also reject omission. Those controls pass on
the parent. Existing original/decoded/restored child funding, finite denial,
current writes, lexical scope, aliases, unchanged publication and retry controls
pass before and after. The implementation changes one lookup, with a comment.

| Complete evaluate_into | R132 control | R133 | Control→current / Roc |
|---|---:|---:|---:|
|8B|1.651958 ms|1.6986875 ms|5.696407×→5.857543×|
|Random1KB|3.530000 ms|3.508979 ms|11.065831×→10.999934×|
|Repeated1KB|4.999000 ms|4.9718965 ms|15.240854×→15.158221×|
|Random32,769B|714.473042 ms|176.555146 ms|678.834244×→167.748357×|
|Repeated32,769B|746.953187 ms|214.904542 ms|559.934923×→161.097858×|

Fresh reference medians: **0.290 /0.319 /0.328 /1.0525 /1.334 ms**.
Large random improves **75.288760% (4.046742×)**, repeated **71.229182%
(3.475744×)**. Each has16 observations per version and8/8 improving paired
blocks. Each1KB version has128 observations; random improves7 blocks with1
adverse, repeated8 with0. The eight-byte control has992 observations per version
across2 prepared pairs and32 blocks, all adverse: **2.828734% slower**. This cost
is retained explicitly rather than discarded. Independent cohorts are not compounded.

Each primary cohort was committed immediately: random/repeated large
`6b64265c8e` /`0678f7a821`, eight bytes `ea5b751ad3`, random/repeated1KB
`7fed47cc69` /`6fe63f515a`. These paired measurements use the actual retained
R132 executables, not a rebuilt estimate. Preparation, clear, rebind, aliases,
validation, independent inflate and cleanup remain outside evaluate_into.

| Separate hardware cohort | R132 instructions | R133 instructions | R132 cycles | R133 cycles |
|---|---:|---:|---:|---:|
|8B|16,809,749.5|16,808,302.5|6,856,780.5|5,205,889.5|
|Random1KB|58,609,360|58,607,405.5|11,238,757.5|11,135,631|
|Repeated1KB|84,140,882|84,139,353.5|15,942,780|15,839,269.5|
|Random32,769B|13,804,631,175|3,346,485,535|2,273,383,051|561,269,864|
|Repeated32,769B|14,611,094,517.5|4,152,998,871|2,395,461,657.5|684,759,970|

Hardware:32 calls/version for each small case,4 for each large. Large instructions
fall **75.758240% /71.576401%**, cycles **75.311250% /71.414280%**.
The eight-byte instruction count is nearly unchanged and its separate cycle
cohort improves, contrary to its primary elapsed regression. Both observations
remain; the cause of that small-case elapsed cost is not established. Hardware
windows are per-call counters, not per-function instruction attribution.
No new larger reference instruction/heap cohort is claimed.

Heap is measured separately:4 calls/version for small cases and2 for large.
Requests, requested bytes and peak live usable heap are identical before/after:
**3,350 /3,157 /3,392 /5,263 /3,428** requests;
**17,639,393 /15,154,378 /17,646,983 /16,547,794 /16,245,514** requested bytes;
**15,279,752 /15,278,216 /15,282,336 /15,314,280 /15,313,264** peak usable bytes.
This is an instruction/control-flow reduction, not an allocation or memory win.
The generated Object and descriptor are byte-identical: text **39,095,068B**,
entry stack **248,688B**; no compressor re-emission was needed. GPU/runtime alone
was rebuilt with the same frozen dependencies and warnings as errors.

Fresh two-second actual-executable CPU samples show the lookup falling to
**4.679245% /3.373016%**. Generated code is now **41.283019% /51.058201%**
exclusive; the remaining disjoint callback/cleanup shares include useful algorithm
work as well as overhead. CPU sample weights are not retired-instruction shares.
The actual runtime function changes from181 to138 statically disassembled
instructions; its SIMD inventory scan disappears. This is not a dynamic count.

The same source/bytecode pins remain
`331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9` /
`a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`.
All14 reference source files were recovered and hash-verified against the original
671077457ff7ef3a3d44caf715a3fc595addf379 pin. Large random raw output is32,774B,
SHA256 `ee86a84061f43999d5000831832c615f29db244ab4e19d0cdddedd85bc3553a1`;
repeated is47B, SHA256
`8c359ea8a8ab0873b34186db8ee504220d4df00a850fcfe323365767ab74b1f4`.
Independent inflate, input aliases, original depth300/nice258/near-EOF rules,
262144U16/1500513U32/305257U64 scratch, pinned mimalloc2.1.7 and output clear/reuse
all pass. Large output capacity is **32,804**, from the original
`n +5*max(1,ceil(n/5000))` bound. The initial harness incorrectly extrapolated
small-input `n+5`; its rejection is archived, then the harness was corrected.
No compressor allocation or allowance was changed. Large logical work remains
**129,288,746,789 /270,676,915,001**, distinct from hardware instructions.
Final managed bytes/nodes are zero.

Passing checks: **536** compatible GPU tests with the same7 inherited exclusions;
**111** CI tests inventoried and separately executed; unchanged **236** components,
**8** connected cases and **1** structural path case; **132** workflow tests;
**15** architecture/catalog checks; runtime-only metadata, formatting and
warnings as errors. Selected GPU library uses opt3 and the maintained AOT/JIT/
canonical program features; unit root uses opt1, nightly2026-03-03 aarch64 macOS.
Frozen Core/Engine/stdlib/math were reused. No new full/minimal/no_std dependency
or linked40/scalar12/frame8 run is claimed. The two initial parent filters that
selected0 tests were corrected to registered nested modules; all11 actual parent
and candidate controls passed. Their receipts are preserved. No introduced
correctness failure remains. The eight-byte elapsed regression remains a measured
performance cost. Ordinary hosted CI is canceled under standing instruction;
the PR stays draft. Raw diagnostics are external hash-listed release assets;
native binaries, libraries, Objects and bytecode stay local.

The broader brief is unfinished: exact scalar/aggregate exit liveness beyond
bounds, true entry initialization, storage compaction, separate Huffman/header
pricing and histogram source variants, early private-write branching, fresh
allocation-stack census and evidence-guided binding/materialization/storage work
remain. The requested bounded history-boundary check is now established. Fresh
large profiles split remaining cost between native state transport/computation
and ordinary callbacks; source-specific callback counts should guide the next
bounded lowering step. No complete-corpus, full Reymont, all source-variant
multiblock/degenerate or parity claim is made.

## R132: exact state at native bounds failures

Update base: `b4e6e1601349c694eb0a6a9ffca2d190e06126b0`.
Measured compiler: **`f6f47a06a98c0aeb97cd596250676209897e9648`**; retained
control: `11f0ea49d2ba577bf18b563a566f325ac7957c7f` (R130).
IR/work/ABI/MBC: **162/107/38/14**; stack base remains
`eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`. Dependencies and compressor source
remain frozen. R131's shared terminal-failure narrowing remains reverted.

At a native bounds miss the existing mode2 consumer settles the current carrier,
finishes its private arrays at their current lengths and materializes the exact
managed operand. The original selected reader/replacement consumes its current
scalar arguments and returns the same source error. It never continues ordinary
source work. The compiler now spills exactly those inputs: readiness/fuel,
carrier scalars, traversal counters, append lengths and current scalar operands.
The shared exit inventory retains all original ordering and duplicates for the
other modes. Success, root bailout, child funding, ordinary continuations and
first-change transport are unchanged. No source check, account authority,
argument-capacity requirement or conservative price is removed.

The real collector's **293** bounds edges each saved **3,590 words**. They now
save **46 words at243 edges and47 at50**, removing **1,038,342 statically emitted
snapshot stores** from those edges. These are static counts, not executed stores
or hardware instructions. The full 3,590-word SSA inventory remains; this is one
exit family in the broader physical-liveness brief.

| Complete evaluate_into | R130 control / Roc | R132 / Roc | Change |
|---|---:|---:|---:|
| 8 bytes | 5.784984× | 5.717743× | 1.162343% faster |
| Random 1KB | 11.684204× | 11.481581× | 1.734163% faster |
| Repeated 1KB | 15.078916× | 15.248975× | 1.127799% slower |

Matched elapsed: **1.6776455→1.6581455 ms /3.5812085→3.5191045 ms /
4.908187→4.9635415 ms**. Fresh reference medians are
**0.290/0.3065/0.3255 ms**. Eight bytes has 3,968 primary observations, four
improving prepared pairs, 50 improving and14 adverse blocks. Each1KB version has
128 observations; random has8 improving blocks, repeated has8 adverse blocks.
Timing commits: `3965d4232f`, `92cba42fdf`, `237386f975`. All adverse data remain.

| Separate hardware cohort | R130 control | R132 |
|---|---:|---:|
| 8B instructions | 17,294,128.5 | 16,809,468 |
| Random instructions | 59,474,645.5 | 58,609,213.5 |
| Repeated instructions | 83,568,627 | 84,140,476.5 |
| 8B cycles | 5,237,517.5 | 5,103,013.5 |
| Random cycles | 11,428,296 | 11,116,953.5 |
| Repeated cycles | 15,542,014.5 | 15,843,489.5 |

Hardware has32 calls/version/input; heap has4, separately. Requests remain
**3,350 /3,157 /3,392**, requested bytes
**17,639,393 /15,154,378 /17,646,983**, peak usable heap
**15,279,752 /15,278,216 /15,282,336**. Text falls
**58,489,744→39,095,068 bytes (33.159106%)**; entry stack grows
**246,624→248,688 bytes (+2,064)**. The patch is retained for the code-size
reduction and two elapsed gains, with the repeated-input elapsed, instruction
and cycle regressions explicitly retained as its cost. This is a measured
tradeoff, not a uniform elapsed improvement. Full Object emission took221.81s
outside primary cohorts; concurrent qualification and old build cohorts prevent
a controlled compilation-speed claim.

Fresh isolated two-second CPU profiles still place the dominant work in the
generated body. Random control→candidate is **70.94→68.57%**, repeated
**72.74→70.74%**. Callback shares are separate, useful algorithm work is included,
and percentages are not a retired-instruction partition. Representative sampled
PC excerpts still show substantial stack transport; their static instruction
windows are archived with actual binary hashes. The reference binary and its
retained typed-store/vector-subtraction assembly were hash-verified again.

Passing qualification: **536** compatible GPU tests (same7 inherited profile
exclusions); **111** CI contracts both inventoried and executed; unchanged
**236** components, **8** connected cases and **1** structural path case;
**132** workflow tests; **15** architecture/catalog checks; runtime-only metadata,
formatting and warnings as errors. The focused original/decoded structural
witness fails on the parent, then passes. The retained ordinary/native/restored
post-append controls preserve exact arithmetic/assertion/read errors and origins,
both assertion/read orders, previous publication, aliases, cleanup and retry.
Finite-denial and private-operand cases pass in the current qualification.
No fresh full/minimal/no_std dependency, Core/Engine, linked40, scalar12 or frame8
execution is claimed. Ordinary hosted CI remains outside qualification by user
direction; the PR stays draft.

All complete-call cases preserve source hash
`331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`, bytecode hash
`a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`, exact raw bytes,
independent inflate, input aliases, original tables/search/near-EOF rules, real
scratch capacities, output13/1029 with clear/reuse, mimalloc2.1.7 and evaluate_into
boundaries. Work remains **18,067,825,665 /18,264,396,817 /25,477,511,424**;
final managed bytes/nodes are zero. The allocation-counter link initially failed
with **errno28 (disk space)**. SHA-verified lossless archiving of34 completed older
executables freed **4,773,437,293 bytes**, and its link-only retry passed. The
transient formatting diff was corrected before commit. No semantic failure
remains introduced by this patch.

The newly accepted brief remains a larger assignment. Next: preserve scalar
continuation liveness from the existing analysis, expand exact aggregate lanes,
finish exit-family recipes with image reconstruction, then remove internal entry
loads and compact storage. Independently develop deferred Huffman/codeword/header
pricing and shared histogram variants, measure private writes with an early
private branch, and take a fresh allocation census. Keep compiler-only source
pins; measure application variants separately and together. Add random/repeated
history-boundary cases above32KiB after this retained checkpoint. Source pricing,
entry compaction, other exit families and that scale check are not complete yet.

## R131: rejected terminal-error snapshot experiment

Update base: `27e7cc4c4c51370d5beb6473a2a9ff6a39df1b0b`.
Measured candidate: `e79a6272aff4a06ebf5fb3fa3dc8f78be23c363a`;
retained control: `11f0ea49d2ba577bf18b563a566f325ac7957c7f` (R130).
Candidate IR/work/ABI/MBC: **161/107/38/14**. The compiler change is **rejected**.
Production emission and identity are restored to R130; the new semantic error
controls remain. Source, bytecode, dependencies, resource prices and work are
unchanged. R130 remains the performance baseline.

The candidate factored the existing exit-word inventory without changing its
order/content, then used the existing subset spill only at the shared terminal
numeric error. The collector's snapshot fell **3,590→45 words**, but changing
that SSA sink adversely changed generated transport and register allocation.
This hypothesis did not survive measurement; smaller snapshots or Objects alone
are insufficient acceptance evidence.

| Complete evaluate_into | Retained control / Roc | Rejected candidate / Roc | Change |
|---|---:|---:|---:|
| 8 bytes | 5.806517× | 5.931372× | 2.150251% slower |
| Random 1KB | 11.889096× | 31.422512× | 164.296901% slower |
| Repeated 1KB | 15.083911× | 71.138462× | 371.618154% slower |

Elapsed medians: **1.6780835→1.7141665 ms /3.602396→9.521021 ms /
4.902271→23.120000 ms**; fresh references **0.289/0.303/0.325 ms**.
The 8B cohort has 3,968 primary observations, zero improving pairs of four,
four improving and 60 adverse blocks. Each 1KB version has 128 primary
observations and zero improving blocks of eight. Results were committed after
each timing: `6e167036fc`, `add6e0f0dd`, `57519ad345`. All observations remain
archived; there is no replacement or historical compounding.

| Separate counter | Control | Rejected candidate |
|---|---:|---:|
| 8B instructions | 17,294,534 | 16,961,845 |
| Random instructions | 59,474,745.5 | 81,956,091.5 |
| Repeated instructions | 83,567,633.5 | 192,367,311.5 |
| 8B cycles | 5,359,468 | 5,392,603.5 |
| Random cycles | 11,415,994 | 30,257,213 |
| Repeated cycles | 15,553,697 | 70,705,588.5 |

Hardware has 32 calls per version/input; heap has four, separately. Allocation
requests remain **3,350 /3,157 /3,392**, requested bytes
**17,639,393 /15,154,378 /17,646,983**, and incremental peak usable heap
**15,279,752 /15,278,216 /15,282,336**. Machine entry stack grows
**246,624→431,056 bytes** despite text shrinking **58,489,744→44,240,292 bytes**.
No heap or code-size result overrides the execution regression.

Fresh isolated two-second CPU samples put generated-body exclusive time at
**69.35→88.03%** on random input and **71.14→92.31%** on repeated input.
Weights are **1,207/1,261** and **1,490/1,508**. Counts are respectively
2,050/1,026 and 2,050/514 complete calls, outside primary timing. Actual sampled
PC excerpts contain repeated large-offset stack-to-stack moves. These static
windows demonstrate surviving transport, not a weighted instruction ledger or
an exact dynamic attribution of the regression. The retained reference binary
and its typed-store/vector-subtraction assembly were verified again.

The parent structural witness fails as predicted; both candidate focused tests
pass, as do all **72** composition controls. Original/decoded/restored cases
compare the full ordinary error and origin after append, assertion/read ordering,
unchanged prior publication, input aliases, cleanup and recovery. All three
linked compressor witnesses retain exact raw bytes, independent inflation,
capacities13/1029 and zero final managed bytes/nodes. Work remains
**18,067,825,665 /18,264,396,817 /25,477,511,424**. The first link failed with
**errno28 (disk space)** after successful Object emission; losslessly archiving
old builds and retrying only the link completed it. This was environmental.
The broader component/architecture/CI suites were not rerun for this rejected
emission experiment; R130's qualification is not presented as new coverage.

The rollback preserves the semantic post-append test and schedules it in the
native CI group. The experimental structural test and changed production
emission are removed. The next bounded experiment will examine exact mode2
bounds-error consumers and current scalar operands while leaving the rejected
shared failure optimization disabled. Ordinary continuations, first-change,
child fuel, finite admission and error precedence retain their current paths.

## R130: retain collector state through nested native loops

Update base: `1f91efb8fa39b77183903c1f58421bc35ce8f0d4`.
Measured compiler: **`11f0ea49d2ba577bf18b563a566f325ac7957c7f`**; retained
control compiler: `ccb9aab277081555385e37b02d824d0c5ef3db3b` (R129).
Stack base remains `eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`.
IR/work/ABI/MBC: **160/107/38/14**. Non-GPU dependencies remain frozen.

The actual collector could select either nested Tail children or a partial
interval ending at every child. Its cold Collection rebase prevented combining
those capabilities. The compiler now selects synchronous Tail descendants with
current-account fuel and exact exits before non-Tail work. The unchanged main
collector has **26 included loops, 4,574 source sites and seven array bindings**;
its two Collection boundaries remain ordinary, taken source edges. Eligible
search/skip work and fixed-size stores stay in the same native region.

All child input bindings are registered before recursive return bindings are
checked. Otherwise an enclosing child's return could be rejected because the
deeper child's carrier had not yet been registered. Existing current-version
checks still reject older observable values. A nested cold exit freezes private
cells and exports its exact live values before restoring the actual active Tail
and its ancestors. Only those already-paid current traversals retain source
credit. The original builder initializes after the exit; no native view or
renewed allowance crosses it.

Entry transport now copies only current carrier scalars, array views/capacities,
private flags and phase counters. Internal frame lanes are defined by source
actions. Copying the full 1,080-word collector bank was unnecessary and exceeded
an existing ordinary instruction template in the real skip qualification. The
smaller transport passes that same template; bank admission, logical prices and
first-change transport remain unchanged. This adds no interpreter or separate
operation vocabulary. Growing child buffers remain outside this extension.

| Complete evaluate_into | R129 / Roc | R130 / Roc | Change |
|---|---:|---:|---:|
| 8 bytes | 5.865234× | 5.816840× | 0.825101% faster |
| Random 1KB | 49.171092× | 11.843616× | 75.913457% faster |
| Repeated 1KB | 33.860709× | 14.903710× | 55.985238% faster |

Matched elapsed: **1.6891875→1.675250 ms /15.046354→3.6241465 ms /
11.1063125→4.888417 ms**. Fresh references are 0.288/0.306/0.328 ms. The 8B
cohort retains 3,968 primary observations, three improving pairs of four,
45 improving and 19 adverse blocks. Each 1KB version has 128 primary observations,
with eight improving blocks of eight. Each cohort was archived and committed
immediately: `571b196aa6`, `18534a6ac2`, `8f1d8ba0f6`. Do not compound historical
comparisons or combine them with the independent counters below.

| Separate counter | R129 | R130 |
|---|---:|---:|
| 8B instructions | 17,692,873 | 17,294,323.5 |
| Random instructions | 233,212,208.5 | 59,473,864 |
| Repeated instructions | 170,308,639 | 83,569,207 |
| 8B cycles | 5,246,697.5 | 5,203,203.5 |
| Random cycles | 48,309,490 | 11,365,088.5 |
| Repeated cycles | 35,494,259.5 | 15,506,526 |
| Allocation requests, 8B/random/repeated | 3,425 /16,463 /11,632 | 3,350 /3,157 /3,392 |
| Requested bytes, 8B/random/repeated | 17,577,513 /17,431,450 /19,694,087 | 17,639,393 /15,154,378 /17,646,983 |
| Incremental peak usable heap, 8B/random/repeated | 15,206,024 /15,204,488 /15,208,608 | 15,279,752 /15,278,216 /15,282,336 |

Hardware has 32 calls/version/input; heap has four, separately. **Peak heap grows
73,728 bytes** in all three cases. Eight-byte requested bytes increase 61,880.
Requested capacity is neither initialized copy volume nor live memory.
Generated text grows **19,434,640→58,489,744 bytes** and machine entry stack
**176,320→246,624 bytes**. Retain these costs. The observed full Object emission
took 223.6 seconds outside performance cohorts; it is not a controlled compile
time comparison. The larger generated body is the next important concern.

| Complete-call diagnostic | 8B before→after | Random before→after | Repeated before→after |
|---|---:|---:|---:|
| Runtime callbacks | 4,348→3,886 | 73,441→2,880 | 32,850→4,001 |
| Native segment exits | 217→125 | 13,397→102 | 4,265→127 |
| Array aliases exported by segments | 277→149 | 16,516→132 | 2,277→149 |
| Ordinary changed replacements finalized | 56→32 | 4,115→26 | 1,085→35 |
| Private array finalizations | 140→142 | 134→131 | 1,170→153 |

These are exact event counts, not CPU shares or payload-copy counts. All selected
numeric regions report zero helpers on their successful interior/backedge paths.
Source price formulas remain policy107; the selected physical execution removes
many charged runtime transitions. Complete-call work is now
18,067,825,665 /18,264,396,817 /25,477,511,424, versus
24,338,249,385 /959,562,617,769 /474,303,448,072. These are resource-account units,
not hardware instructions, and no allowance was increased.

Fresh isolated two-second CPU samples show the bottleneck moving:

| Exclusive context | Random before→after | Repeated before→after |
|---|---:|---:|
| Generated entry body | 8.77%→67.48% | 30.66%→71.10% |
| Other callbacks | 75.89%→22.41% | 55.78%→19.06% |
| Private-frame callbacks | 4.72%→3.88% | 5.68%→3.81% |
| Tail cleanup | 9.11%→1.78% | 6.15%→1.61% |
| Outside/unrecognized | 1.52%→4.45% | 1.74%→4.41% |

Random has 1,186/1,236 sample weights; repeated 1,497/1,495. Each input has
514 retained control and 1,026 candidate complete calls so the faster candidate
outlives the sample window. These are CPU-time samples; do not multiply shares
by the separate retired-instruction counts. Callbacks include useful algorithm
work. The next target is the generated body's temporary-field transport,
register pressure and repeated checks, while retaining the remaining callback
costs in the ledger. The collector uses 3,590 SSA words. Static inspection also
finds its cold mode6 emitter copying 256 frame words across all exit recipes
although each actual Collection edge needs none of those additional frame lanes;
this is an opportunity, not a measured saving.

The retained reference binary was disassembled and hash-pinned. Its generated
compression code contains direct typed stores and a vectorized U16 saturating
subtraction/remainder loop. R130 adds control/ownership coverage, not SIMD.
The archived assembly is static evidence, not a dynamic instruction ledger or
a claim that every displayed path runs at level12.

Passing qualification: **534** compatible GPU tests with the same seven inherited
profile/registry exclusions; **109** native CI contracts both inventoried and
executed; unchanged **236** components, **eight** connected cases and **one**
structural path case; **132** workflow tests; **15** architecture/catalog checks;
runtime-only metadata, formatting and warnings as errors. New controls exercise
hot and taken root/nested Collection exits after writes, original/decoded/restored
artifacts, fresh nested arguments, aliases, prior publication, assertion rejection,
finite settlement denial, cleanup, retry and forged image edges/bindings.

Failure disposition: the parent fails the committed coverage reproducer
`82bab42456`; input-only transport fixes the introduced instruction-template
failure. Earlier structural observers expected a cold boundary that is now
native, and now check the expanded region and actual prepaid source visits.
An initial test returned a public tuple without the existing private-use bridge;
the corrected witness uses the same private-return boundary as the real caller.
No public-escape proof was weakened. Diagnostic library naming/patch-needle
errors were external tooling corrections. Linking initially failed with
`write() failed, errno=28`; 2.34GB of completed builds were losslessly compressed
and hash-verified, and only linking was retried. All comparison binaries remain.
No introduced correctness failure remains.

Source/MBC pins remain 331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9 /
a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7. The algorithm,
tables, depth300/nice258, near-EOF checks, source allocation sites, real tree/cache/
path buffers, output capacity/clear/reuse, input aliases, allocator and timing
boundary are unchanged. The largest timed input remains 1KB. Core/Engine/math/
Runtime/Stdlib were not rebuilt; no fresh full/minimal/no_std dependency graph or
linked40/frame8/scalar12 suite is claimed. Hosted CI is canceled under standing
instruction; latest-head review is requested and the PR stays draft.

Parity remains unachieved. Preserve this measured baseline and use the new
native-body profile for the next compiler optimization.

## R129: eliminate cleanup for unmaterialized native segment values

Update base: `9a3bc1b66340b70647f1824d6f91b48383af9b7b` (published R128).
Measured compiler: **`ccb9aab277081555385e37b02d824d0c5ef3db3b`**;
initial experiment: `aadf0447f84de3ad74206c59dfc0b9567ee30ad2`.
IR/work/ABI/MBC: **159/107/38/14**. Stack base remains
`eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`. All non-GPU dependencies remain frozen.

Native pieces already represent intermediate frames and array projections as
words and bindings, yet their lexical retirements still cleared empty storage.
The compiler now derives physical cleanup from that representation. A virtual
step pays its original per-value charge in its original order and granularity;
current input frames and actual owners keep their existing retirement. Existing
current admission proves aliases empty; omitted frame retirements additionally
check canonical emptiness. Native execution creates neither kind of owner, and
actual escaping values are installed after cleanup. This adds no batching layer,
permission cache, owner scan, source rewrite or guest execution loop.

Unchanged groups retain their original loop. Additional slot checks apply only
to frames whose retirement is actually omitted. This refinement removes the
first version's unnecessary checks and per-value dispatch on unaffected groups.
The cleanup cache is not serialized authority: image restoration rederives it
from the canonical plan, and rejects attempts to supply a forged cache.

| Complete evaluate_into | R128 / Roc | R129 / Roc | Change |
|---|---:|---:|---:|
| 8 bytes | 5.775431× | 5.811066× | 0.617001% slower |
| Random 1KB | 49.951618× | 49.416324× | 1.071626% faster |
| Repeated 1KB | 34.444147× | 34.324387× | 0.347695% faster |

Times are 1.674875→1.685209 ms /15.185292→15.0225625 ms /
11.228792→11.18975 ms. Fresh references are 0.290/0.304/0.326 ms. All 3,968
primary/3,984 total eight-byte observations remain, with 0/4 improving pairs,
18 improving and 46 adverse blocks. Each 1KB version has 128 primary observations;
random improves 8/8 blocks, repeated 7/8. These comparisons are separate from
R128's 17% gain and are not compounded into a combined timing claim.

| Separate counter | R128 | R129 |
|---|---:|---:|
| 8B instructions | 17,714,267 | 17,693,350 |
| Random instructions | 234,392,369 | 233,262,675.5 |
| Repeated instructions | 170,816,949.5 | 170,664,987.5 |
| 8B cycles | 5,215,137.5 | 5,257,834 |
| Random cycles | 48,384,517.5 | 48,387,026 |
| Repeated cycles | 35,510,933.5 | 36,030,820.5 |

Retain all adverse cycles, particularly repeated-input +1.464020%, despite the
separate primary elapsed gain. Hardware has 32 calls/version/case; heap has four,
separately. Requests stay 3,425 /16,463 /11,632; requested bytes stay
17,577,513 /17,431,450 /19,694,087. Incremental peak usable heap stays
15,206,024 /15,204,488 /15,208,608 bytes; reallocations remain 68. Requested
capacity is not copied or live bytes.

The complete generated text is **byte-identical** to R128: 19,434,640 bytes,
SHA-256 `7f6f67312884e2f0bc69a255d6e626543363a13d9d8974a073ff5f515b860e30`.
Machine entry stack stays 176,320 bytes; native region coverage is unchanged.
The runtime implementation and immutable cleanup cache change. No generated-code
size or peak-memory reduction is claimed.

A separate complete-call census proves the disappearing family directly:

| Input | Empty physical retirements before→after | Necessary retirements | Runtime callbacks |
|---|---:|---:|---:|
| 8B | 330→0 | 400 unchanged | 4,348 unchanged |
| Random 1KB | 12,482→0 | 21,693 unchanged | 73,441 unchanged |
| Repeated 1KB | 6,393→0 | 7,461 unchanged | 32,850 unchanged |

Each removed physical effect retains its original price. Complete-call logical
work stays 24,338,249,385 /959,562,617,769 /474,303,448,072, identically before and
after. These are resource-account units, not retired instructions. The generic
witness removes 24 reached effects across original/decoded/restored success,
rejection and retry, retaining exact empty/prior-publication receipts of
197,450,724 /197,467,056, aliases, prior output, cleanup and finite error prefixes.

Fresh isolated profiles cover 514 complete calls/version/input. Random has
1,289/1,161 exclusive sample weights before/after; repeated has 1,499/1,505.
Current random shares: generated 8.53%, other callbacks 76.92%, frame 5.51%, Tail
cleanup 7.75%, outside 1.29%. Repeated: generated 31.03%, other callbacks 56.35%,
frame 4.58%, Tail cleanup 5.91%, outside 2.13%. Thus runtime work remains about
90.18% /66.84% respectively. These are CPU samples, not function instruction
counts. Callbacks include algorithm work. The Tail-cleanup bucket excludes
cleanup in native-segment callbacks; do not assign its entire share to this patch.
Entry/exit bookkeeping, ordinary visits, current frame lookup and work accounting
remain prominent. The next substantial target is longer continuous native
regions and fewer handoffs around already supported work.

Passing: 530 compatible GPU tests with the same seven inherited profile/registry
exclusions; 105 inventoried and executed CI contracts; unchanged 236 components,
eight connected cases and one structural path case; 132 workflow tests;
15 architecture/catalog checks; runtime-only metadata; formatting and warnings
as errors. Core, math, Engine, Runtime and Stdlib remain frozen. No fresh full
feature/minimal/no_std graph or linked40/frame8/scalar12 performance suite is
claimed. Hosted CI is canceled under standing instruction; no green CI or clean
new-head review is claimed, and the PR stays draft.

The initial cleanup version is retained as adverse evidence: eight-byte/random
elapsed +0.563209/+0.599436%, repeated −0.047660%, with fewer instructions but
adverse cycles. Its binaries/Objects have verified lossless local backups and
are not uploaded. The first proposed scalar-guard cleanup was already handled
by existing retirement tables, so no production change was made for it. The
first witness incorrectly assumed equal work with/without prior publication;
clean-parent observations establish the two exact receipts preserved above.
No component output or allowance is changed and no unresolved correctness
failure remains.

Source/MBC pins remain 331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9 /
a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7.
Algorithm, tables, depth 300, nice 258, near-EOF limits, source allocation sites,
real tree/cache/path capacities, output clear/reuse capacities, input aliases,
allocator and timing boundaries stay fixed. Largest timed input remains 1KB.

All four bounded implementation stages in the current brief are now qualified:
cold Huffman paths, collector intervals/terminal forwarding, fixed-size mutable
children/current-account fuel, and representation-based cleanup. This does not
mean all collector paths are native or performance work is exhausted. Preserve
this checkpoint for review before the next compiler capability.


## R128: native child visits with exact current-account fuel

Compiler: **`2fbb199e2bc1c684f987ad6e4f0d468c2fb17dcb`**. Published parent:
`7fe6b6e611e678e34ec141efd4aeebd3726bf54f`; stack base
`eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`. IR/work/ABI/MBC:
**157/107/38/14**. Engine, Core, Syntax, Stdlib, Runtime, Compute and math
libraries use the retained R125 graph. The held R127 first-change envelope and
its Engine API are superseded; its adverse evidence remains retained.

Ordinary fixed-size mutable children now retain current phase-owned buffers.
Native code funds one reached child visit against the same current account,
before its initializer or recursive header. A static whole-child trip count is
optional. A cold fuel miss exports the exact live state, settles actual counters,
restores the source activation chain and continues at that initializer/header.
Already-paid ancestors retain their current traversal credit only. No source
replay, allowance renewal, per-iteration helper, or new dispatcher is introduced.
A first mutation may precede another child because later visits check the
post-mutation account. Appending child buffers remains outside this extension.

Version proofs compose loop-local current returns. Cold continuations' outer
buffer bindings are explicit child-entry preconditions checked by the caller;
a write invalidates every older binding to that root. Compound guards still
keep their conservative fallback. Images reconstruct funding homes, edge scope,
prices and versions, and reject altered recipes and older revisions.

The actual unchanged header loop passes all22 pinned cases with real19-entry U32
frequencies and320-entry U32 items, including untouched tails. Original/decoded
and restored-image controls exercise rejection after earlier writes, finite
initializer and recursive-header misses, aliases, unchanged prior publication,
zero final managed bytes/nodes and successful retry. The complete executable has
**63 numeric phases /137 included loops**, versus21/44 before;42 phases now use
checked child visits. It retains the pinned full level12 computation.

| Complete evaluate_into | R125 / Roc | R128 / Roc | Change |
|---|---:|---:|---:|
|8B|6.950100×|5.757588×|17.158200% faster|
|Random1KB|60.160589×|49.870151×|17.104949% faster|
|Repeated1KB|34.355126×|34.603400×|0.722671% slower|

Times are2.022479→1.675458ms /18.3489795→15.210396ms /
11.199771→11.2807085ms. References are0.291/0.305/0.326ms, fresh per case.
All3,968 primary/3,984 total8B observations remain, with4/4 improving pairs and
64/64 improving blocks. Each1KB version has128 primary observations: random
improves8/8 blocks; repeated improves2/8 and regresses6/8. The earlier8B cohort
and a clean-tree preflight stop during test-only editing remain separate;
production binaries did not change. No historical cohorts are pooled.

| Separate measured counter | R125 | R128 |
|---|---:|---:|
|8B instructions|22,608,589|17,716,541|
|Random instructions|283,628,006|234,201,012|
|Repeated instructions|172,685,983.5|170,813,427|
|8B cycles|6,415,757.5|5,302,077|
|Random cycles|58,679,246|48,182,471.5|
|Repeated cycles|35,836,913|35,554,370|
|8B requests, including reallocations|3,741|3,425|
|Random requests|22,197|16,463|
|Repeated requests|11,948|11,632|
|8B requested bytes|17,680,961|17,577,513|
|Random requested bytes|19,033,866|17,431,450|
|Repeated requested bytes|19,798,753|19,694,087|

Hardware has32 calls/version/case; heap has4, separately. Peak live usable heap
stays15,206,024 /15,204,488 /15,208,608 bytes. Reallocations stay68. Generated
text grows17,335,388→19,434,640B; entry stack158,144→176,320B. Retain those costs
and the adverse repeated-input elapsed result. No hardware/sample/heap cohorts
are multiplied together, and requested capacity is not copied or live bytes.

Executed callbacks, including admission, fall6,732→4,348 /93,457→73,441 /
35,228→32,850. Ordinary changed-update finalizations fall64→56 /4,243→4,115 /
1,085→1,085; private finalizations fall188→140 /1,295→134 /1,224→1,170.
Fresh isolated repeated-input profiles cover514 full calls/version and1,359/1,307
exclusive weights. Before/after shares: generated29.65/28.54%, other callbacks
57.54/56.69%, frame5.37/6.04%, cleanup5.81/6.66%, outside1.62/2.07%. These are CPU
samples, not per-function retired instructions. Remaining runtime work is real.

Passing current validation:527 compatible GPU tests (7 inherited profile/registry
exclusions),102 inventoried and executed CI contracts,236 components /8 connected
/1 structural path case,63 focused composition tests,132 workflow tests,15
architecture/catalog checks, runtime-only metadata, formatting and warnings as
errors. Output goldens and component allowances remain fixed. Shape observers
now recognize the fused parent instead of demanding callbacks or isolated child
regions. The old cleanup unit's protocol-work receipt changes88,531,834→41,515,530
because broader native selection omits ordinary boundary/materialization work;
policy107 price formulas and its output are unchanged. Current-version, prior
publication, error ordering, finite funding and retry controls all pass.

The temporary source-derived test wrapper had been deleted; it was regenerated
from the maintained integration harness, omitting the unused linked-project
module whose build library is absent from this frozen graph. Its component
bodies and allowances are untouched. The temporary Python environment also lost
its source files; restoring cached PyYAML6.0.3 repairs the workflow tests. Initial
metadata-observer failures and the intentionally stopped obsolete run are retained
with their disposition. No unresolved correctness failure remains.

Source SHA331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9
and MBC SHAa2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7
stay fixed, as do the262144-entry U16 tree,1500513-word U32 cache,305257 U64 path
capacity, depth300, nice258, near-EOF limits, tables, source allocation sites,
input aliases, output capacity13/1029 clear/reuse, allocator and timing boundary.
Largest timed input is1024B. No fresh full-feature/Core/no_std/minimal-graph build,
linked40/frame8/scalar12 performance suite, complete source-variant qualification,
full-scale timing, hosted green CI or clean review is claimed.

Retain this checkpoint. Brief stages1–3 now execute in the real program, with
native fuel addressing the demonstrated trip-proof restriction. Stage4—cleanup
for values that never materialize—remains the next bounded compiler change. The
repeated-input profile still spends69.40% in runtime callbacks/cleanup. Derive
remaining cleanup from actual representation while retaining original ordered
charges and owner cleanup; do not add another batching layer. Full parity and
complete collector lowering remain unestablished.


Raw observations, diagnostic drivers, full censuses and rejected experiments stay
outside source. The maintained harness and [compact evidence index](../../benchmarks/mech-deflate/experiments.json)
retain the reproduction path. [PR #973](https://github.com/mech-lang/mech/pull/973)
stays draft pending green CI and clean review. Hosted CI is not awaited under
instruction; no green hosted-CI claim is made.

## R127: held mutable-child prerequisite

Compiler candidate `6c1cb798e35da3e056d8043381d3488253d319f1`, test checkpoint
`71dda676dd`; IR/work/ABI/MBC 156/107/38/14. Generic fixed-size mutable child
calls now carry their current owners through one native phase. Each mutating
loop has an inductive carried/return proof and the caller invalidates older
bindings. A first-change admission envelope preserves funding for later native
child work on the same account. Original checked operation and actual debit
remain authoritative. Original/decoded/restored controls exercise a first write
inside a child and before a later child, rejection after writes, input aliases,
unchanged publication and retry. A compound Boolean guard retains conservative
fallback; that separate existing coverage restriction is not silently removed.

This is a held prerequisite, **not a complete compressor optimization**. The
complete pinned program still has exactly the same selected native regions,
17,335,388 generated bytes and 158,144-byte entry stack. Forty-two former mutation
refusals now reach the bounded-recurrence restriction. That is the next concrete
blocker for real header runs; no new complete-program coverage is claimed.

| Fresh pinned-source cohort | R125 / Roc | Candidate / Roc | Change |
|---|---:|---:|---:|
|8 bytes|7.085160×|7.066228×|0.267200% faster|
|1KB random|60.491955×|61.084158×|0.978978% slower|
|1KB repeated byte|34.851320×|35.212862×|1.037384% slower|

All 3,968 primary/3,984 total 8B observations remain (3/4 improving groups,
37 improving/27 adverse blocks); each 1KB version has 128 observations and
regresses in all eight blocks. Separate 32-observation instructions are
22,608,935→22,598,122.5 /283,642,867.5→283,464,874 /
172,674,166.5→172,626,070.5; cycles rise in every case. Separate four-observation
heap counts, requested bytes and incremental peaks are unchanged. Exact raw
bytes, independent inflate, input aliases, output reuse/capacity and zero final
managed bytes/nodes pass for all complete calls. No full-scale timing, broad
final qualification, hosted green CI or finished-brief claim is made.

## R126: source zero-trip guards rechecked on R125

Compiler `914ab57fab23f831f6bb99e5f8a0e8419d2c3004`, libraries and versions
155/107/38/14 remain unchanged. The existing R118 source variant skips empty
match and skip children. Pinned source/MBC remain the compiler baseline; the
variant stays a separately labeled option, with its contrary result retained.

| Fresh source-only complete-call cohort | Pinned / Roc | Variant / Roc | Change |
|---|---:|---:|---:|
|8 bytes|7.072300×|6.977789×|1.336347% faster|
|1KB random|60.156867×|54.267748×|9.789603% faster|
|1KB repeated byte|34.087942×|34.886238×|2.341870% slower|

Matched medians are 2.029750→2.0026255ms /18.2876875→16.4973955ms /
11.249021→11.5124585ms, against fresh common references 0.287/0.304/0.330ms.
All 3,968 primary/3,984 total eight-byte observations remain: four improving
groups, 54 improving/10 adverse blocks. Each 1KB version has 128 observations;
random improves in all eight blocks and repeated regresses in all eight.

Separate 32-observation hardware cohorts show instructions
22,609,326.5→22,378,913.5 /283,688,420→256,708,152 /
172,687,317→172,350,115. Repeated-data cycles rise
35,865,008→37,275,252 despite the slight instruction reduction. Separate
four-observation heap counts are 3,741→3,733 /22,197→21,173 /11,948→11,946;
incremental peak usable heap is unchanged for every input. Requested bytes are
17,680,961→17,676,985 /19,033,866→18,225,218 /19,798,753→19,799,529.
These independent cohorts are not factors to multiply.

Exact raw bytes, independent inflation, input aliases, real scratch capacities,
output capacity/clear/reuse and zero final managed bytes/nodes pass on every new
call. The old original/decoded source qualification is retained; no new complete
source-variant qualification, full-scale timing, hosted CI or compiler change is
claimed here. Canonical revision-14 roundtrip and source/MBC pins are checked
before native emission. Numerical reports reconstruct from raw retained rows.

## R125: collector intervals and paid native continuations

Update base **`0baa0519a2725ed1387fa6bb0ba0648da29a5184`**; measured compiler
**`914ab57fab23f831f6bb99e5f8a0e8419d2c3004`**; CI inventory update
`ea41f99050`. IR/work/ABI/MBC **155/107/38/14**. This is compiler/runtime work;
compressor source and revision-14 bytecode retain their exact pins.

The real skip collector now retains its native recurrence, stops at the actual
unsupported Tail/collection/range boundary, and forwards current buffer fields
at its terminal return. Cold exits freeze current versions, export only live
values and release a proved dead root carrier after those exports. Live older
aliases still require independent storage. The ordinary continuation can use the
existing native parent pieces: only the exact current root's already-paid source
visits receive credit. Current entry, per-piece overhead, synchronization and
reached cleanup remain charged. No child account, previous invocation or later
traversal can lend credit. Repeated immutable conditions also prevent an impossible
backedge from displacing the existing acyclic optimization. Images reconstruct
these facts independently; native SSA and boundary accesses use the same memory
region for each home.

| Fresh matched complete-call elapsed | R124 | R125 | R124 / Roc | R125 / Roc | Change |
|---|---:|---:|---:|---:|---:|
|8 bytes|2.0326245ms|2.020479ms|7.057724×|7.015552×|0.597528% faster|
|1KB random|18.794521ms|18.3514375ms|62.233513×|60.766349×|2.357514% faster|
|1KB repeated byte|12.9634165ms|11.2805625ms|39.643476×|34.497133×|12.981562% faster|

Each row has a fresh common reference: 0.288/0.302/0.327ms. All 3,968 primary /
3,984 total eight-byte observations are retained: four improving groups, 47
improving and 17 adverse blocks. Both 1KB cohorts retain 128 observations per
version; random improves in seven blocks with one adverse, repeated in all eight.
No full-scale timing was run. The largest input completed in this round is 1KB;
exact raw bytes, independent inflation, input aliases, capacities and zero final
managed bytes/nodes agree on all three inputs.

| Separate hardware / heap cohort | R124 | R125 |
|---|---:|---:|
|8B instructions|22,826,219|22,609,033|
|Random instructions|292,553,294|283,838,633.5|
|Repeated instructions|199,373,853|172,899,575|
|8B cycles|6,414,749.5|6,372,169|
|Random cycles|59,636,804|58,918,948|
|Repeated cycles|41,175,546|35,988,504.5|
|8B requests|3,733|3,741|
|Random requests|21,173|22,197|
|Repeated requests|17,021|11,948|
|8B requested bytes|17,674,585|17,680,961|
|Random requested bytes|18,222,818|19,033,866|
|Repeated requested bytes|19,754,529|19,798,753|

These are separate cohorts, not factors to multiply. There are 32 CPU and four
heap observations per version/input. Incremental peak usable heap remains
15,206,024 /15,204,488 /15,208,608 bytes; reallocations remain 68. The random case
adds 1,024 small allocation requests despite its elapsed/instruction gain. Generated
text grows 17,064,660→17,335,388 bytes; entry stack 154,112→158,144 bytes. Retain
these costs explicitly. Requested capacity is not initialized bytes copied or RSS.
Large-request histograms remain one cache and one tree; path reservations are two
on random input and three on the other cases, identically before/after.

Separate executed callback counts, including admission, fall 6,780→6,732 /
99,601→93,457 /52,585→35,228 on 8B/random/repeated. The repeated case takes 1,014
real cold exits. These counts are neither retired instructions nor a complete
allocation ledger. Fresh repeated-input CPU sampling assigns 26.40% to generated
entry code, 60.31% to other callbacks, 4.54% to the frame callback subtree, 6.44% to
tail cleanup and 2.31% outside/unrecognized. Some callback work is actual algorithm
work. Do not infer function instruction counts from sample weights.

The rejected intermediate is preserved: one version requested the full 6MB cache
1,015 times; the lifetime repair restores one but still regresses repeated-input
elapsed by 59.53%, with 111,098 callbacks. Restoring native continuations resolves
that regression. The shared SSA memory-region correction resolves an earlier
phase debit mismatch. Test-only observer mistakes and a diagnostic link failure
from disk exhaustion are also retained with their corrections; the identical link
succeeds after verified lossless build backups. No remaining introduced failure.

Passing qualification: 521 compatible GPU tests; 96 inventoried and executed CI
contracts; unchanged 236 components /8 connected /1 structural path case; all 13
focused interval, named-field and Huffman checks, including the unchanged
88,531,834 work golden; 12 optimized original/decoded scalar controls at the
unchanged 1.05 gate (worst 1.017598), no allocation/byte increases and zero loop
allocation slope; 132 workflow tests; 15 architecture/catalog checks; runtime-only
metadata; formatting; warnings as errors. Original/decoded/restored regressions
cover post-write cold selection, live/dead aliases, real tree/cache buffers,
finite denial, unchanged previous publication, cleanup, retry and forged images.
Seven inherited selected-profile feature/registry exclusions remain explicit.
No fresh Core/full-feature/no_std/minimal-graph rebuild, linked40/frame8,
complete source-variant qualification, hosted green CI or clean review is claimed.

All algorithm, table, depth 300, nice 258, near-EOF, allocation-site, real-capacity,
allocator and evaluate_into boundaries remain fixed. Retain this measured
checkpoint; fixed-size mutable children/header loops and never-materialized-value
cleanup remain next. This does not complete the whole brief or establish parity.
All 39 public diagnostic archives (34,888,296 compressed bytes) and sidecars have
verified server hashes. Fresh public retrieval verifies 3,613 file occurrences /
846 unique paths, preserves 11 multi-version paths, and rebuilds four reports
byte-for-byte without compiling or compressing. Native executables, libraries,
Objects and bytecode remain local; completed intermediate executables have
verified lossless local backups after disk cleanup. Raw evidence and numerical
reconstruction stay outside source, hash-indexed in
[the evidence ledger](../../benchmarks/mech-deflate/experiments.json).

## R124: native parents with exact cold scalar-child exits

Update base `0a7237f66c407f0ec880b3ad5768a8433c7b6ec0`; measured compiler
**`fc8e98e8d7230b78ff01644ae1457828f281673b`**; qualified diagnostic correction
**`d73f332c467db89b5ba03659062b7917d8185c6a`**; formatting-only follow-up
`2169f283b754c1fc77bf08dd1d8aec2f940485f1`. IR/work/ABI/MBC **151/104/38/14**.
The correction preserves the original proof refusal when both selection attempts
decline. Its full pinned Object and descriptor are byte-identical to the measured
compiler, as are source and bytecode; measured libraries and workers stay immutable.

The first part of the new brief is complete: Huffman counting retains its buffers
and scalar state natively until a conditional unsupported scalar child is actually
needed. The exact exit exports live current versions after earlier stores and
runs the original child and remaining source iteration ordinarily. The parent can
re-enter at its next original header. One prepaid parent traversal and actual
child/library work share the same authoritative account. There is no prefix replay,
allowance renewal or duplicate source debit. The resume flag occupies independent
region state, outside scratch shared by successive phases. Image restoration
reconstructs exits, live versions and debit facts independently.

| Fresh matched elapsed cohort | Before | R124 | Before / Roc | R124 / Roc | Change |
|---|---:|---:|---:|---:|---:|
|8 bytes|2.183042ms|2.0305625ms|7.686768×|7.149868×|6.984726% faster|
|1KB random|21.4515625ms|18.819708ms|69.761179×|61.202302×|12.268824% faster|
|1KB repeated byte|12.831042ms|12.987021ms|39.000128×|39.474228×|1.215638% slower|

Each row has a fresh common reference: 0.284ms, 0.3075ms and 0.329ms respectively.
All 3,968 primary/3,984 total eight-byte observations and 128 per version per 1KB
cohort are retained. All 4 eight-byte pairs improve, with 61 improving/3 adverse
blocks; random improves in 8/8 blocks, repeated data regresses in 8/8. The contrary
repeated-data timing remains part of the retention decision. No full-scale timing.

| Separate serial hardware/heap cohort | Before | R124 |
|---|---:|---:|
|8B instructions|24,975,234.5|22,827,125|
|Random instructions|335,823,511|292,561,285.5|
|Repeated instructions|199,272,682.5|199,405,344.5|
|8B cycles|6,840,859|6,429,108|
|Random cycles|68,107,211|59,407,498.5|
|Repeated cycles|41,025,796.5|40,906,187.5|
|8B requests|4,101|3,733|
|Random requests|29,396|21,173|
|Repeated requests|17,109|17,021|
|8B requested bytes|17,744,809|17,674,585|
|Random requested bytes|20,001,570|18,222,818|
|Repeated requested bytes|19,767,433|19,754,529|

Incremental peak usable heap is unchanged: 15,206,024 /15,204,488 /15,208,608 bytes
for 8B/random/repeated. Reallocations remain 68. The repeated-data instruction increase
of 0.066573% is retained alongside its elapsed regression; lower cycles in the
separate cohort do not override either observation. Generated text grows
16,533,748→17,064,660 bytes and entry stack 148,240→154,112 bytes. Requested bytes,
physical footprint, live heap, source work and retired instructions are distinct.

Fresh separate diagnostic calls count callbacks including ordinary admission:
8B 7,892→6,780; random 119,933→99,601; repeated 52,945→52,585. Ordinary changed indexed
replacement finalizations fall 180→64 /6,499→4,243 /4,167→4,131. These count the actual
changed branch whose selected Engine operation constructs one canonical result,
not all stores multiplied by a guessed allocation cost. Private-array finalizations
rise 172→188 /1,281→1,295 /197→209; first-change preparations have those same counts.
These are separate observed construction families, not a complete allocation ledger.
Parent exits fall 829→573 /22,655→18,129 /8,780→8,692; they are a subset of helpers.
Array alias exports fall 765→669 /21,057→19,539 /5,772→5,732. The 13 newly eligible
scalar-cold parent instances have zero taken cold exits in these complete small
calls; positive post-write fallback is established by the focused regression tests.
Fresh serial CPU-time samples cover 130 complete calls per version/input. Other
host callbacks retain 79.88% of random-data sample weights and 63.51% on repeated
data; generated entry shares are 4.94% and 23.56%. These are CPU samples rather
than function-level retired-instruction counts. Collector/child boundaries remain
a concrete next coverage target.

All source/bytecode pins, source algorithm, tables, depth 300, nice 258, near-EOF
rules, allocation sites, real capacities, pinned allocator, exact raw output,
independent inflation, aliases, output clear/rebind and evaluate_into boundary stay
unchanged. Large-request histograms still show one 6,002,052-byte cache, one 524,288-byte
tree and three 2,442,056-byte path requests. Final managed bytes/nodes remain zero.

Passing qualification: 3 new cold-child tests across original/decoded/restored
forms, 16 existing nested-phase controls, 3 argument-bank controls, 512 compatible GPU
tests, 87 inventoried native contracts, unchanged 236 components/8 connected/1 structural
path case, 12 optimized original/decoded scalar controls at the unchanged 1.05 gate
(worst 1.008892), 132 workflow tests, 15 architecture/catalog checks, runtime-only
metadata, formatting and warnings as errors. Huffman work golden remains 88,531,834.
No expected value or allowance increase. The initially failing older-array-version
component diagnostic is fixed separately; the initial rustfmt line-wrap failure
and its correction are retained in evidence. Seven inherited selected-profile
feature/registry exclusions remain explicit. No fresh Core/full-feature/no_std/
minimal-graph build, linked 40/frame 8, source-variant complete qualification, green
hosted CI or clean review is claimed.

All 23 public diagnostic archives (20,619,880 compressed bytes) and their sidecars
have verified server SHA-256 hashes. Native binaries, Objects, libraries and bytecode
stay local. Fresh public retrieval verifies 1,858 file occurrences/483 unique paths, preserves
12 multi-version paths and reproduces all four reports byte-for-byte. The compact
index retains every timing event and numerical replay entry.

Retain the tiny/random elapsed and instruction gains with the repeated-input and
code/stack tradeoffs. Only scalar-return synchronous Tail children are covered.
The remaining brief is: collector intervals with genuine cold collection/checkpoint
exits and terminal buffer-owner forwarding; fixed-size mutable children/header runs;
then cleanup removal for values proven never materialized. No collector coverage,
managed child result or whole-brief completion is claimed by R124.

## R123: retain whole matching phases on the repaired runtime

Update base `2d9a0b3d8092836a21cab938f0d1356646e2a27b`; measured compiler
**`71f8cb7bed8bc84bfcb75915b79b8c299401adc5`**; IR/work/ABI/MBC **150/103/38/14**.
The preserved positive-stride/checked-offset proof now runs on the retained
first-change handoff and separated argument spans. Three matching roots 27/35/41
each select six loops under one phase. Actual child boundary callbacks are zero;
observed interior callbacks are only the required first-change bridge. The
independently restored executable funds 239 argument slots but exposes only 36
canonical slots. Source, revision 14 bytecode, algorithm, tables, depth 300, nice 258,
near-EOF rules, capacities, source allocation sites and timing boundary stay fixed.

| Fresh matched elapsed cohort | R122 / Roc | R123 / Roc | Change |
|---|---:|---:|---:|
|8 bytes|7.516093×|7.534698×|0.247538% slower|
|1KB random|71.713017×|71.530060×|0.255123% faster|
|1KB repeated byte|56.639914×|39.694189×|29.918345% faster|

Each row has its own fresh common reference. Preserve all 3,968 primary /3,984
total 8B observations; two of four prepared pairs improve,27 blocks improve and 37
regress. Random improves in 7/8 alternating blocks, repeated data in 8/8, with 128
observations per version in each 1KB cohort. No full-scale timing. Raw output,
independent inflation, aliases, capacities and final managed bytes/nodes zero agree.

| Separate hardware/heap cohort | R122 | R123 |
|---|---:|---:|
|8B instructions|24,841,845|24,976,680|
|Random instructions|335,420,451|335,805,569.5|
|Repeated instructions|299,264,848.5|199,273,142|
|8B allocation requests|4,101|4,101|
|Random allocation requests|29,391|29,396|
|Repeated allocation requests|16,091|17,109|
|8B requested bytes|17,728,849|17,744,809|
|Random requested bytes|19,981,650|20,001,570|
|Repeated requested bytes|19,212,913|19,767,433|
|Repeated incremental peak heap|15,178,400|15,208,608|

All three incremental peaks increase 30,208 bytes. Cycles are 6,890,881.5→6,877,023,
68,082,700.5→68,130,332 and 59,025,345.5→42,103,394 for 8B/random/repeated. Rich
allocation requests rise by 1,018; this metadata cost remains explicit. The fresh
histograms contain one 6,002,052-byte cache and one 524,288-byte tree request, with
three 2,442,056-byte path requests on both sides. The held R120 experiment's four
extra full-cache requests are not reintroduced. Requested bytes are not memcpy
volume or live memory. Native text grows 14,780,140→16,533,748 bytes and entry
stack 144,528→148,240 bytes. The tiny/random adverse instruction counts and larger
code/stack are retained; no all-input speedup or allocation reduction is claimed.

Fresh separate diagnostics count all registered callbacks, including ordinary
admission:8B7,892→7,892; random 120,088→119,933; repeated 112,103→52,945. Within
those totals, repeated helper callbacks fall 59,738→32,660 and parent exits
17,806→8,780. Exits are a subset of helpers, not another additive category.
Repeated internal array exports and source-site details remain in the replayed
mechanism report. After CPU-time profiles attribute 80.56% /64.69% to other host
callbacks and 4.95% /23.70% to the generated body for random/repeated. Sample
percentages are not retired-instruction partitions.

The pinned reference's largest anonymous fast function was freshly disassembled:
16,684 static instruction lines and 826 static call sites. Its original source
name is unavailable, so no particular source routine or dynamic count is inferred.
The full local binary hash and bounded assembly are retained outside source.

Passing checks:16 nested-phase controls,3 argument-bank controls,84 selected
native tests,509 compatible GPU tests, unchanged 236 components /8 connected /
1 structural path case,12 scalar original/decoded controls at the unchanged 1.05
gate (worst 1.017157),132 workflow tests,15 architecture/catalog checks,
runtime-only metadata, formatting and warnings as errors. No expected output or
allowance increase; Huffman work golden remains 88,531,834. Seven independently
reproduced selected-profile feature/registry failures remain excluded. No fresh
Core/full-feature/no_std/minimal-graph build, linked 40/frame 8, source-variant
complete qualification, green hosted CI or clean review is claimed.

The first negative driver assumed a different assertion message; its original
failed selection and outer expectation failure are preserved. A clearer selection
expectation then reproduces the absent whole phase on unchanged production code.
After the repair the same controls pass.36 completed old test executables were
hash-inventoried and removed, freeing 4,065,733,664 logical bytes; frozen libraries,
references, measured workers, Objects, descriptors and current baseline stay.

All 19 public archives (13,103,351 compressed bytes) and sidecars have verified
server hashes. Fresh public retrieval verifies 1,645 file occurrences / 438 unique
paths and reproduces all four reports byte-for-byte. Native outputs remain local.

Retain the substantial repeated-data gain with the measured small-input and
metadata tradeoffs above. Follow the new brief next: native Huffman counting with
an explicit cold exit immediately before its clamp child, receiving current
post-write state and exact debit/continuation. Then collector intervals with
terminal owner forwarding, fixed-size mutable children/header runs, and removal
of cleanup obligations only for values that no longer materialize. The new brief's
historical numbers do not replace this fresh checkpoint. Full collector native
coverage, arbitrary cold exits and mutating child results remain unimplemented.

## R122: separate canonical owner spans from numeric transport words

Update base `7eaebbd7e6bd8deb572af5aafccddb0b0d0e22ad`; measured compiler
**`da58ad43e12a7c9e078631514f1c06d874d4c384`**; IR/work/ABI/MBC **149/102/38/14**.
The same-build independently restored descriptor confirms 61 funded argument
slots but only 36 canonical owner slots. Numeric word requirements no longer
expand the managed prefix. The original physical backing, initialization,
resource admission and all logical prices remain. The prefix is independently
derived from source operations and parent/window recipes, never serialized as
authority. Original/decoded/restored controls establish 75/9 on the small witness.

| Fresh matched elapsed cohort | R121 / Roc | R122 / Roc | Reduction |
|---|---:|---:|---:|
|8 bytes|7.688302×|7.616406×|0.935146%|
|1KB random|72.580879×|71.841963×|1.018059%|
|1KB repeated byte|57.627250×|56.801056×|1.433687%|

Each row uses a fresh common reference. Four of four prepared 8B pairs improve,
with 58 improving / 6 adverse blocks. Both 1 KB cohorts improve in 8/8 blocks.
All 3,968 primary / 3,984 total 8B observations and 128 per-version observations in
each 1 KB cohort are retained. Exact raw output, independent inflation, input
aliases, output capacities, source/bytecode, allocator and timing boundary agree.
No full-scale timing.

Separate hardware cohorts show instructions 24,887,726→24,844,684 at 8B,
337,065,606.5→335,451,272.5 on random data and300,357,022.5→299,273,793.5
on repeated data. Cycles are 9,462,679→6,853,749.5,68,514,460.5→68,477,069 and
58,848,421→58,728,198.5 respectively. The disproportionately large 8B cycle
change is preserved as observed; it is not a matched elapsed or causal speedup
claim. Hardware/heap cohorts are separate from primary elapsed and CPU samples.

Requests stay 4,101 / 29,391 / 16,091; requested bytes stay 17,728,849 / 19,981,650 /
19,212,913, and incremental peak heap stays 15,175,816 / 15,174,280 / 15,178,400
bytes for8B/random/repeated. Generated text 14,780,140 and stack 144,528 bytes,
numeric selections and cleanup reports are unchanged. Actual host disassembly
confirms the same successful empty-slot loop over the checked shorter slice;
static instruction counts are not retired instruction counts. No fresh dynamic
callback census was collected. Fresh after CPU-time profiles attribute 81.01%
(random) and 68.31% (repeated) to other callbacks under generated entry; generated
body shares are 4.95% / 18.78%. Those sample weights are not an instruction ledger.

Passing checks: 3 new argument-bank tests, 80 inventoried native tests, 505 compatible
GPU tests, unchanged 236 component / 8 connected / 1 structural path controls,
132 workflow tests,15 architecture/catalog checks, runtime-only metadata,
formatting and warnings as errors. Twelve optimized original/decoded scalar
controls pass the unchanged 1.05 gate (worst 1.004283) with no request/byte increase.
The unchanged Huffman work golden is 88,531,834. Seven previously demonstrated
selected-profile feature/registry failures remain excluded. No fresh Core,
all-feature/no_std/minimal-dependency build, linked 40/frame 8, source-variant
complete qualification, hosted green CI or clean review is claimed.

The parent reproducer fails 75 versus 9 before the repair and passes afterward.
Full backing remains admitted under tight limits; invalid geometry is rejected;
the final canonical owner remains checked; clearing preserves live aliases and
releases managed memory. Full compressor work pins are unchanged. macOS denied
an optional noninteractive debugger launch; that diagnostic produced no
compression result. An ordinary same-build public Debug view independently
confirmed the 61/36 geometry without a compiler edit.

All 17 public archives (8,054,420 compressed bytes) and sidecars have verified
server hashes. Fresh retrieval verifies 1,351 file occurrences / 399 unique paths
and regenerates four reports byte-for-byte. Native outputs remain local.

Retain this measured improvement. Next: rebuild the held positive-stride
matching phases on R121's owner handoff and this separated bank, then repeat the
same small cohorts. Its earlier repeated-data gain was accompanied by cache
copies and a numeric-word-induced scan increase; neither old regression is
assumed solved until that new complete executable is measured.

## R121: canonical owner handoff at a nested phase's first change

Update base `445cf91bcd8ea50955942c5a1895206a620a92ad`; measured compiler
**`6ed4e0fb7e7e34abd1162f090e112045bc052565`**; IR/work/ABI/MBC **148/102/38/14**.
The cold bridge now applies the existing same-plan ordinary alias-retirement
proof before Core decides whether a changed array can be consumed. It retires
only currently matching dead canonical pins. Live earlier results and outside
aliases remain owners. Existing child-free handoffs, prices, golden values and
component allowances remain unchanged.

| Fresh matched elapsed cohort | R119 / Roc | R121 / Roc | Change |
|---|---:|---:|---:|
|8B|7.571121×|7.547341×|0.314079% faster|
|1KB random|71.710410×|71.500138×|0.293224% faster|
|1KB repeated `a`|57.128589×|57.431283×|0.529847% slower|

This is an enabling repair with mixed standalone elapsed results. Only two of
four8B prepared pairs improve;41 descriptive blocks improve and23 regress.
Random data improves in6/8 blocks; repeated data regresses in8/8. Keep all3,968
primary/3,984 total8B observations and128 per-version observations in each1KB
cohort. References are fresh within each row; do not compare absolute ratios
across rounds or compound independently collected improvements.

Separate retired-instruction medians are24,971,843.5→24,888,483 for8B,
338,023,420→336,812,584.5 for random data and300,963,185→300,396,174 for repeated
data. Allocation requests remain4,101/29,391/16,091 respectively, and incremental
peak usable heap remains15,175,816/15,174,280/15,178,400B. Generated text remains
14,780,140B; entry stack144,528B; numeric emitter reports and source logical work
match R119. No allocation reduction in the retained benchmark is claimed.

A separate causal witness combines the unchanged held R120 Object with this
final cold handoff. Across two complete repeated-data calls, its eight first
cache changes reuse their backing: the four extra1,500,513-word U32 cache copies
per call disappear. This is a mechanism witness, not a portable-image or elapsed
comparison. The larger phase still needs a complete rebuilt performance cohort.
The prior live-result control still copies; denying that original copy preserves
published output, frees unpublished state and permits a successful retry.

Passing checks:12 nested-phase controls,77 selected native tests,502 compatible
GPU units,236 unchanged components,8 connected tests,1 path-structure test,
132 workflow tests,15 architecture/catalog checks and runtime-only metadata.
All12 optimized original/decoded scalar controls pass the unchanged1.05 gate
(worst1.003103), with no request/byte increase. Seven previously reproduced
missing-feature/registry failures remain excluded. No fresh full-feature/Core/
no_std/minimal-graph, linked40/frame8, source-variant complete run, hosted CI
green, clean review or full-scale timing is claimed.

Fresh profiles retain130 complete calls per subject/input. Other host callbacks
account for81.89% of random-data sample weights and68.33% of repeated-data
weights after the repair. These are CPU-time samples, not instruction counts.
The next concrete target is raw numeric argument capacity inflating the managed
value bank: the held larger phase grows it61→239, causing147,562 additional
empty-slot checks across829 tiny-input parent exits. Separating those prepared
bounds is proposed; its performance benefit has not yet been measured.

## R120: strided matching phases held outside the retained compiler

Candidate compiler `740e98506ea6abc3ac8d889999cdcfcc628684e8` extends the existing
phase proof to positive constant strides behind checked affine bounds. The
original source and bytecode stay fixed. This selects the search/skip matching
children as complete native phases, but is **not retained** because its results
are mixed. The retained compiler keeps R119 selection with R121's cold handoff.

| Fresh matched elapsed cohort | R119 / Roc | Candidate / Roc | Change |
|---|---:|---:|---:|
|8B|7.725280×|7.944908×|2.842985% slower|
|1KB random|71.650316×|78.479788×|9.531670% slower|
|1KB repeated `a`|57.456581×|45.452363×|20.892677% faster|

These are new paired cohorts with fresh common references, separate from the
R119 table below. Instructions change24,974,182.5→25,695,890.5 for8B,
337,873,526.5→354,285,515 for random data and301,093,187.5→210,928,847.5 for
repeated data. Repeated-data requests increase16,091→17,113 and requested bytes
19,212,913→43,775,641; incremental peak rises15,178,400→18,566,688B. Generated
text grows14,780,140→16,533,748B and entry stack144,528→148,240B.

The exact owner diagnostic finds four first changes per repeated-data call
copying the1,500,513-word cache while dead canonical slot1117 remains. The
existing same-plan ordinary ownership proof already includes that slot. The
numeric first-change bridge does not apply that retirement inventory. This
demonstrated handoff gap is the next bounded repair; an arena-wide scan remains
diagnostic only. Requested capacity and initialized copy volume are separate
metrics. The tiny-input callback counts remain identical; the instruction
regression's cause has not been established. A larger generated body is a
hypothesis, and the host-assembly relocation comparison is inconclusive.

Candidate checks pass13 nested-phase tests,78 selected native tests,236 unchanged
components,8 connected tests,3 retained source-variant components,1 complete
source-variant test and1 path-structure test, with warnings as errors. No fresh
scalar12, broad GPU, architecture/catalog/runtime-only, Core/full-feature,
linked40/frame8 or full-scale gate is claimed for this held candidate. All
adverse timings and corrected diagnostic-harness errors are preserved in the
compact index and public diagnostic archives. The candidate remains available
as an experiment; it does not replace R119's accepted improvement.

## R119: complete nested path phases, original source fixed

Base **`e7acd5c17bc1f73f424f1a375563d8c77796f06a`**; representation extraction
`48f42c2371`; production compiler **`99fef709f1d984e32ad22e5c8929e8dfd0ff0984`**.
The compiler comparison uses retained R116 `509c0d8e05a234020372b43832d7785f255c1b59`
and the identical original pinned source/bytecode on both sides. R118's source
variant is excluded from these timings. IR/work/ABI/MBC are **146/100/38/14**.

| Complete level-12 elapsed cohort | Before / Roc | R119 / Roc | Reduction |
|---|---:|---:|---:|
|8B, four prepared pairs|8.584585×|8.243080×|3.978123%|
|1KB random, no matches|105.755961×|72.897517×|31.070064%|
|1KB repeated `a`, many matches|175.226011×|58.371386×|66.687944%|

Fresh common reference medians are respectively0.283/0.297/0.317ms. Mech medians
are2.4294375→2.3327915ms,31.4095205→21.6505625ms and55.5466455→18.5037295ms.
All4 prepared pairs and64 descriptive8B blocks improve; all8 alternating blocks
improve in each1KB cohort. Preserve3,968 primary/3,984 total8B observations and
128 observations per version in each separate1KB cohort. Do not pool or compound
these results. Largest timed input remains1KB; parity remains1×.

The path root and its two nested match/length state machines now execute under
one checked numeric plan, current views, private nodes owner and native fuel.
Six views retain the five invariant input tables and current growing nodes;
scalar/enum arguments and results remain native. Selected path iterations136
and170 each include all three loops,398 source sites,94 scalar operations,
10 typed loads and one append. Actual generated success backedges contain zero
helpers. Two identical complete-call diagnostic traces per size confirm the
replaced path segments disappear:1KB internal path-segment alias exports **18,450→0**, including
18,432 per-position exports and18 terminal exports. Across the complete call,
ordinary admissions fall100,264→48,025, helpers96,648→72,063 and segment alias
exports39,527→21,077. These are callback/export counts, not allocation counts.

Separate hardware/heap cohorts show:

| Metric per complete call |8B before→after|1KB random before→after|1KB repeated `a` before→after|
|---|---:|---:|---:|
|Retired instructions|26,164,238→24,973,897.5|494,401,360→337,779,025|947,224,009→300,909,708.5|
|CPU cycles|7,218,221.5→6,874,517.5|100,771,717.5→68,846,886|179,882,224.5→58,976,932|
|Allocation requests|4,257→4,101|47,796→29,391|40,631→16,091|
|Requested bytes|95,876,945→17,728,849|7,523,247,338→19,981,650|10,023,567,217→19,212,913|
|Incremental peak live usable bytes|17,782,424→15,175,816|17,773,224→15,174,280|17,784,368→15,178,400|

Hardware uses32 observations per subject, heap8 per Mech subject; reference
hardware/heap each32 where collected. Reference1KB random instructions are
1,690,857 and heap requests84. Repeated-data reference hardware/heap was not
collected. All68 reallocations per Mech call remain. Requested capacity is not
initialized/copied bytes. Whole-harness1KB random RSS is319,258,624→318,963,712B,
physical footprint228,968,192→228,607,744B and between-call allocator live bytes
144,335,128→144,345,464B. This harness embeds compiler/JIT seed/clear support and
the full prepared plan; these are not minimum runtime-only AOT memory figures.
No zero-copy input claim is made.

Generated text grows14,584,260→14,780,140B (**+195,880B**); entry stack
134,960→144,528B (**+9,568B**). Whole1KB harness text grows63,660,224→63,966,888B;
the reference harness text is195,132B. The whole-binary figures include host
support and must stay separate from generated compressor text. Native binaries
and Objects remain local.

Fresh serial profiles cover130 complete calls per Mech subject at each1KB input.
Random-data exclusive context weights: generated3.83→5.28%, other host callbacks
82.06→78.05%, cleanup9.13→11.54%, private-frame callbacks3.53→3.92%. Match-rich
generated share rises9.63→16.83%, other callbacks fall74.15→69.39%. Remaining
hot symbols include numeric-segment entry/settlement, account debits, owner
retirement and thread-local access. Those callbacks also include useful algorithm
work. A separate long reference harness profile places1,505/1,521 weights
(98.95%) in generated reference functions; opaque symbols are not assigned
source names. CPU-time sample weights are not retired-instruction partitions.

Original/decoded/restored witnesses now require complete three-loop selection,
zero hot child handoffs and constant boundary counts as positions increase.
The growing-array witness reads appended nodes and their new length on the next
child visit, preserves the older external alias, fails after prior mutation,
keeps published output unchanged and retries successfully. Controls retain fresh
Boolean arguments, assertion/bounds/division precedence and origins, checked
overflow, metadata tamper rejection, finite funding after a nonempty native
prefix, capacity fallback without duplicate append and admission-bound overflow
falling back before source checks.

Passing inventories: nested phase9, selected native74, profile-compatible GPU
units499, unchanged component236, connected8, structural path1, explicit native
path/entry controls4, retained source-variant components3/complete1, workflow132,
scalar original/decoded12 (unchanged1.05× gate, worst1.032241×, no request/byte
increase), architecture/catalog15 and runtime-only metadata with warnings as
errors. Existing finite path allowances/goldens stay unchanged; separate
NoLimits controls require the real path phase rather than accepting fallback.
CI inventory repairs `fd466b3aef`/`c005ad976a` are separately reviewable.

The initial broad selected-dependency GPU run passed498 and failed8. All8 also
fail on clean parent `e7acd5c` with the same frozen dependencies. One inherited
stale ABI34 assertion is repaired separately in `1532e432ff` and its test passes.
Seven inherited configuration failures remain explicitly excluded from the499:
five batched tests lack the GPU f32 feature, one lacks matrix/matmul and one lacks
stats/sum in the selected registry. Their exact commands and parent failures
are archived. This is not a claim that a full-feature CI run is green.

Exact raw bytes, independent inflate, input aliases, capacities13/1,029,
clear/rebind outside `evaluate_into` and final managed bytes/nodes zero hold
throughout. Original algorithm/tables/search limits/depth300/nice258/near-EOF
limits,262,144U16 tree,1,500,513U32 cache,305,257U64 path reservation and pinned
allocator remain unchanged. Source SHA256 remains
`331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`;
revision14 MBC SHA256 remains
`a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`.

The initial conservative outer-traversal funding guard is397,217,883,291,064;
actual traversals, not that maximum, determine charges. Existing1e12 finite
component budgets lawfully fall back. A tighter compositional length bound,
nested growth/exhaustion, child managed results and worker/scoped funding remain
future work. Preparation also repeatedly reconstructs definition facts; that is
a known compile/restore cost outside these `evaluate_into` timings. Keep these
limitations explicit before selecting the next measured compiler change.

No fresh Core/no_std/all-features/minimal dependency rebuild, linked40/frame8
rerun, hosted CI completion, review approval or full-scale timing is claimed.
The compact index supplies immutable journals, source revisions, exact commands,
counter/profile scopes and deterministic offline report reconstruction. All20
evidence archives (8,666,137 compressed bytes) and sidecars have verified server
digests. Fresh public retrieval verifies1,678 file occurrences/499 unique paths
and reproduces all four indexed reports byte-for-byte. No native binary, Object,
library or bytecode is uploaded.

## R118: zero-trip source variant; R116 compiler unchanged

Compiler control **`509c0d8e05a234020372b43832d7785f255c1b59`**; published base
`6a303c6d92d14f5a88ab4aecb9999fda205f285a`; application variant
**`50751f9445356b9b33d16b89f636876e6014379f`**; test-filter correction
`2203da6c04`. This is a separately labeled source optimization. The maintained
pinned source and its MBC remain unchanged; compiler-only parity is still a
separate measurement. IR/work/ABI/MBC remain145/99/38/14.

The variant restores general zero-match and zero-skip guards. It bypasses only
empty children and returns their incoming state. Nonzero calls, earlier checks,
search/skip separation, algorithm, tables, real scratch capacities, depth300,
nice258, near-EOF limits, allocator and `evaluate_into` boundary are unchanged.

| Matched elapsed cohort | Before / Roc | Variant / Roc | Change |
|---|---:|---:|---:|
|8B, four prepared pairs|7.836520×|7.653202×|−2.339278%|
|1KB random, no matches|102.481686×|91.356686×|−10.855598%|
|1KB repeated `a`, many matches|171.389060×|165.643853×|−3.352143%|

Each row uses its own fresh common reference. All4 prepared pairs improve;
8B descriptive blocks include62 improving and2 adverse. Both1KB cohorts have
8/8 improving alternating blocks and128 observations per version. Preserve all
3,968 primary /3,984 total8B observations. These results are neither pooled nor
compounded; the target remains1×. Largest timed input is1KB.

Separate hardware:8B instructions26,163,688→25,679,547 and cycles
7,216,289.5→7,088,817;1KB no-match instructions494,413,943→439,689,257
(**−11.068597%**) and cycles100,478,452.5→89,185,590.5 (**−11.239088%**).
No match-rich hardware or heap cohort was collected. Serial CPU-time profiles
cover695 complete8B calls and130 complete1KB no-match calls per subject;
CPU-time weights are not a retired-instruction ledger.

Heap requests remain4,257 at8B and47,796 at1KB, with68 reallocations at each.
Requested bytes rise by**2,368** per call:95,876,945→95,879,313 at8B and
7,523,247,338→7,523,249,706 at1KB. Incremental peak live heap is unchanged:
17,782,424B /17,773,224B. Whole prepared-harness between-call live usable bytes
rise**66,544B** at both sizes; this includes prepared metadata/current values,
not an exclusive cache census. Requested capacity is not copied bytes.
Native text shrinks14,584,260→14,571,128B and entry stack134,960→134,768B.
Whole linked text also shrinks13,132B. Native binaries remain local.

Two identical execution censuses per size/subject confirm that all**3,072** empty
path-match child calls disappear at1KB. Registered admission/helper callbacks
fall196,912→164,144; numeric entries/exits31,912→29,864; array-alias exports
39,527→38,503. At8B the corresponding callback count is8,724→8,412.
GPU scalar-transport materializations remain3; this observer does not count all
Core canonical constructors. The two per-position path frontiers still export
**18,432 arrays**. Their owners are path iterations136 and170, retaining6,144
and12,288 exports respectively. This is the unchanged compiler witness for a
complete phase with persistent views/owner/scalar state.

Passing, with inventories checked separately: unchanged baseline component236,
composition8, path1; variant component3; complete variant1 test covering all
existing small cases from original and decoded artifacts. Expected values and
allowances are unchanged. Checks retain exact nodes, real305,257-node capacity,
entry errors/order, live aliases, failure publication and recovery. Complete
compression and timed calls match pinned raw bytes and independent inflate,
with final managed bytes/nodes zero. Timed output capacities remain13 /1,029.

The initial standalone producer missed a retained `sha2` dependency; correcting
the extern fixed this harness error. The first managed reference build returned
`PermissionDenied` without identifying an operation; the identical approved
command succeeded. The inherited scale-summary scope string initially claimed
identical source/MBC; original and corrected metadata are both retained, with
raw observations/statistics unchanged. These are resolved harness/environment
issues, not introduced compiler failures.

No new whole-compiler/Core/no_std/all-features/minimal-graph, scalar12/frame8 or
linked40 mini-program rerun is claimed: compiler and libraries are unchanged
from R116. No16KB/full-scale timing or green hosted-CI claim. The next compiler
work keeps source fixed on both sides and targets the complete nested path phase.
No no-match whole-block simplification or packed-load gain is claimed yet.

All19 evidence archives (7,956,453 compressed bytes) and sidecars are published
with matching server digests. Fresh retrieval verifies999 file occurrences /
305 unique paths and reproduces all four indexed reports byte-for-byte.

Variant source SHA256:`85fba4bc9f55796c8a83c7ff0533590d4bf5e97bd8be42d4dd56c5d448427b36`;
revision14 MBC SHA256:`5d9318ae144d5d42b3bee53dd23ff6e26955ef6652c5f93ef05eeadf69a9ca26`.
The evidence index retains the exact commands, feature profile, raw journals,
reference/source pins, corrected scope, code inspections and replay reports.

## R117: ordered retirement scan held; restore R116

Baseline `6a303c6d92d14f5a88ab4aecb9999fda205f285a`; candidate
`8e7334a4fdd929c58f0d741195c3d8a1857d81af`; restoration **`a057d5f570fc3ad9b460bf66876af182fa442257`**.
The three runtime/test files now match the retained R116 checkpoint exactly.
The experiment remains on `codex/retirement-scan-r117-experiment`.

| Matched elapsed | Before / reference | Candidate / reference | Change |
|---|---:|---:|---:|
|8B, four prepared pairs|8.028728×|8.105774×|+0.959624%|
|1KB, alternating blocks|104.010278×|105.039737×|+0.989766%|

No pair/block improves at its respective level: 0/4 prepared pairs at 8B,
0/8 blocks at 1KB. The 8B descriptive blocks include 13 improving and 51 adverse.
All 3,968 primary / 3,984 total 8B observations and 128 per-version 1KB
observations remain. Separate 1KB instructions rise 494,394,047.5→501,094,481;
cycles rise 101,451,651→103,381,783.5. Separate 8B instructions also rise
26,164,453→26,401,464.5. Per-call heap requests/bytes/peak are unchanged at both
sizes. The generated Object/image/metadata remain byte-identical, including
14,584,260B native text and 134,960B entry stack; whole linked runtime text grows
12,368B. No source algorithm, pricing, allocator or buffer change.

The scan replaces repeated arena-account checks with one check while the arena
and work meter are immutably borrowed. Two diagnostic executions agree: at 1KB
account checks fall 233,456→184,929 while all 233,456 slot visits remain. Native
entries stay 31,912. At 8B checks fall 7,963→6,804 with the same 7,963 slots and
933 entries. Actual ARM64 contains an 824-byte flattened-iterator scan, compared
with the old 220-byte single-slot helper. Removed checks did not lower whole-call
instructions or elapsed; this candidate is held.

Disjoint entry-stage clock windows and an approximately 24ns empty-clock
calibration are retained as diagnostics, with their substantial observer cost
explicit. They are neither primary timings nor instruction attribution. Fresh
serial profiles cover 695 complete 8B calls and 130 complete 1KB calls per subject.
The exact reference binary is inspected again; opaque symbols are not assigned
source names.

Passing: maintained runtime356, work/memory/values33, guarded native65, image1,
and both extended generation tests. Those tests check empty scans, foreign
ledgers, ordered refusal versus invalid slots, untouched work/aliases/generations
and recovery. The complete linked compressor matches exact bytes and independent
inflation at both sizes; source work stays 42,390,445,317 / 1,970,438,688,821.
No fresh component236/connected8/path1, linked40 mini-program, scalar12/frame8,
Core/no_std/all-features/minimal graph, hosted CI or full-scale run is claimed for
this held candidate. No introduced failing test remains in executed scope.

The next assignment changes strategy: preserve R116 as the compiler control,
measure zero-match/zero-skip source bypasses separately, and target a complete
path scan with persistent typed views, scalar state and one nodes owner. The
no-match block certificate and whole-pass simplification are a separate source
track; vector shape, range proofs and packed loads follow those measurements.

All17 evidence archives (7,193,521 compressed bytes) and sidecars are published
with matching server digests. Fresh retrieval verifies1,128 file occurrences /
304 unique paths and reproduces all four indexed reports byte-for-byte.

## R116: fund native scalar aggregate words through the ordinary path

Base **`d5e88ff2e66714ad4ee1d1f200518958401cea9a`**; retained compiler/runtime
**`509c0d8e05a234020372b43832d7785f255c1b59`**; focused regression
`b91ab7e6a3`. Independently derived private scalar aggregate actions already
emit checked native words. They now share the existing ordinary scalar funding
path, without a new owner frame, argument bank or cache. Owned-map selection,
lifecycle boundaries, lexical failures and the shared aggregate emitter remain
unchanged. Image loading rederives the same groups after validating the scalar
word layout. IR145 rejects IR143 and the held IR144; work policy99, private
ABI38 and bytecode14 remain.

| Matched elapsed cohort | Baseline / reference | Candidate / reference | Change |
|---|---:|---:|---:|
|8B, four prepared pairs|7.979811×|7.874930×|−1.314335%|
|1KB, alternating blocks|113.499239×|104.174555×|−8.215636%|

8B takes 2.322125→2.2916045 ms against a fresh 0.291 ms reference. All four
prepared pairs improve; 61/64 descriptive blocks improve and three worsen.
The cohort retains 3,968 primary / 3,984 total observations. The separate 1KB
cohort takes 34.163271→31.356541 ms against a fresh 0.301 ms reference, with
128 observations per version; all eight blocks improve. Each row has its own
common reference. Historical ratios and independent cohorts are not pooled or
compounded. The target remains 1×; no larger-input or full-scale speed is claimed.

| Separate hardware cohort | Baseline | Candidate |
|---|---:|---:|
|8B retired instructions|26,773,029|26,168,226|
|8B cycles|7,394,977.5|7,211,731.5|
|1KB retired instructions|552,038,310|494,396,024|
|1KB cycles|109,262,186.5|100,770,407.5|

At 1KB, instructions fall **10.441718%** and cycles **7.771928%**, independently
of the primary elapsed cohort. Hardware has 32 observations per subject; heap
has eight Mech observations after controls are excluded. Instrumented elapsed
is not a primary timing result. The fresh 1KB reference uses 1,690,857 instructions
and 974,536.5 cycles. Samples are CPU-time weights, not function instruction counts.

Per-call heap demand is exactly unchanged: 8B 4,257 requests / 95,876,945 requested
bytes / 17,782,424B incremental live peak; 1KB 47,796 requests / 7,523,247,338 requested
bytes / 17,773,224B peak. Reallocations 68, included in requests. Requested capacity
is neither initialized bytes copied nor live memory. Between-call allocator live
bytes rise **1,224B** at both sizes in the whole prepared harness; that metric
includes current values and is not an exclusive prepared-metadata census. This
small increase remains explicit rather than claiming a universal heap reduction.

Generated native text shrinks **20,436B**, 14,604,696→14,584,260B; machine entry
stack shrinks **768B**, 135,728→134,960B. Whole linked instruction text falls
21,844B to 63,660,796B; executable bytes fall 15,744 to 172,981,960. These are
actual binary/metadata measurements, not a complete host-stack census. Fresh
bounded ARM64 inspection covers helper, entry and exit functions. The pinned
reference is inspected again: its complete 195,456B text includes platform and
allocator, while the Mech generated-text row excludes its host runtime. Opaque
reference symbols are not assigned source names.

### Executed mechanism

| Registered ABI callbacks per complete call | 8B before → after | 1KB before → after |
|---|---:|---:|
|Ordinary admission|4,890 →3,823|201,648 →100,264|
|Helpers, including fused callbacks|4,911 →4,901|96,655 →96,648|
|Total|9,801 →8,724|298,303 →196,912|
|Numeric entries/exits|938 →933|31,916 →31,912|

Two diagnostic executions agree at each size and compiler. The observer counts
the registered admission callback and helper separately; a fused helper counts
once. Internal admission routines are not falsely counted as another ABI call.
These are callback counts, not payload copying or retired instructions. Array
alias exports remain 981 at 8B and 39,527 at 1KB. Ordinary scalar groups rise
1,372→1,373, with 8,642→8,939 covered static sites and 35 groups containing native
aggregate actions. Emitted numeric pieces decrease 484→479. This removes repeated
funding work instead of adding the extra owner-entry boundaries seen in R115.

The exact blocking source remains `#OptimalPathMatches :Try(m,k,best,item)` at
pinned source lines 700–714, including four-word tuple and enum packing. All
algorithm code, tables, search limits, near-EOF bounds, real capacities, source
allocation sites, allocator and evaluate_into timing boundaries are unchanged.
Mech performs the complete compression. Exact raw bytes, independent inflation,
input aliases, output capacity 13/1029, clear/rebind outside the timer and final
managed bytes/nodes 0 pass at both sizes.

### Correctness and validation

The existing scalar-funding regression is extended with a private tuple and
changing true/false arguments, retaining its original source variant and expected
behavior. It fails on the clean parent at missing aggregate group coverage.
Original/decoded and independently restored artifacts then pass native interior
arithmetic origins, one-short funding fallback, assertion-before-overflow order,
unchanged prior publication, live aliases and success→failure→retry. Test inventory
counts and allowances do not increase. No R115 numeric-aggregate SSA selection is
enabled. Successful numeric exit clears both the scalar marker and its shared
readiness lane so an unfunded ordinary suffix retains its original admissions.

Passing selections: runtime 356, work/memory 33, guarded native 65, image 1;
unchanged components 236 / connected 8 / path 1 with actual listed and executed counts;
linked 16+12+12; scalar 12 and frame 8. Scalar's maximum elapsed ratio is 1.005790
under the unchanged 1.05 gate; pure scalar requests/bytes are equal and loop
allocation slope is zero. Frame requests/bytes/work do not increase. Workflow 132,
architecture/type/memory 15, runtime-only cfg and full formatting pass, with
warnings-as-errors. No golden or allowance changes. No fresh Core/no_std,
all-features or minimal dependency graph qualification is claimed; the existing
expensive knucleotide-body exclusion remains explicit. Hosted CI is not awaited
and the PR stays draft pending green CI and clean review.

Logical resource work is not hardware instructions. Existing policy99 prices
the changed funding/partition geometry: 42,410,605,541→42,390,445,317 at 8B,
1,970,398,096,074→1,970,438,688,821 at 1KB. Both actual work values are pinned
separately in every timing and hardware cohort. No source operation is removed
to lower that counter. Fresh serial profiles cover 695 complete calls per 8B
subject and 130 per 1KB subject. Remaining sampling cost is concentrated in numeric
entry/exit, budget checks and cleanup; callback sample shares are not a causal
instruction ledger. The next measurement should split those remaining entry
checks before another implementation change.

No newly introduced compiler/runtime or diagnostic failure remains in executed
scope. The baseline negative regression is intentional. Obsolete failed-test
executables from R115 are deleted after their source/commands/receipts were
archived, freeing 337,841,784B; active baseline and candidate binaries remain pinned.

All 19 R116 archives (12,815,394 compressed bytes), plus sidecars, have verified
server digests. Fresh independent retrieval verifies 1,445 file occurrences /
482 unique paths, including all preserved versions, and reproduces all four
indexed reports byte-for-byte. Native binaries stay local. The maintained
`experiment.py replay-round --round 116` entry verifies and reconstructs the
numbers without compiling or compressing.

## R115: scalar aggregate parent pieces held

Base `e7644df6bb40969bfbe3b7e38856071fc4ef2090`; initial candidate
`0bb239f3849df2bea16dc4929581d1ba83f61ca8`, corrected compiler
**`2f0cc8b85e7dbcef53abfe16d76742872a7ceca2`**, test-only correction
`c1b0265887`. The candidate uses the existing independently derived scalar
aggregate word layout and shared checked SSA emitter inside acyclic parent
pieces. IR144 rejects old IR143 identities; work policy 99, ABI38 and bytecode14
remain. A fresh complete-compressor Object is emitted, not reused.

| Cohort | Baseline / reference | Candidate / reference | Elapsed change |
|---|---:|---:|---:|
|8B, four prepared pairs|8.380415×|8.363944×|−0.196543%|
|1KB, original|112.018587×|113.075641×|+0.943642%|
|1KB, independent confirmation|113.620849×|114.912444×|+1.136759%|

The 8B cohort retains 3,968 primary / 3,984 total observations; two of four
pairs improve, with 47 improving and 17 adverse descriptive blocks. Each 1KB
cohort has 128 observations per version; neither improves any of its eight
blocks. The independent confirmation was declared after the contrary hardware
cohort, before its own observations. All original observations remain. Each row
uses its own common reference; historical multiples are not pooled or compounded.

Actual 8B elapsed is 2.321375→2.3168125 ms, reference 0.277 ms. The 1KB times
are 34.2776875→34.601146 ms, reference 0.306 ms; confirmation
34.1998755→34.5886455 ms, reference 0.301 ms. No full-scale run is made.

The separate 1KB hardware cohort is favorable: instructions
552,040,295.5→545,942,708.5 (**−1.104555%**), cycles
109,309,426→108,070,921.5 (**−1.133026%**), 32 observations per subject.
These counts remain explicit contrary evidence to both paired elapsed cohorts.
Their instrumented elapsed is not a primary speed claim. The fresh reference
uses 1,690,807 instructions / 968,725 cycles in that cohort.

Heap demand is exactly unchanged: 47,796 requests, 7,523,247,338 requested bytes,
68 reallocations and 17,773,224B incremental live peak, eight measured calls per
Mech subject. The separate 1KB reference has 84 requests / 9,012,988 requested
bytes / 9,257,328B incremental peak. Requested capacity is neither initialized
bytes copied nor live memory. No new 8B hardware/heap cohort is claimed.

Generated native text grows **87,040B**, 14,604,696→14,691,736B; native entry
stack grows **688B**, 135,728→136,416B. Whole linked text grows 86,868B to
63,769,508B; the executable grows 98,064B. Fresh bounded ARM64 disassembly
covers actual helper/entry/exit functions. The pinned reference is inspected
again: its complete text is 195,456B, including platform and allocator; Mech's
generated-text row excludes its host runtime. Static sizes are not executed
instruction counts, and opaque reference symbols are not assigned source names.

### What the census changed

| Exact counts per 1KB computation | Baseline | Candidate |
|---|---:|---:|
|Native numeric entries/exits|31,916|34,988|
|Runtime helper callbacks, excluding ordinary admissions|96,655|102,796|
|Exported array aliases|39,527|39,527|
|Exported managed frame fields|52,432|52,438|

Two repeated diagnostic executions agree at each size and compiler revision.
The extra 3,072 entries are dominated by two six-site pieces reached 1,024 and
2,048 times, each with two aggregate actions, no numeric views, no managed
sources and no frames. The selection formula counts source sites plus actions,
although aggregate actions already emit native word instructions without an
ordinary helper. The candidate adds helper boundaries without reducing array
alias exports. These are measured mechanisms, not a causal attribution of every
elapsed or cycle difference.

The real source witness is `#OptimalPathMatches :Try(m,k,best,item)` at pinned
source lines 700–714, including four-word tuple and enum packing; one original
frontier reaches that pack 2,048 times at 1KB. The next bounded target is to fund
already-native aggregate words through the existing cheap scalar-admission
path and base selection on actual callback work. No additional ownership cache
is proposed. The positive instruction result is preserved for that refinement.

### Correctness, failures and disposition

The new minimal regression fails on the retained parent because aggregate
actions are absent from its native piece. The first implementation then exposes
an introduced cursor failure: piece 25 executes 91→7, but an unchanged readiness
word incorrectly skips ordinary finish 7 and attempts 91→92. Its exit now clears
the readiness lane together with `scalar_path`; the test requires that exact
partial ordinary-group suffix. The first error-class expectation incorrectly
looked for `Bounds`; it now requires the maintained `IndexOutOfRange` and adds
a simultaneous failed-assertion / zero-divisor control. Earlier lexical load
failure wins. Both failed receipts remain.

Corrected candidate execution passes runtime 357, work/memory 33, guarded native
inventory/execution 66 and image 1, including explicit IR143 rejection. The new
original/decoded and independently restored controls check current Boolean
words across recursive visits, success→assertion/arithmetic/bounds failure→retry,
unselected child calls, live aliases, unchanged prior publication and cleanup.
The complete linked compressor matches exact raw bytes and independent inflation
at 8B and1KB; capacity 13/1029, clear/rebind outside evaluation and final managed
bytes/nodes 0 remain. Source, algorithm, tables, limits, real buffers, source
allocation sites, allocator and timing boundaries are unchanged.

Logical resource prices are not hardware instructions. The changed piece
partition charges 42,410,605,541→42,381,503,754 at 8B and
1,970,398,096,074→1,970,444,323,983 at 1KB under the existing policy 99 formula.
The restored default retains the baseline charges. No component golden or
allowance is changed. Candidate workflow 132, architecture/type/memory 15,
runtime-only cfg and formatting pass. Fresh component 236 / connected 8 / path 1,
linked 40 mini-program, scalar 12 / frame 8 performance, Core/no_std/all-features
and minimal-graph qualifications are skipped for this held trial; no full
retained-candidate or hosted-CI qualification is claimed. Fresh serial profiles
cover 130 complete 1KB calls per subject, outside primary timing.

Other preserved diagnostic failures: a reader requested sites beyond a 94-block
fixture; its already-created descriptor was reread at valid sites without new
execution. An initial hardware driver could not inspect host activity; it failed
before any observation and succeeds with authorized access. A stale initial
diagnostic-only scope sentence is corrected by an explicit addendum; actual
primary specs, artifact identities, version fields and numerical cohorts were
correct. No hardware resource denial or unresolved retained compiler failure
remains in executed scope.

**Held, not default.** Experiment branch
`codex/scalar-aggregate-pieces-r115-experiment` preserves checkpoint
`7ddc7acd1b8cf812f38bf9da682e69d7f5f206f9`. Restoration
**`94c7447f7fb5c30073340388b8cf865a0d1a888d`** returns all nine compiler,
runtime, test and CI files exactly to base R114. Default versions remain
**143/99/38/14**; its prior qualifications remain applicable. This update adds
evidence and status relative to that base.

All 15 R115 archives (19,477,535 compressed bytes), plus sidecars, have verified
server digests. Fresh independent retrieval verifies 1,087 file occurrences /
324 unique paths, including all eight paths with multiple preserved versions,
and reproduces all three indexed reports byte-for-byte. Native binaries remain
local. The maintained `experiment.py replay-round --round 115` entry verifies
and reconstructs these numbers without compiling or compressing.

## R114: inline private dispatch at the callback boundary

Base `a105e9a76978ff7bbb4a8dd56968a3c7b8e457b8`; retained runtime change
**`f327ef9489aaa0997f4d8627256f1a14eb70b0c4`**. The complete production change is
one `#[inline(always)]` attribute on `Invocation::helper`. The source program,
algorithm, tables, search limits, real buffer capacities, allocation sites,
allocator, logical prices, work and evaluate_into boundaries remain pinned.
Versions remain IR/work/private ABI/bytecode **143/99/38/14**.

| Cohort | Same-round baseline / reference | Candidate / reference | Elapsed change |
|---|---:|---:|---:|
|8B original|8.003090×|7.966452×|−0.457797%|
|8B independent confirmation|8.104747×|8.047038×|−0.712040%|
|1KB original|111.327477×|110.976443×|−0.315317%|
|1KB independent confirmation|112.166735×|111.884938×|−0.251231%|

The two 8B cohorts each retain 3,968 primary/3,984 total observations across four
fresh prepared pairs. Original pairs improve 3/4, confirmation 4/4; descriptive
blocks improve 41/64 and 51/64. The separate 1KB cohorts each retain 128 observations
per version in alternating 16-call blocks; 7/8 and 6/8 blocks improve. Every adverse
observation remains. Each row uses its own common fresh reference; these are not
pooled or compounded gains. The target remains 1× and larger-input throughput is
still unestablished. No full-scale timing is run.

8B elapsed 2.320896→2.310271 ms (reference 0.290 ms), confirmation
2.3260625→2.3095 ms (reference 0.287 ms). The 1KB times are 34.066208→33.9587915 ms
(reference 0.306 ms), confirmation 34.0986875→34.013021 ms (reference 0.304 ms).
The small effect motivated the separately declared confirmation before retention.

| Separate counter cohort | Baseline | Candidate | Change |
|---|---:|---:|---:|
|8B retired instructions|26,860,744|26,774,972|−0.319321%|
|8B cycles|7,333,426|7,353,423.5|+0.272690%|
|1KB retired instructions|552,670,712|551,997,052.5|−0.121892%|
|1KB cycles|109,466,354.5|110,060,389.5|+0.542664%|

**Both cycle cohorts worsen.** Preserve this contrary result alongside the four
matched elapsed improvements. Counter, heap, sample and primary timing cohorts
are distinct; instrumented elapsed is not a speed claim. Hardware has 32
observations per subject; heap has 32 reference / eight Mech observations after
controls are excluded. No per-function instruction ledger is inferred from samples.

Heap demand is exactly unchanged: 8B 4,257 requests/95,876,945 requested bytes/
17,782,424B incremental live peak; 1KB 47,796 requests/7,523,247,338 requested bytes/
17,773,224B incremental peak. Reallocations 68, included in requests. Requested
capacity is not copying volume or live memory. No new cache or retained data bank
is introduced; this is not a measured whole-prepared-memory reduction. Retention
is a small speed/instruction win with a 4 KiB code-size cost, not a memory win.

### Mechanism and code footprint

The native error type and `Result<Option<u64>, Error>` each occupy 80B;
`Option<u64>` occupies 16B. Actual baseline ARM64 supplies a stack return place to
the separate generic dispatcher. Inlining removes that internal call from the
hot callback while retaining the original body and check order. This does not
prove 80B copied on each successful return. The callback frame remains 320B;
its symbol span grows 884→3,044B and the standalone 19,276B dispatcher symbol
is absent. Two additional internal String/map fallback callers also exist; the
change is not based on a claim that there is only one production caller.

Whole linked instruction text grows **4,096B**, 63,678,544→63,682,640B. Total
executable bytes fall 172,998,888→172,997,704; other sections differ. Report both
rather than claiming a code-memory reduction. Native program text remains
14,604,696B and entry stack 135,728B. No fresh complete host-stack census is claimed.

The exact retained R108 Object/descriptor/metadata are reused, after verifying
that the only GPU source delta is this runtime attribute. Code generation,
protocol, types, image semantics and all dependencies remain unchanged. Object
producer source is `705c5acdc4fc92be5beeacf28d02721ea4e39ddf`; runtime source is
`f327ef9489aaa0997f4d8627256f1a14eb70b0c4`. All three pins and actual native text
are byte-identical. No fresh full-compressor Object emission or matched compiler-
time improvement is claimed. The actual GPU-only release build takes 23.231s;
all project/external dependencies remain frozen from R105.

### Qualification and remaining cost

Passing current selections: runtime 356, work/memory 33, guarded native CI 65,
image 1, unchanged components 236 / connected 8 / path 1 with listed inventories,
linked 16+12+12, scalar 12 and frame 8. Scalar's maximum ratio is 1.015528 under the
unchanged 1.05 gate; pure scalar requests/bytes are equal and loop allocation
slope is zero. Frame requests/bytes/work do not increase. Workflow 132,
architecture/type/memory 15, runtime-only cfg and full formatting pass. No golden
or allowance changes. Original/decoded/restored and linked controls retain current
arguments, live aliases, checked failures, unchanged publication, cleanup and retry.
Exact raw bytes and independent inflation pass at both sizes; output capacity 13/
1029 with clear/rebind outside the timer and final managed bytes/nodes 0 remain.
The existing expensive knucleotide-body exclusion stays explicit. No fresh Core,
no_std, all-features or minimal dependency graph qualification is claimed.

Fresh serial profiles cover 695 complete calls per 8B subject and 130 per 1KB subject.
CPU weights are not retired instructions; ancestry categories overlap. The next
bounded diagnostic is an actual census of which managed operations end native
pieces and force array publication, to select a larger boundary reduction.

The 16-case linked run passed before its receipt hit a protected Git-metadata write.
The exact archive/index entry was verified and its pending commit completed with
authorized access; tests were not rerun. Two numerical-reader path assumptions
were corrected without new measurements: the section receipt is at
`binary-audit/sections.json`, and an already-symbolized raw sample may have no
separate resolved file. Original sealed packets and failures remain, with a new
corrected-reader packet. No retained compiler/runtime failure remains in executed
scope. The PR stays draft pending green CI and clean review; hosted CI is not awaited.

All 24 R114 archives (12,027,300 compressed bytes), plus their sidecars, have
verified server digests in the continuation evidence release. Independent fresh
retrieval verifies 1,747 file occurrences / 491 unique paths and reproduces all
four indexed numerical reports byte-for-byte. No native binaries are published.

## R112–R113: adjacent numeric admission trials held

R112 source `c85dfe8bcea3bc39a626a5287f2fa1ee5a22ad5c` combines source admission
and numeric entry through a private ABI39 tag. R113
`62b901e9b0cb81b2c84359bc41a48a50f56ecfa5` reuses the already computed scalar
fallback decision. Both preserve source work and exact output, and both are held.

| Trial | 8B baseline → candidate / reference | Elapsed change | 1KB baseline → candidate / reference | Elapsed change |
|---|---:|---:|---:|---:|
|R112|8.111014× → 8.115967×|+0.061063%|112.203467× → 116.062363×|+3.439195%|
|R113|8.038864× → 8.128734×|+1.117953%|106.951137× → 107.672749×|+0.674712%|

Each 8B trial retains 3,968 primary/3,984 total observations; each 1KB trial retains
128 observations/version. R113 has no improving 8B pair or 1KB block. Different
reference times explain different baseline multiples across rows; baseline 1KB
elapsed is 34.109854 ms and 34.063937 ms respectively. Do not treat those historical
multiples as a compiler improvement.

R112's separate 1KB instructions increase 552,670,237→553,848,641 (+0.213220%);
cycles rise 109,696,621.5→109,868,435 (+0.156626%). This small counter-cycle delta
contrasts with the primary 3.44% elapsed regression and remains explicit. Heap
requests 47,796, requested bytes 7,523,247,338 and live peak 17,773,224B are unchanged.
R113 has no new hardware/heap cohort. Both have fresh serial 1KB CPU profiles and
actual bounded ARM64 disassembly; samples are not instruction attribution.

Actual program text falls 16,300B to 14,588,396, but native entry stack grows 272B
to 136,000. The callback frame grows 320→400B in R112 and 464B in R113. R112 adds
an outlined 184B scalar-decision lookup; R113 removes the redundant lookup but
still regresses. These are verified code differences, not a complete causal
explanation of elapsed changes. R112/R113 Objects are byte-identical to each other,
with changed ABI identity versus R108. Old ABI rejection remains strict.

Preserved diagnostic failures: the first R112 unit driver started before its
library manifest existed, before Rust compilation; after the dependency completed,
the driver succeeds. Its one-short-fuel observer originally injected only through
the old callback; shared injection at both real admission forms fixes the observer
in `f410781775` without relaxing assertions. Nine focused cases pass. R113's full
runtime run passes 354 cases and fails two stale ABI38 assertions against ABI39.
Both pass on retained ABI38; test-only update `d4a7f89b9b` makes both pass in focused
reruns. No new all-356-pass cohort is claimed for the held ABI39 candidate.

Restoration **`951eb640a6`** returns all 12 changed GPU files exactly to the retained
ABI38 implementation. Source commits, tests and raw adverse results remain on
`codex/numeric-admission-r112-experiment` and
`codex/numeric-admission-r113-experiment` and in the published history. Full
component/linked/scalar/frame qualification is not claimed for either held trial.
These findings do not reopen the previously completed assertion, fixture or CI
work. R114 retains ABI38 and only the separate runtime inline attribute.

R112 publishes 11 archives (10,356,141 compressed bytes), R113 10 (10,279,867),
plus sidecars, with verified server digests. Fresh independent retrieval verifies
705 / 643 file occurrences and reproduces both reports for each trial exactly.
The maintained `experiment.py replay-round` entry performs verification and
numerical replay without compiling or compressing. R112–R114 together retain
45 immutable archives and eight byte-identical derived reports.

## R109–R111: frame geometry trials held; default remains R108

The speed target remains 1×. The earlier R106 memory-saving ownership change
remains held rather than becoming the default. Its 1KB gain and smaller-input
regressions are preserved below; cumulative requested capacity is not live memory.

Three subsequent GPU-runtime trials reuse immutable frame-field geometry during
numeric handoffs. Every comparison uses the retained R108 executable, source
revision `705c5acdc4fc92be5beeacf28d02721ea4e39ddf`, with a fresh reference for
that comparison. The candidate source revisions are:

- R109 `0cbf5b9f6b5870c40aac5a75bc7c89bb23c9b32a`: lazy layout reuse plus field writers.
- R110 `7a47b86b5e05e696beac21b7fe673c8b171c4fd2`: explicit lazy conditionals remove an outlined Option helper.
- R111 `727a955105fe43ee99d120560157bbdf985dacd2`: resolve the immutable layout before the field loop and borrow field descriptions.

| Trial | 8B baseline → candidate / reference | Elapsed change | 1KB baseline → candidate / reference | Elapsed change |
|---|---:|---:|---:|---:|
|R109|7.973493× → 8.037212×|+0.799135%|110.613871× → 111.660288×|+0.946009%|
|R110|7.944563× → 7.969462×|+0.313409%|112.791323× → 113.392659×|+0.533140%|
|R111|7.948558× → 7.967464×|+0.237852%|112.394942× → 112.059211×|−0.298707%|

Each 8B trial retains 3,968 primary/3,984 total observations over four prepared
pairs. The separate 1KB cohorts retain 128 observations/version in alternating
16-call blocks. R111 improves two of four 8B pairs and seven of eight 1KB blocks;
its positive result is retained alongside the adverse one. These trials do not
establish a consistent speed improvement across both measured sizes. Neither
historical reference ratios nor results from independent cohorts are combined.

Address-bounded inspection of the actual ARM64 executables finds a new per-field
outlined Option helper in R109 and 32-byte field-description transport at the
new writers. R110 removes the Option helper. R111 borrows the field description
and removes that aggregate copy at the scalar writer. These are verified binary
changes, not a causal attribution of the elapsed differences. No instruction,
heap, prepared-memory or fresh CPU-sample cohort is claimed for these held trials.
The earlier weighted layout-lookup prediction remains explicitly a prediction;
it is not an executed-call census for the final eager-lookup variant.

All trial Objects/descriptors/metadata remain byte-identical to R108. Generated
compressor text remains 14,604,696B and machine entry stack 135,728B; this does not
claim unchanged host helper stack. Versions 143/99/38/14, source/bytecode pins,
algorithm, real capacities, allocation sites, allocator, source work and timing
boundaries remain unchanged. Both input sizes preserve exact raw bytes,
independent inflation, aliases, output capacity13/1029 with clear/rebind outside
the timer, and final managed bytes/nodes0. No larger-input timing is run.

The new R111 focused regression passes for original/decoded artifacts, covering
invalid Bool bits, wrong lane/schema, unchanged state/accounting and old aliases,
error precedence with absent backing, and recovery. Its initial expectation
incorrectly assumed every fixture field contained 7; the existing helper uses
7 plus the leaf ordinal. Test-only fix `11b61ba7f7` captures each alias's actual
pre-update draft. The failed receipt is retained. The existing collection-lifetime
regression also passes. Full runtime/component/linked/scalar/frame qualification
was not rerun for these unretained trials; no inherited allowance or golden changed.

Candidate source/tests/CI inventory are saved on local branch
`codex/frame-geometry-r109-r111-experiment` at `af69cc894c`. Restoration commit
**`8bd35198ca69e0dac5c78e967c758f3397c7e3b7`** returns all compiler, runtime, test
and CI files exactly to published R108 head
`3b310eb80f84ab16757923f270931fd53cf1477b`. The default diff since that head
contains only evidence/index/status updates. Source commits remain in history,
and diagnostic patches/receipts preserve the experiments for later comparison.
No new optimization is retained from R109–R111; R108's validation scope below still
applies to the unchanged implementation.

All31 archives and31 sidecars (29,015,328 archive bytes, no native binaries) are
published with verified server digests. Fresh maintained retrieval reproduces all
six timing/validation reports byte-identically from raw observations without
compiling or compressing. It verifies the original and corrected fixture receipts
as well as every earlier packet version. The restored default passes132 workflow
checks,15 architecture/type/memory checks and full formatting. Exact commands,
replay counts, omissions and report hashes are in the compact index. Hosted CI
is not awaited; the PR remains draft pending green CI and clean review.

## R108: reuse current owner references within one numeric entry

Base `ac80ae5e7766c0aef16978f5ba39b90aafd198c2`; compiler/runtime change
**`705c5acdc4fc92be5beeacf28d02721ea4e39ddf`**. Only GPU runtime code changes;
all project and external dependencies stay frozen from R105. GPU release build
22.597s; Object emission59.816s. These are descriptive setup durations, not matched
compiler-time improvement claims. The R106 ownership candidate remains separate.

A numeric entry first resolves and validates every current source. It now keeps
eight borrowed owner references for the rest of that same entry. Higher source
indices keep the existing lookup path; this is no program-size/eligibility limit.
There are no owner clones, heap allocations, persistent caches or new permissions.
Current schema, footprint, typed-view, account, funding, failure and publication
checks retain their order. Argument-word writes touch the separate admitted bank;
shared owner borrows cannot mutate owner tables, and no bank reference escapes.
No source, algorithm, table, capacity, allocator, ABI or pricing change occurs.

External diagnostics find all938/31,916 entries still selected on8B/1KB. Source
lookups fall5,540→2,877 and211,691→112,793 respectively:2,663/98,898 avoided.
Only38 repeats remain in each witness, from the ordinary larger-index fallback.
Both diagnostic repetitions agree. Logical work stays42,410,605,541 /1,970,398,096,074;
diagnostic timings and allocations are excluded from performance claims.

| Input | Fresh baseline / reference | R108 / reference | Elapsed change |
|---|---:|---:|---:|
|8B|8.098347×|7.993464×|−1.295113%|
|1KB|113.933211×|111.848379×|−1.829872%|

8B elapsed2.3485205→2.3181045ms, common fresh reference0.290ms;3,968 primary /
3,984 total observations, all four pairs improve,58 descriptive blocks improve
and6 worsen. The separate1KB cohort is34.8635625→34.225604ms, reference0.306ms,
128 observations/version; all eight alternating16-call blocks improve. All adverse
observations remain. Each row uses its own common reference; historical ratios
are not combined. The1KB input is the retained deterministic xorshift prefix;
all capacities, exact raw output, independent inflate, input aliases, clear/rebind
outside evaluate_into and final managed bytes/nodes0 pass. No full-scale run.

| Separate counter cohort | Baseline | R108 | Change |
|---|---:|---:|---:|
|8B instructions|27,013,922.5|26,861,966.5|−0.562510%|
|8B cycles|7,417,961|7,328,817.5|−1.201725%|
|1KB instructions|558,180,681|552,668,745.5|−0.987482%|
|1KB cycles|111,651,054.5|109,037,226|−2.341069%|

Heap demand is exactly unchanged:8B4,257 requests/95,876,945 requested bytes/
17,782,424B incremental peak; 1KB47,796 requests/7,523,247,338 requested bytes/
17,773,224B incremental peak. Reallocations68 in both cases, included in requests.
These are separate hardware/heap cohorts, not primary timings or copying totals.

Object/descriptor/metadata stay byte-identical. Generated text14,604,696B and
native entry stack135,728B remain; whole linked text shrinks128B. Actual bounded
ARM64 helper inspection finds a480→560B host frame (**+80B**) and4,060→3,868B
symbol span. Eight borrowed references are local temporary storage, not a heap
or prepared-owner bank. No fresh full prepared-heap census is claimed.
Versions143/99/38/14 and every component golden/allowance remain unchanged.

Passing selections: runtime356; work/memory33; guarded native CI65; image1;
unchanged components236/connected8/path1; linked16+12+12; scalar12 (maximum
1.003199 under unchanged1.05 gate, pure counts/bytes equal, loop allocation slope0);
frame8 (no request/byte/work increase); workflow132; architecture/type/memory15;
runtime-only feature cfg and full formatting. Original/decoded/restored images,
fresh recursive/current arguments, live aliases, finite funding, rejection,
unchanged prior publication and retry are covered by these maintained controls.
No fresh Core/no_std/all-features/minimal dependency graph is claimed; the existing
expensive knucleotide-body exclusion remains. Hosted CI is not awaited.

Fresh serial profiles cover695 calls/subject at8B and130 at1KB. Current sample
weights1,269/1,319 are not retired instructions. At1KB, generated entry has54/1,319
exclusive weights; other callbacks1,096, tail cleanup109 and private-frame49.
Callbacks include actual algorithm work. Numeric entry, visit/dispatch, debit/TLS
and exit remain prominent. Next count repeated frame geometry/entry-exit work;
do not infer function instruction totals by multiplying sample shares.

One diagnostic disassembly reader incorrectly assumed every byte before the next
symbol decoded as an instruction. Eight padding/non-mnemonic bytes violate that
precondition. The repaired reader bounds addresses and reports that gap; original
failure is retained. No compiler/runtime correctness failure remains in executed
scope. The source-borrow change is retained; PR stays draft for CI/review.

Sixteen diagnostic archives and sidecars (11,383,144 archive bytes, no native
binaries) have verified server digests. A fresh independent maintained replay
verifies1,272 file occurrences/473 paths and five preserved multi-version paths;
all four indexed reports reproduce byte-identically without compilation or
compression. Exact commands, exclusions, raw pairs and report hashes are indexed.

## R107: input scaling and CPU/memory profile; default compiler unchanged

The default implementation stays R105 at `8f6e6a8de9e4928a88f1341dd0538504bd2336e6`.
No new compiler/runtime code is added here. The ownership candidate
`4834caa483d1f79437dace189accdba82eaf9770` is saved separately on
`codex/branch-ownership-r106-experiment` at local checkpoint `1693fe593f`.
It remains held, with no universal speed claim. Its existing implementation,
controls, raw adverse observations and1KB gain are preserved.

### Matched uninstrumented timing of the held candidate

| Input | R105 / reference time | Candidate / reference time | Candidate time change |
|---|---:|---:|---:|
|8B|8.027041×|8.081192×|+0.674603%|
|64B|25.204773×|25.595225×|+1.549122%|
|256B|55.363354×|56.026910×|+1.198547%|
|1,024B|115.013911×|113.918447×|−0.952462%|

The8B primary comparison has3,968 observations and no improving pair. Larger
small cases are a separate cohort:128 observations/version/size, alternating
16-call blocks with the peer blocked on stdin. All eight descriptive blocks agree
with each size's direction; they are not independent experiment replications.
The64/256/1,024B inputs are deterministic prefix-consistent xorshift bytes. Their
exact reference raw bytes and independent inflate pass. Inputs differ from the
8B alphabetic case; the table does not isolate size from content across that row.
Original level12 computation, source/bytecode, search limits, real scratch sizes
and pinned allocator remain. Output capacity is the original bound `n+5` for these
small inputs, cleared/rebound outside evaluate_into; aliases and final managed
bytes/nodes0 pass. No full-scale/long benchmark is run.

### Fresh1KB counters, independent of timing and CPU sampling

| Metric | Reference | R105 | Held candidate |
|---|---:|---:|---:|
|Retired instructions|1,690,807|558,388,496|559,500,546|
|Cycles|973,713.5|112,154,903|111,583,370|
|Allocation requests, including reallocations|84|47,796|43,100|
|Cumulative requested bytes|9,012,988|7,523,247,338|20,234,842|
|Incremental peak live usable heap|9,257,328B|17,773,224B|15,173,352B|

Hardware has32 observations/subject; heap has32 reference and8 Mech observations/
subject after controls are excluded. Heap-instrumented instructions and elapsed
are excluded from hardware and primary timing claims. Counter read overhead is
retained. Candidate instructions rise0.199154% while cycles fall0.509593%.
Cumulative reservation requests are not bytes copied or resident memory. Despite
7.50GB fewer requested bytes, actual incremental heap peak falls only2.60MB and
whole-subject physical footprint stays roughly229MB. Reference footprint is about
14MB. This includes setup/image/runtime ownership, not merely the1KB input;
no exact whole-footprint attribution is established.

### CPU evidence and next bounded target

Fresh serial two-second samples contain1,329 Mech and1,517 reference exclusive
weights. Only65/1,329 Mech weights are generated-entry body;1,112 are other host
callbacks,101 tail cleanup and39 private-frame callbacks. Categories are disjoint;
callbacks include real algorithm work and are not all removable overhead.
Reference's leading opaque generated function has1,362/1,517 exclusive weights;
no source name is inferred from its hash. Sampling weights are not instructions.

The leading Mech leaves are numeric-segment entry85, ordinary visit82, compute
debit70, helper dispatch68, thread-local lookup64, numeric-segment exit53 and tail
cleanup40. Next, count actual repeated source previews/validation/footprint work
within each numeric entry, then test entry-local reuse before adding persistent
caches or changing admission. This targets work now demonstrated at the larger
small input. Shared debit inlining, buffer ownership, shape forwarding and prior
frame changes are already investigated; this report does not re-propose them.

An environmental removal around midnight deleted the pinned source/bytecode and
98 otherwise-clean tracked files. Missing tracked files were restored from exact
HEAD; source assembled byte-identically and bytecode restored from an existing
pinned snapshot. Both longstanding hashes match, with a persistent backup outside
temporary storage. A current-producer regeneration differed and was rejected
before execution. The cause of the removal is not established. Initial reference
platform-path and missing-input diagnostic failures remain in the packet; corrected
builds, counters and profiles pass. There is no compiler semantic change or fresh
compiler/CI qualification claim in this round.

Three diagnostic archives/sidecars (647,188 archive bytes, no native binaries)
are published with verified server digests. Fresh independent downloads verify
276 file occurrences/239 paths; only packet manifests have multiple versions.
All four timing/counter/sample reports replay byte-identically without a compiler
or compressor run. The compact index retains exact download URLs and hashes.

## R105: preserve live immutable shapes when finalizing private appends

Published parent `accd7f478628a073e3f4e0007a830c1f95ccb82a`; production fix
`341bf02918f4573fb518e12c5435ddd93efe0598`, measured compiler
`ec55e3f856306b07d34050b8b95ac0289a28fb5f`. Test-only canonical fixture repair
`05d3a9ce5`; separate guarded CI inventory `ec55e3f85` requires exactly four
private-append tests.

Review identified an introduced R103 correctness bug: a replacement shares its
immutable shape with a live old alias, but private append finalization tried to
mutate that shared owner. Two maintained Core tests reproduce the parent panic
`private shape`, including when the append branch is unselected. Finalization now
copies current parameters into its already admitted independent shape scratch,
performs the original checked length update, and installs that owner consistently.
No new allocation site, budget allowance or publication identity is introduced.

Fresh complete eight-byte elapsed **8.125431×→8.124210× Roc's time**:
2.356375→2.356021ms, common fresh reference0.290ms, −0.015023%. Only one of four
pairs improves; 28 descriptive blocks improve and36 worsen. All3,968 primary /
3,984 total observations remain. **Retain as a correctness repair, without an
elapsed-win claim.**

| Metric | Fresh retained R104 | R105 |
|---|---:|---:|
| Allocation requests |4,257|4,257|
| Reallocations, included above |68|68|
| Requested bytes |95,876,945|95,876,945|
| Incremental live heap peak |17,782,424B|unchanged|
| Instructions, separate cohort |27,013,011|27,014,335.5 (+0.004903%)|
| Cycles, same hardware cohort |7,388,324.5|7,411,965.5 (+0.319978%)|
| Whole linked text |63,678,544B|63,678,672B (+128B)|

The fresh complete census reconciles4,257 requests/2,683 stack groups/1,118 PCs
with the separate heap counters. Object, descriptor and metadata remain
byte-identical; generated text14,604,696B and entry stack135,728B stay fixed.
Logical work42,410,605,541 and versions143/99/38/14 remain. Algorithm, source and
bytecode hashes, allocation sites, real tables/capacities, pinned allocator, input
aliases and evaluate_into timer are unchanged. Exact raw bytes, independent
inflate, output capacity13 clear/rebind and final managed bytes/nodes0 pass;
physical output backing still changes.

Actual Core unit root:52 tests pass, including all four append cases, all unsigned
widths, selected/unselected append, live aliases, abort cleanup and a preparation
from an older value with a different current length. Current Core no_std check
passes locked/offline with warnings-as-errors. Twelve Core-dependent project
crates rebuild coherently under the exact retained selected-operation features;
external dependencies remain frozen. Full current formatting also passes.

Existing checks pass unchanged: runtime356, work/memory33, native CI65, image1;
components236, connected8, path1; linked16+12+12; scalar12 with maximum1.009568
under unchanged1.05 gate, pure-scalar counts/bytes equal and zero loop allocation
slope; frame8 with no request/byte/work increase; workflow132 and architecture /
type/memory15. The expensive knucleotide-body exclusion remains. No fresh full
all-features/minimal dependency graph, full no_std test suite or larger-input run
is claimed. Hosted CI is not awaited; the PR remains draft.

Failure disposition: the introduced parent panic is repaired. One added control
initially provided an unused third schema parameter, which canonicalization
prunes; its initial fixture error remains, and the corrected legal two-parameter
case passes. A proposed native source witness hit an existing exact match-schema
restriction before execution and was removed, without broadening source semantics;
it is not claimed as a runtime reproducer. A separate external diagnostic initially
omitted an existing fixture path; an unchanged-fixture symlink repairs that copy.
One successful linked test's archive write could not update Git metadata under
restricted access; its exact receipt was committed through authorized access.
No timing was rerun or altered for bookkeeping. No unresolved correctness failure
remains in this repair's executed scope.

Fresh isolated profiles retain695 complete calls per subject and1,265/1,308 sample
weights, excluded from primary timing. Current frame-action ancestry128/1,308,
tail cleanup95/1,308 and registration admission/removal23/1,308 are descriptive
samples with overlapping ancestry groups, not retired-instruction partitions.

The earlier registration question is now measured separately: each of two complete
eight-byte diagnostic visits has336 changed replacements,252 may-consume visits,
166 actual consumes, and all166 have an actually exclusive detached registration
in the same account/arena with the old root dead.24 nonreplacement consumes remain
separate. This establishes a166-candidate envelope, not an implemented allocation
saving. Diagnostic timings and allocations are excluded from performance claims.

Publication verifies17 archives and17 sidecars in the existing continuation
release, without native binaries. Fully fresh retrieval verifies1,372 file
occurrences /602 paths and reproduces all four indexed reports byte-identically.

The next main compiler target follows the branch-ownership brief: refine the
old-value query using one exact current Boolean, discard it at invalidating
boundaries, and retain Core's actual alias/account/arena/ownership checks. All four
current scanner append witnesses independently reproduce the impossible false-arm
read. The prepared-schema148-request change is already complete in R104; borrowed
descriptor validation and initial-turn plan overrides remain separate follow-ups.

## R104: reuse prepared schema headers; allocation improvement only

Published parent `a88acd5d87650e12cca6d28015334fa027989594`; production/compiler
`636da5e074add0632f7154f14d8d39848721cd16`. The main implementation is
`22554ca76fdbc9841245e28f696223550b50648c`; its explicit key-type repair is the
measured compiler above. Separate CI inventory: `f51c45f8febc5d4f9855ac970931a2bc3292ab77`.

Binding borrows existing prepared input/output keys and checks the retained
schema entry and template. Uncached live footprint calculation borrows the exact
retained encoded length/key independently of a DATA transaction; current values,
retained arenas, shapes, payload chunks and their original charges remain checked.
Unsigned-buffer no-ops use the existing prepared output key. No new cache, stored
prepared fields, permission, ABI or pricing change is introduced.

Fresh matched complete eight-byte elapsed **8.139130×→8.151457× Roc's time**:
2.3522085→2.355771ms, common fresh reference 0.289ms, **+0.151453%**. Two of four
pairs improve and two worsen; 29 descriptive blocks improve and 35 worsen.
Geometric block ratio 0.997238553 also remains recorded. All 3,968 primary/3,984
total observations are retained and committed. **No elapsed win is established.**

| Metric | Fresh retained R103 | R104 |
|---|---:|---:|
| Allocation requests |4,405|4,257 (−148; −3.359818%)|
| Reallocations, included in requests |162|68 (−94)|
| Requested bytes |95,881,838|95,876,945 (−4,893B)|
| Incremental live heap peak |17,782,424|unchanged|
| Instructions, separate cohort |27,046,906|27,012,942 (−33,964; −0.125574%)|
| Cycles, same hardware cohort |7,434,770.5|7,474,271.5 (+0.531301%)|
| Whole linked text |63,680,080B|63,678,544B (−1,536B)|

Retain as an immutable-metadata/construction improvement. The fresh complete
allocation censuses reconcile exactly with the separate heap counters. Binding
loses 66 requests/2,263B/42 reallocations; footprint loses the same; buffer no-ops
lose 16 requests/367B/10 reallocations. All three targeted encoding families become
zero, and the whole size/kind histogram difference is exactly those families.
Reallocations are already counted in requests. No other size/count bucket changes.

Object, descriptor and metadata are byte-identical; generated text14,604,696B and
entry stack135,728B stay fixed. Payload copying branches are unchanged; this is no
claim of measured payload-copy savings. No stored prepared fields/cache are added;
a separate prepared-heap census is not claimed. Work42,410,605,541 and versions
143/99/38/14 remain. Source/bytecode, level12 algorithm, real tables/capacities,
allocator, input aliases and evaluate_into boundary stay pinned. Exact raw bytes,
independent inflate, output capacity13 clear/rebind and final managed bytes/nodes0
pass. Public output physical backing still changes.

Passing current checks: 20 distinct actual Engine unit tests (two new; header,
current-payload, wrong-contract/arena, equal foreign IDs, overflow and exact finite
denial/spent-prefix controls); runtime356, work/memory33, native CI65, image1;
unchanged components236, connected8, path1; linked16+12+12; scalar12 and frame8;
workflow132 and architecture/type/memory15. The existing scalar gate1.05 is
unchanged; maximum ratio1.025436251. Pure-scalar requests/bytes remain equal and
loop allocation slope remains zero. Frame request/byte/work counts do not increase.
The two new tests have a separately committed CI step requiring exactly two
discovered tests. Local execution uses the retained richer selected-operation
profile; a fresh minimal graph for that CI profile is not claimed.

Five Engine-dependent crates rebuild coherently with the retained optimized
warnings-as-errors flags; Core, Syntax, Bytecode, math and external dependencies
remain frozen. Existing expensive knucleotide-body runtime exclusion remains;
no fresh all-features/no_std graph or larger-input benchmark is claimed. Hosted CI
is not awaited under instruction, and the PR remains draft pending green CI and
clean review.

Failures and diagnostic corrections are retained. The first two builds reject a
missing Option<SchemaKey> closure annotation (introduced, repaired in636da5e074).
Both corrected actual builds pass. A diagnostic disassembler initially counted
following symbols; original excerpts remain, and corrected static counts are
bounded by each symbol's address range. No performance claim uses those initial
counts. Static module matches may include cold/drop helpers; the census supplies
the executed encoding-family proof. No correctness failure remains.

The initial evidence upload hit GitHub's 1,000-asset release cap (HTTP422), an
environmental publication failure. The [continuation evidence release](https://github.com/mech-lang/mech/releases/tag/evidence-native-compression-experiments-2)
retains this round's complete packets. Partial original uploads and original
sidecars remain. Payload archive hashes are unchanged; canonical sidecars and the
round index point to the continuation. The maintained replay harness accepts an
optional per-round release tag while preserving historical default downloads.

The initial fresh replay verified all15 packets and reproduced three reports,
but the single entrypoint omitted the profile report generator. The original
failure and matching reports remain. The entrypoint now invokes its sibling
profile reader; a clean empty-output control passes. A supplemental immutable
packet retains that repair. Fully fresh retrieval verifies all16 packets,
1,402 file occurrences, 663 paths and ten multi-version paths; all four numerical
reports reproduce byte-identically. The prior R103 default-release path also
retrieves16 packets and reproduces its original four reports, and all132 workflow
tests pass again after the per-round release change. No compiler build or
performance observation was changed for these evidence-only repairs.

Fresh isolated profiles retain695 complete calls per subject, excluded from
primary timing. Their1175/1292 sample weights are not retired instructions.
Current registration admission/removal accounts for29/1292 weights, frame-copy
ancestry24/1292 and tail cleanup74/1292; ancestry groups overlap. These are no
removable-instruction estimates. Next, classify actual registration exclusivity
through consuming updates before attempting physical registration transfer;
166 remains a candidate envelope, with node release/reacquisition and fresh data
identities preserved. Shared immutable descriptor contracts follow that bounded
ownership check.

## R103: forward immutable shape owners; allocation improvement only

Production `a43888583dc37806a1c9e39b4b98d6dece3dac52`, measured compiler
`6b064cdeb678f2e7c89e5c2a39143e5ac2f36e79`, published parent
`9bc0fae10a806eeb5e0a8f980f3ce6d6df11ad72`. The Core unsigned reconstruction
constructor retains the existing immutable shape owner without retaining source
payload, root, registration or mutation permission. Both aliased and consuming
updates use ordinary numeric validation and create fresh frozen data identities.
Original conservative admission, checks, work and transfer order remain.

Fresh matched eight-byte elapsed **8.205418×→8.233392× Roc's time**:
2.3467495→2.354750ms, reference 0.286ms, **+0.340918%**. One of four pairs
improves; 23 descriptive blocks improve and 41 worsen. All 3,968 primary/3,984
total observations remain. **No elapsed improvement is established.** Historical
reference ratios and separate counter cohorts must not be combined into a gain.

| Metric | Fresh retained R102 | R103 |
|---|---:|---:|
| Allocation requests |5,077|4,405 (−672; −13.236163%)|
| Requested bytes |95,895,278|95,881,838 (−13,440B)|
| Incremental live heap peak |17,782,744|17,782,424 (−320B)|
| Reallocations |162|unchanged|
| Instructions, separate cohort |27,004,221|27,045,073 (+40,852; +0.151280%)|
| Cycles, same hardware cohort |7,868,378.5|7,401,114 (−5.938511%)|
| Whole linked text |63,675,184B|63,680,080B (+4,896B)|

Retain as an allocation/representation improvement with adverse elapsed and
instructions explicit. Fresh complete traces reconcile all 5,077/4,405 events
with bytes and reallocations. The whole size/kind histogram changes only by
336 eight-byte allocations and 336 32-byte shape owners; both disappear from
unsigned replacement ancestry. Compression payload copy branches stay unchanged;
13,440 requested bytes are metadata, not saved payload copying. Source implies
2,688 parameter bytes no longer cloned; no separate memcpy-volume cohort is claimed.

The complete Object, descriptor and metadata remain byte-identical, generated
text 14,604,696B and entry stack 135,728B unchanged. No prepared fields/cache are
added; a separate exact prepared-heap census is not claimed. Logical work
42,410,605,541 and IR/work/ABI/bytecode 143/99/38/14 stay fixed. Source, algorithm,
tables, capacities, allocator, aliases, cap13 output clear/rebind and evaluate_into
boundary stay pinned. Exact bytes, independent inflate and final managed
bytes/nodes zero pass. Public output backing still changes.

Current actual Core selections pass 48 tests: numeric8 (three new), unsigned
ownership14, string ownership7, identity3, tokens5 and admission11. Fresh locked,
offline no_std Core check passes with warnings-as-errors. Twelve dependent crates
rebuild coherently using unchanged selected features and frozen external dependencies.
Runtime356, work/memory33, native CI65, image1, unchanged components236/connected8/
path1, linked40, scalar12 (unchanged1.05 gate, max1.024748; allocation equality/
zero loop slope), frame8, workflow132, architecture15 and runtime-only GPU cfg pass.
Fresh separate profiles cover695 complete calls per subject.

One introduced test-only audit failure is repaired in `750adeea5`: raw-pointer
casts are replaced by Arc identity comparison. Clean published parent audit
passes; corrected actual Core48 and all15 architecture checks pass. Runtime
production files and measured binaries are unchanged, so measurements are retained.
Original failures/report precondition, repair, parent proof and reruns stay archived.
CI inventory5→8 is a separate commit, matching the three new tests; the old5 guard
is an explicit negative control. No allowlist, warning or allowance is weakened.

The existing giant knucleotide-body exclusion remains. No fresh all-features
dependency graph, full no_std suite, larger-input benchmark or green hosted CI is
claimed. Next: prepared schema-header reuse (148-request candidate, conditional
4,257 remaining); then classify registration reuse eligibility. Immutable contract
sharing/private-owner continuity and the independent frame/zero-argument/source
instruction track remain later work.

## Previous retained checkpoint: R102 — inline paired budget debit; retain measured gain

Compiler **`fa1ffe75e78c7ace538d7d14aa5f3ebc74783166`**, previous published head
`c489e5fc30a4e9cea549cc1de66e2c94ab2f9eb9`, stack base
`eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`. One `#[inline(always)]` keeps the
shared paired-debit checks in their hot callers. No checks, fee, limit, debit
order, worker atomicity or benchmark source change. R101's ordinary `#[inline]`
was ineffective, measured adverse, archived, independently replayed and reverted.

Fresh matched complete eight-byte compression **8.477859×→8.156141× Roc's time**:
2.4331455→2.3408125ms against a0.287ms reference, **3.794800% less elapsed time**.
All four matched pairs and64 descriptive blocks improve. All3968 primary/3984
total observations remain; blocks are not independent experiments. Historical
ratios with different reference clocks are not gain estimates.

| Metric | Fresh retained R100 | R102 |
|---|---:|---:|
| Instructions, separate cohort |27,930,744.5|26,988,708 (−942,036.5; −3.372758%)|
| Cycles, same hardware cohort |7,594,187|7,435,497 (−2.089625%)|
| Requests / reallocations |5,077 /162|unchanged|
| Requested bytes / incremental live peak |95,895,278 /17,782,744|unchanged|
| Generated text / entry stack |14,604,696B /135,728B|byte-identical /unchanged|
| Whole linked text, including runtime |63,674,544B|63,675,184B (+640B)|

Actual host disassembly removes the separate520-byte helper. The library caller
512→736B and observer428→628B now contain the checks directly. The complete
Object, descriptor and metadata stay byte-identical. This is a controlled
R100-to-R102 result; it does not assign all historical R99 instruction growth or
all Result transport to outlining. Fresh isolated profiles cover695 calls per
subject; samples remain separate from retired instructions.

Passes: actual Engine budget108, runtime356, work/memory33, guarded native CI65,
image1, unchanged components236/connected8/path1, linked40, scalar12 (same1.05
gate; max1.016923, pure-scalar allocation equality, loop slope0), frame8,
workflow132, architecture15 and current runtime-only GPU cfg. Five affected
crates rebuild coherently, Core/math frozen, warnings-as-errors unchanged. A
Git metadata permission denial occurred only when recording sixteen already
passing linked cases; the same sealed packet was committed with authorized
access. No program rerun, receipt rewrite or source/allowance change was needed.

Logical work42,410,605,541 and IR/work/ABI/bytecode143/99/38/14 are unchanged.
Algorithm, tables, buffers, allocator, input aliases, cap13 output clear/rebind
and evaluate_into timing stay pinned. Exact bytes, independent inflate and
final zero managed bytes/nodes pass. The existing expensive knucleotide-body
exclusion remains; no full math/all-features/minimal graph, larger-input timing
or green hosted-CI claim. Public output backing still changes.

The new complete allocation trace independently reconciles5077 requests,
95,895,278 bytes and162 reallocations. It finds336 eight-byte replacement
requests (immediate callee deduplicated; exact ancestry/return offsets retained)
and336 32-byte final shape owners. The672-request/13,440-byte shape-forwarding
candidate is confirmed as a current target, not an achieved saving. Next: shape
owner forwarding, then prepared schema headers. The independent instruction
track remains prepared frame transfers, known-zero arguments and entry-local
checked-source reuse.

## Previous retained checkpoint: R100 — move ordinary binding metadata; no elapsed win established

Compiler **`bebe5a245d312796c630de76fc558fbcd2c2f2ff`**, published parent
`c84e307aa64e136bd0d3f6d2fc2bbf869b4e3b25`. The no-witness binding branch now moves
the original BoundCall into its sole kernel consumer. Certified scalar kernels
still retain independent executor/seal metadata; all builder invalidation,
accounting, current-value validation and admission order stay unchanged.

Fresh matched complete eight-byte compression is **8.268488×→8.281428× Roc's
time**, 2.422667→2.4264585ms against a 0.293ms reference: **+0.156501%** pooled
median elapsed. Two pairs improve and two worsen; 25 descriptive blocks improve
and 39 worsen. All observations remain: 3,968 primary and 3,984 total. No elapsed
improvement is established. R99's historical 8.363107× used a different reference
clock and must not be subtracted from this ratio to claim a gain.

| Metric | Fresh R99 baseline | R100 |
|---|---:|---:|
| Instructions, separate cohort |27,950,216.5|27,933,245 (−16,971.5; −0.060720%)|
| Cycles, same hardware cohort |7,697,293.5|7,622,510.5|
| Allocation requests |5,149|5,077 (−72; −1.398330%)|
| Requested bytes |95,900,682|95,895,278 (−5,404B)|
| Incremental peak live bytes |17,782,744|unchanged|
| Reallocations |162|unchanged|
| Object/image/metadata |R99 artifacts|all byte-identical|
| Generated text / entry stack |14,604,696B / 135,728B|unchanged|

Retain this as a small allocation/instruction cleanup, with no elapsed-win or
peak-live-heap claim. Price formulas, logical work 42,410,605,541 and versions
IR/work/private ABI/MBC143/99/38/14 stay unchanged. Source, tables, search limits,
real scratch capacities, allocator, output capacity13 clear/rebind and evaluate_into
timing remain pinned. Mech computes the exact raw bytes and independent inflate
passes; input aliases and final zero managed bytes/nodes remain verified.

Passing current selections: nine actual Engine semantic-seal unit tests, runtime356,
work/memory33, guarded native CI65, image1, unchanged components236/connected8/path1,
linked40, scalar12, frame8, workflow132 and architecture15. Scalar gate remains1.05;
the isolated rerun's maximum ratio is1.010357, pure scalar requests/bytes stay
equal, and loop allocation slope remains zero. The initial scalar batch also
passed but is excluded because a descriptor read overlapped its start. All its
observations are preserved. An auxiliary Engine test driver initially used
metadata-only extern paths and an incompatible filename flag; switching to the
same frozen rlibs fixes the driver without compiler or allowance changes.

Five Engine-dependent crates rebuild coherently with the retained optimized
operation profile and warnings-as-errors; Core/math dependencies remain frozen.
The additional actual Engine unit root uses opt-level1 and the same selected
features. No new Core, all-features, full-math or larger-input qualification is
claimed. The existing expensive knucleotide-body runtime exclusion remains.
Fresh serial before/after profiles cover695 complete calls per subject.

### New instruction finding: runtime budget helper outlining

A current calibrated R97/R99 audit finds exactly9,801 callbacks in both versions.
R99 adds only eight ownership-candidate checks (980→988) across200 matrix updates;
57 buffer visits and260 buffer checks stay equal. Broad additional retirement
scanning is not established as the instruction regression's cause. Four fresh
hardware pairs reproduce about2.05% more instructions in R99, with lower cycles.
Instrumented windows remain estimates, separate from uninstrumented counters.

The budget source file is byte-identical across R97 and R99 (Git blob
`bbcc2b4c97ac2a61ebe6b0a912349c8a85ef33cb`). Actual disassembly finds a732-byte
library charge routine with its checks inline in R97. R99 instead has a512-byte
caller plus a separate520-byte charge_control_account_at_limits routine, with
intermediate Result transport. R100 still has this split. This establishes a
binary/code-generation difference; its exact contribution to the regression
remains unproved. The next bounded experiment explicitly requests inlining of
that helper and measures it separately, retaining every finite check and error
order. Prepared flat frame transfers and per-entry source reuse follow.

Seventeen immutable evidence packets verify1,249 file occurrences across538
paths and29 preserved multi-version paths. Fresh retrieval reproduces all four
reports byte-identically. The first retrieval exposed a missing linked metadata
input; a new replay-input packet fixes packaging without changing compiler source
or measured data. Commands, all raw cohorts, profiles, assembly excerpts and
diagnostic readers remain archived. Native binaries remain local. Local
validation is complete; hosted CI is cancelled by instruction and the PR stays
draft pending green CI and clean review.

## Previous retained checkpoint: R99

### Retire finished ordinary collection sources

Complete eight-byte level-12 compression improves from **8.470875× to 8.363107×
Roc's time**, **1.272220% less elapsed time** against the fresh matched baseline.
All four pairs improve; 61 descriptive blocks improve and three worsen. All
adverse observations remain. There are 3,968 primary and 3,984 total observations.
Blocks within a pair are descriptive, not independent experiments. The reference
is 0.289ms; Mech is 2.448083→2.416938ms. The target is 1×. Larger-input throughput
remains unestablished. R97's historical 8.378794× used a different reference clock;
it is not the baseline of this comparison.

| Metric | Fresh retained R97 baseline | R99 |
|---|---:|---:|
| Instructions, separate 32-call cohort |27,389,660.5|27,947,877 (+2.038056%)|
| Cycles, same hardware cohort |7,732,271.5|7,622,493 (-1.419745%)|
| Allocation requests including reallocations |5,153|5,149|
| Requested allocator bytes |97,997,834|95,900,682 (−2,097,152B)|
| Incremental peak live allocator bytes |17,782,744|unchanged|
| Reallocations |162|unchanged|
| Generated compressor text |14,604,696B|byte-identical|
| Machine entry stack |135,728B|unchanged|

The independent reference hardware cohort retires 1,364,372 instructions and
921,002 cycles. Reference allocation measurements retain 95 requests,
9,021,923 requested bytes and 9,257,328 incremental peak live bytes. These are
observed implementation results, not semantic minima. Hardware, heap, samples
and primary elapsed are separate cohorts. Requested capacity is not initialized
copy volume or live memory. Peak live heap does not improve. Hardware-cohort RSS
is 319,963,136→319,700,992B and footprint is 228,968,128→228,919,040B; these include
prepared code, libraries and harness state, rather than just compressor buffers.

## What changed

The Engine's old-value graph query previously treated CollectionSource as opaque,
so an ordinary completed comprehension could keep its input alive indefinitely.
The query now places ordinary materialized source reads at initialization,
header, body, latch, exit and collection finish. Finish matters: shape-preserving
finalization reads the arena source after the generator releases its source pin.
The same graph traversal preserves backedges, fresh definitions and read/write
order. Unsupported consumers remain conservative. Private-frame eligibility and
Core's current same-account/exclusive ownership check remain in place.

The real tree trace finds aliases 882 and 908 before each update. Previously 882
remained after retirement and forced a copy. After the fix neither remains; all
four 262,144-entry U16 updates reuse their 524,288-byte backing. Four complete
calls verify all 16 updates. This establishes exactly **2,097,152 initialized
bytes no longer copied per complete call**. The trace's arena scan is diagnostic
only; no production owner scan, source-field specialization or algorithm change
is introduced. The selected site's static inventory grows 45→47, adding 882 and
1857. Later readers and externally retained aliases still prevent unsafe reuse.

An independent Mach-O section audit finds the entire generated text byte-identical:
SHA-256 `65099c20101e43aaa1e355c0758a40316fc7ed6aa3ad358b1e0b5c8ba07020d4`.
Object/image identity and ownership metadata change. IR143 reconstructs the
new proof independently on image restoration; rejected R98's IR142 is not reused.
IR/work/private ABI/MBC are **143/99/38/14**, scalar ABI1. Strict prior-image
rejection remains. Price formulas are unchanged; actual logical work falls
42,449,879,781→42,410,605,541 as already-accounted copying/ownership paths change.
Logical work is not a hardware instruction count.

## Instruction tradeoff and fresh profiles

Whole-call instructions increase by 558,216.5 despite lower elapsed time and
cycles. This adverse result remains explicit; R99 is retained for its elapsed
and physical-copy reduction, without an instruction-reduction claim.

Separate nonnested windows around the four tree helpers observe raw instruction
totals 473,722→87,110.5 and cycle totals 209,935→32,007.5 per full call, over 32 calls
per subject. Empty read-pair calibration averages 5,006 instructions; raw window
totals include counter overhead and are not subtracted from the uninstrumented
whole-call cohort. No complete per-function instruction ledger is established.
The whole-call increase remains unattributed outside that bounded diagnostic.

Fresh serial profiles cover 695 complete calls per subject and 1,302/1,293
exclusive sample weights. Copy/move ancestry falls 41/1,302→15/1,293 weights;
frame-action ancestry is 131/1,302→113/1,293. These are overlapping ancestry unions
of CPU samples, not retired instructions or exact timer fractions. Runtime
helper, admission, native-segment and cleanup costs remain prominent.

## Revisions and qualification

Published parent: `64705927265fa331735ef39cdb3185471015054e`.
Retained comparison compiler: `20f002f4ff5ff2186888b7efc8517261756b425e`.
Regression/CI inventory: `bdf3df43a2c0d5dc9483ae3ade9793f85de2a0f7`.
Compiler fix: **`5c219ed220879b05ee147bdf5d3f85680311eae2`**.
Test-only final-root publication correction: `348101721a2870ab2f503508491e7d3395c94b32`.
Stack base: `eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`.

The new regression fails against the retained Engine at missing source eligibility,
then passes after the fix. Original/decoded artifacts and original/restored images
cover selected/unselected mapping, current arguments, a later published reader,
external input aliases, unused failing results, assertion/index error order,
unchanged prior publication and retained accounting through failure and retry.
Final managed bytes/nodes remain zero.

Passing current selections: runtime356, work/memory33, native CI65, image1,
unchanged components236, connected8, path1, linked16+12+12, scalar12, frame8,
workflow132 and architecture/type/memory15. CI's guarded inventory is updated
64→65 and actually lists and executes all65. Scalar allowance stays 1.05; the
maximum ratio is 1.018146. Pure scalar requests/bytes stay equal and loop allocation
slope remains zero. Frame request/byte/work counts do not increase. Component
expected values and allowances are unchanged.

Engine, Stdlib, Compute, Runtime and GPU are coherently rebuilt with the same
selected operations, optimized flags and warnings-as-errors. Core, Syntax,
Bytecode, math libraries and external dependencies stay frozen. No fresh Core,
full-math, minimal-dependency or all-features graph qualification is claimed.
The existing expensive knucleotide-body runtime exclusion remains explicit.
Formatting and architecture/catalog/type/memory checks pass.

Preserved preparation failures: an initial diagnostic reporting prefix of16 was
insufficient for45 static candidates and was replaced by a complete inventory;
this was no production resource denial. An initial narrow-arithmetic witness was
outside the retained lowerable subset and was corrected to the compressor's wide
arithmetic/cast pattern. The new test's initial publication expectation was
corrected to the language's final-root rule. No existing component golden changes.
No current correctness/test failure remains; the instruction increase remains
an adverse performance result.

## Fidelity, evidence and next bounded work

Source SHA-256:
`331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`.
Bytecode SHA-256:
`a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`.
Level12 retains the 262144-entry U16 tree, 1500513-word U32 cache, 305257-entry U64
path capacity, depth300, nice258 and original near-EOF rules. Search and skip stay
separate. Algorithm, tables, source allocation sites, allocator and evaluate_into
timing stay pinned. Exact raw output is `[75,76,74,78,73,77,75,207,0,0]`, checked by
independent inflate. Input aliases and output capacity13 clear/rebind remain.
Physical public-output backing still changes; physical output reuse is not proved.

Every measured cohort is sealed and committed. Fourteen uploaded evidence
archives verify 1,032 file occurrences, 463 paths and two preserved multi-version
paths. Fresh independent retrieval reproduces all four numerical reports
byte-identically without compilation or compression. The initial automatic upload
rejection was resolved by explicit authorization of evidence uploads. Native
Objects and executables stay local; descriptor/excerpt/hash retrieval does not
establish native-Object retrieval. No larger-input run is performed.

Next, explain the remaining instruction cost using exact callback/ownership
visits, remove the no-witness BoundCall clone, then stage immutable constructor
binding/planning and prepare flat frame-transfer geometry. Current funding,
values, error order and publication remain dynamic. The rejected R98 direct child
seed experiment stays reverted; its 112 executed handoffs were slower and are
not proposed again as unexplored work.
