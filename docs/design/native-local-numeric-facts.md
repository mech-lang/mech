# Current-visit facts through numeric writes — R179

Retain a bounded compiler improvement. Natural text at 32 KB improves 4.183810× to 3.720143× Roc (11.082413%); natural16 improves 10.435046% and random32 10.185168%. The pinned compressor source, canonical plan and numeric recipes are unchanged. The compiler preserves facts that remain true after a checked replacement and retains a closed supported prefix before an opaque boundary.

## Design and scope

The previous numeric adapter abandoned an entire local scope at a Write, Append or unsupported child edge. That discarded facts established before the boundary and facts about unrelated immutable input. The new Replace effect preserves immutable scalar snapshots, target shape/length and unrelated array facts on the successful continuation. This fact domain contains no array-content equality assumptions. Original write bounds/type checks, comparisons, ownership decisions, first-change materialization, account validation, charges and error paths remain.

The shared Scope implementation finds the supported part reachable from a fresh header, cuts at unsupported actions or opaque child edges, and prunes every nonheader continuation entered from outside that part, including its descendants. It uses the existing acyclic transfer/merge engine and original analysis allowances. Growth, rebinding and unknown effects remain boundaries. No control flow is skipped merely because a boundary is opaque.

Only ordinary local substitutions use that prefix. Captured/window versions and completion retain their full-scope requirements. A partial result discards exported loop-wide windows. Completion and terminal-expression recording explicitly reject Replace as authority for totality. Local facts stay within the current supported visit. Unknown child returns, cold edges and fresh activations keep their original admission, and no old permission is retained. Native image validation rederives the same source facts, with IR199 invalidating the previous native identity; work/ABI/bytecode stay 107/39/14. No new serialized skip flag, runtime dispatch or user syntax is introduced.

This completes the first bounded fact-recovery experiment. Current child admission still checks its current arguments and views. Mapping caller predicates into the child gate remains a later semantic step. The separate R172 source-frequency ledger/variant is already complete and stays separate; its gain is not combined with this compiler comparison.

## Generic witness and machine evidence

The maintained witness uses independent U32 input, an ordinary mutable U64 buffer carrier, scalar-state children and fresh invocations. Four source toggles separate a preceding write from a nested call. For six qualified original/decoded cases, the clean parent emits zero proved parent read guards and the candidate emits one. Independent resident execution, literal expected values, aliases and fresh values agree before the final parent coverage assertion fails. The candidate passes. A read added after the opaque child gets no inherited prefix proof.

The combined preceding-write/child shape already declines the complete numeric region with “mutable children require current-account visit funding.” Its existing native fallback behavior remains covered by the independent oracle. This patch leaves that funding refusal intact. The three qualified shapes have actual emitted text reductions of 300, 220 and 704 bytes; original/decoded Object hashes agree within each compiler. Their stack sizes stay at 240 or fall from 336 to 320 bytes. The combined fallback text stays 16,524 bytes. These are emitted-code counts, not only recognizer counts.

In the complete compressor, 40 additional projection guards and 510 additional scalar read guards are removed. Prepared readonly/write numeric regions, all 22 guarded-loop selections, complete reduction recipes and lowered pack selections are exactly equal. Independent compilation-only labels regenerate the whole current Object byte-for-byte. In the inspected parent at header 15296, source 15428–15431 previously emitted four repeated compare/branch pairs and their error origins; the candidate emits none of those tagged source/error spans. That selected projection segment removes eight ARM64 instructions and no stack accesses; its source selector remains. This is a segment count, not a total parent-to-child cost or dynamic visit census. Actual excerpts use each Object’s own mapped coordinates.

Generated text shrinks 8,955,224→8,601,148 bytes (−354,076). Entry stack grows 15,792→17,440 bytes (+1,648). Both prologues still use one immediate move and one stack subtraction for their local allocation; no extra stack probe is inferred. The larger stack is retained as an explicit tradeoff. The physical minimum kernel was not separately retuned or assigned an old instruction count.

## Complete-call timing

One predeclared balanced eight-block cohort contains 3,968 primary/4,048 total complete calls. All 20 unchanged 1.05 gates pass: 17 faster/3 slower medians; geometric elapsed change -3.754262%. All eight paired blocks improve for each of the seven priority cases. Three stored-only medians worsen by 0.121355–0.358408%; zero reference medians remain unresolved. Contrary observations are preserved.

| Input | Before / Roc | After / Roc | Elapsed change | Improving/adverse blocks |
|---|---:|---:|---:|---:|
|reymont-prefix-32769|4.183810×|3.720143×|-11.082413%|8/0|
|reymont-prefix-16384|4.951733×|4.435018×|-10.435046%|8/0|
|random-32769|5.427371×|4.874584×|-10.185168%|8/0|
|16384-distribution-change|6.970006×|6.415489×|-7.955760%|8/0|
|repeated-32769|3.205891×|3.076759×|-4.027966%|8/0|
|33027-periodic-window|3.728267×|3.565564×|-4.364050%|8/0|
|eight-literals|4.709432×|4.593000×|-2.472322%|8/0|
|empty|unresolved (zero median)|unresolved (zero median)|+0.358408%|4/4|
|one-stored|unresolved (zero median)|unresolved (zero median)|+0.346842%|4/4|
|seven-stored-prefix|unresolved (zero median)|unresolved (zero median)|+0.121355%|5/3|
|eight-overlap|4.505080×|4.410279×|-2.104322%|8/0|
|320-overlap|4.306716×|4.216485×|-2.095118%|8/0|
|eight-literals-prefix|4.622763×|4.560659×|-1.343439%|7/1|
|128-biased-dynamic|3.970542×|3.860161×|-2.779991%|7/1|
|32-high-entropy-stored|4.286989×|4.197954×|-2.076867%|6/2|
|random-64|4.293829×|4.221815×|-1.677147%|7/1|
|random-256|4.372255×|4.270128×|-2.335790%|7/1|
|random-1024|4.073437×|3.934248×|-3.417001%|8/0|
|repeated-1024|4.225928×|4.095593×|-3.084183%|7/1|
|reymont-prefix-1024|4.845807×|4.688434×|-3.247607%|8/0|

Every complete call checks exact pinned reference bytes, independent inflate, identical logical source work, input aliases, prefixes, real capacities and final managed bytes/nodes zero. Largest input 33,027 bytes. Ratios from separate rounds or source variants are not compounded. The timer is the full evaluate_into call; setup, restoration, clear/prefix/rebinding, validation and cleanup remain outside it under the unchanged harness.

## Independent counters and memory

| Input | Instructions before | Instructions after | Instructions change | Cycles change |
|---|---:|---:|---:|---:|
|eight-literals|12542665.0|12489030.5|-0.427616%|+2.845981%|
|random-32769|79517170.5|72561005.5|-8.748004%|-9.215186%|
|16384-distribution-change|71843971.5|66499723.0|-7.438687%|-7.876273%|
|repeated-32769|74003229.0|72374293.0|-2.201169%|+1.097985%|
|33027-periodic-window|79762971.5|78099983.5|-2.084912%|-0.729328%|
|reymont-prefix-16384|136289287.5|127148519.5|-6.706887%|-10.378929%|
|reymont-prefix-32769|242296244.0|225143791.5|-7.079124%|-11.246402%|

Across all 20 cases, 17 instruction medians fall and 3 rise; 6 cycle medians rise. The adverse cycle cases are empty, one-stored, seven-stored-prefix, eight-literals, 128-biased-dynamic and repeated-32769. Eight-byte cycles rise 2.845981% and repeated32 cycles 1.097985% in this cohort despite their primary elapsed improvements. Those contrary observations remain. Whole-call retired events, CPU-time samples, static instruction sequences and source logical work are separate quantities. No sample percentage is multiplied into an instruction count.

All 20 allocation requests, requested bytes, reallocations, before/after/peak usable-live-heap and incremental peak medians are exactly equal. This is an instruction/control improvement with unchanged heap construction. Equal requested capacity or usable heap is not a copied-byte or streaming result. Real source buffers and allocation sites remain fixed.

| Input | CPU RSS before | CPU RSS after | Physical before | Physical after |
|---|---:|---:|---:|---:|
|eight-literals|207060992.0|309444608.0|233424576.0|233637632.0|
|reymont-prefix-32769|215072768.0|311001088.0|233424576.0|233637632.0|

Initial eight-byte RSS rises 49.446115%, while physical footprint rises 0.091274%. A separate four-session fresh ABBA check retains all 40 full calls and four memory maps: RSS change -0.070895%, physical change -0.010540%. The initial RSS increase does not recur. Initial and repeat readings both remain; the initial cause is not proved. No allocator change, purge or system-memory saving is claimed.

## Current profiles and scalar controls

Five fresh exact-primary-binary CPU profiles cover 17,559 complete calls. Whole-Object byte identity is independently verified before mapping any current PC. Current source-location maps contain 115,800 tags/279,470 spans and 32 fine reduction labels. CPU-time sample weights describe emitted regions, not an exact algorithm/overhead split. Outside-entry includes other generated helpers, useful runtime algorithms and validation; unknown native attribution remains explicit.

eight-literals: 10.668357% inside the main generated entry; 5.397573% unknown native attribution.
random-32769: 71.492537% inside the main generated entry; 34.570896% unknown native attribution.
16384-distribution-change: 51.469783% inside the main generated entry; 26.163958% unknown native attribution.
repeated-32769: 72.094718% inside the main generated entry; 30.236794% unknown native attribution.
reymont-prefix-32769: 81.026664% inside the main generated entry; 39.742427% unknown native attribution.

All 12 original/decoded Bool/U64/I64/F64 and 0/64-loop scalar controls pass the original 1.05 gate, with unchanged allocation counts/bytes and no iteration-dependent allocation slope. Ratios range 0.961564222–1.005926034. The adverse scalar observations remain.

| Case | Form | Steps | After / before |
|---|---|---:|---:|
|scalar-bool|original|0|0.992979111|
|scalar-bool|decoded|0|0.999616812|
|scalar-u64|original|0|0.997990806|
|scalar-u64|decoded|0|0.996933031|
|scalar-i64|original|0|0.998046009|
|scalar-i64|decoded|0|0.993275077|
|scalar-f64|original|0|0.997318792|
|scalar-f64|decoded|0|0.999465396|
|scalar-loop|original|0|0.961564222|
|scalar-loop|decoded|0|1.005926034|
|scalar-loop|original|64|0.998868280|
|scalar-loop|decoded|64|1.002121751|

## Qualification, revisions and failures

Four new focused tests pass. Full GPU qualification passes 639 tests with the same seven inherited exclusions; all 222 native CI-selected names are listed and verified executed. New source controls cover original/decoded, direct/restored, fresh current inputs, selected/unselected visits, zero/out-of-input positions, aliases, exact outputs, assertion-before-arithmetic order, overflow, unchanged prior publication, cleanup and successful recovery. The existing six-dimension finite-boundary helper covers both the replacement and opaque-child shapes without raising any allowance. A separate completion regression rejects the checked replacement as totality authority.

Fresh 236-component/eight-composition/one-real-capacity-path qualification passes with a byte-identical wrapper and unchanged expectations/allowances. All 15 architecture/catalog checks, 132 workflow tests, runtime-only, formatting and warnings-as-errors pass. No parser or unrelated compiler work is included.

Preserved witness development failures are representation/coverage mistakes: an open matrix FSM payload was rejected by existing closed-kind semantics; a closed managed variant did not enter numeric lowering; an attempted combined scalar child exposed the existing funding refusal. The final witness uses the maintained ordinary buffer carrier and scalar-state children, retaining that refusal as a control. The clean-parent missing-fact assertion is the intended negative regression, not an output disagreement.

The first measurement startup was denied read-only ps access before starting either worker and produced zero primary observations. The same inventory and sampling command then ran with approved host access. No primary observation was discarded or rerun. There is no unresolved introduced functional failure; the same inherited exclusions and the combined-shape optimization refusal remain explicitly recorded.

Published parent `176d8134133befb8e35a6413fb8f98387bacebb3`; exact clean performance source `8f01da96d1203e60fc26078d19a9f8807cd6ffc6`; retained before compiler `09c57a46a33fd7d5420a59f60b71e3abf373a3b7`. IR/work/ABI/bytecode 199/107/39/14. Unit compilation occurred before the local Git commit, after all final files were written/formatted; its ten-file tracked patch is reconstructed byte-for-byte, the two then-untracked test/fixture files are snapshotted, and all twelve files match the final clean-commit performance build. The later commit changed only Git metadata. Source reconstruction from the public parent and exact patch reproduces every source-file hash.

Object `ffbe09255bcbff4c6d808a7d1fa8f292b5ac55a4cbefc4927d8d66901f6b514a`; worker `a16290e23e62628f87abdc9d0490c56498b3001285e4c6d1971bea942dc8156f`; source `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`; bytecode `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`. Frozen R145 Core/Engine/math/dependencies, pinned nightly-2026-03-03/aarch64, GPU opt-level 3 and unit/component opt-level 1, canonical-native/program/runtime plus jit/aot, pinned mimalloc2.1.7. No full math/dependency rebuild. Exact commands/environments appear in library/producer/worker builds, unit/, gpu-compatible-final*.json, checks/receipts.json, ci-inventory.json, qualification/, probe receipts and every measurement/profile spec.

Preserve original algorithm/tables, depth 300/nice 258, near-EOF limits/ties, 262144-entry U16 tree, 1500513-word U32 cache, 305257-word U64 path capacity, source allocation sites, output capacity 13 with clear/reuse, aliases and full API boundary. Mech performs the complete computation. Logical work is not hardware instructions.

Fresh all-feature/dependency builds, another standalone AOT qualification, multi-megabyte runs and the separate ten-workload suite are skipped. The compressor executes through its linked Object. Ordinary hosted CI is cancelled by standing instruction; review automation is preserved and #973 remains draft pending green CI and clean exact-head review. One optimization commit in the existing PR.

Public hash-indexed evidence retains raw cohorts, source/fixture pins, generic before/after receipts, exact-current maps/disassembly, qualification and numerical replay. Native binaries, bytecode, raw trace containers, copied user briefs and private environment exports are excluded. Every timing case was sealed immediately. The next bounded compiler step is current child-edge predicate mapping after this retained patch is reviewable; facts must establish the particular current window and keep residual checks and ownership admission. The broader brief and 1× goal remain active.
