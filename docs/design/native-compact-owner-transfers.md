# Dead-source aggregate owner transfers (R211)

**Preserve the complete diagnostic improvement; production remains R199 /IR214.** Geometric complete-call elapsed changes-57.017303%;19 medians fall,0 rise,1 are equal. All20 unchanged1.05 gates pass against both R210 and unchanged production. All20 instruction and cycle medians fall. This is an experimental compiler result with ordinary contract omissions, not production adoption or parity.

## Exact scope and design

Base `2b5049786feb3dfece5b7c743dc45a0ff352009f`; effective production `7748204bd12e57a20369ce7970f31f8d15c3901f`; PR base `92f75db84e2523693648084bea16cd52ad60777a`. IR/work/ABI/MBC214/107/39/14. Source SHA256 `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`; MBC `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`. Same M1/8GiB host, nightly2026-03-03, Cranelift0.131.3, sparse SSA frontend and pinned mimalloc2.1.7. One measured commit in existing draft#973; no new branch or PR.

R210 hottest random/distribution root PC0x37af4 was traced through its free-failure edge0xed5fc and cold error return0x6b4588. The existing origin constant identifies canonical site2338, the literal-observation CompositePack(4), followed by copies2339/2340. Pinned source lines473–477 return unchanged frequencies, updated observations and two counters. All three actual sites now transfer both eligible owner lanes. This is source attribution from the timed Object and Plan, not a guessed function label.

The existing owner-flow solver already records field-sensitive copy edges. Retain those edges and select one held source reference per source lane when no current output post-use needs it and the source is dead afterward, either unconditionally or under the existing stable MUST-path proof. A same-site source definition/overlap declines. Duplicate source uses move once and retain every other destination normally. Distinct live aliases retain independent references.

Keep the shared Pack/Copy/Project emitter unchanged. It performs all original guards and reads before destination writes. Only after successful shared emission does native code zero the moved source owner lane and omit its destination retain. Previous target snapshots are still released; ordinary retirement and edge cleanup still run. Current heap exclusivity is still checked before mutation. A failed source action reaches its original cold error without any transfer. Complete publication post-uses are explicitly excluded.

Three compiler files change from R210; the full archived diagnostic has30 changed compiler files. Static eligibility is8,236 lanes at5,515 sites, not an executed-count claim. Source and timed artifact hashes remain unchanged throughout qualification.

The full level12 computation, algorithm/tables, depth300/nice258, original EOF limits,262144-U16 tree,1500513-U32 cache,305257-U64 path capacity and source allocation sites remain. Actual allocation, initialized copies, native work, output transfer and private cleanup remain inside complete-call timing. Preparation/clear/prefix/import/rebind/output slots/validation/inflate/drop stay outside. Output capacity13 on eight literals and saved aliases remain verified. Physical final-output backing reuse and a complete all-memory-traffic census remain unestablished.

## All twenty matched elapsed results

8,616 complete Mech calls /8,496 retained primary observations,12 balanced permutation blocks per case;5,120 fresh verified reference compressions. Each row uses the same current reference denominator for all three variants. Do not combine unmatched historical multiples or compound separate rounds.

| Input | Production / Roc | R210 diagnostic / Roc | R211 diagnostic / Roc | Diagnostic elapsed change |
|---|---:|---:|---:|---:|
| reymont-prefix-32769 | 3.375661× | 4.951098× | 2.688841× | -45.692% |
| 33027-periodic-window | 3.508200× | 5.312800× | 2.529616× | -52.386% |
| repeated-32769 | 3.206685× | 5.117834× | 2.462476× | -51.884% |
| random-32769 | 5.168768× | 17.381165× | 4.574694× | -73.680% |
| 16384-distribution-change | 6.916824× | 15.416745× | 4.178916× | -72.894% |
| eight-literals | 4.501242× | 2.214912× | 0.836039× | -62.254% |
| reymont-prefix-16384 | 4.091257× | 5.625488× | 2.729908× | -51.473% |
| empty | unresolved | unresolved | unresolved | +0.000% |
| one-stored | unresolved | unresolved | unresolved | -22.133% |
| seven-stored-prefix | unresolved | unresolved | unresolved | -12.575% |
| eight-overlap | 4.322159× | 2.138554× | 0.804355× | -62.388% |
| 320-overlap | 4.208260× | 2.334184× | 0.913903× | -60.847% |
| eight-literals-prefix | 4.445094× | 2.192057× | 0.825160× | -62.357% |
| 128-biased-dynamic | 3.692493× | 2.017051× | 0.763841× | -62.131% |
| 32-high-entropy-stored | 4.120525× | 2.465155× | 0.887546× | -63.996% |
| random-64 | 4.034770× | 2.561160× | 0.913427× | -64.335% |
| random-256 | 4.193882× | 3.344837× | 1.152286× | -65.550% |
| random-1024 | 3.773638× | 3.831998× | 1.232772× | -67.830% |
| repeated-1024 | 4.062241× | 2.658424× | 1.057436× | -60.223% |
| reymont-prefix-1024 | 4.638784× | 4.079997× | 1.437248× | -64.773% |

The three tiny stored-only reference medians round to zero, so their multiples stay unresolved. Equal/adverse values remain in the raw cohort; no favorable unchanged rerun replaces an observation.

## Separate counters and memory

2,400 complete Mech calls supply the separate CPU/heap cohorts. All20 cases have fresh reference counters:1,280 windows /10,240 verified compressions from the exact40 SHA-checked executables and pinned source/allocator. Counts include unadjusted counter-window overhead. CPU instrumentation allocation zeros are not heap results. Timer samples are not per-function retired-instruction counts.

| Input | R210 instructions | R211 instructions | Change | R211 instructions / Roc | Cycle change |
|---|---:|---:|---:|---:|---:|
| reymont-prefix-32769 | 259,028,797.5 | 161,624,183.0 | -37.604% | 3.396831× | -45.918% |
| 33027-periodic-window | 85,356,607.5 | 53,546,285.0 | -37.268% | 2.779474× | -52.257% |
| repeated-32769 | 84,088,787.5 | 52,932,053.0 | -37.052% | 2.768166× | -52.230% |
| random-32769 | 197,518,546.5 | 69,934,343.0 | -64.594% | 5.950449× | -73.271% |
| 16384-distribution-change | 136,453,997.5 | 47,419,145.0 | -65.249% | 5.061578× | -72.930% |
| eight-literals | 5,156,745.5 | 2,006,386.0 | -61.092% | 1.470556× | -61.377% |
| reymont-prefix-16384 | 142,645,688.5 | 82,432,700.0 | -42.212% | 3.321328× | -51.896% |

| Input | Allocator requests before → after | Requested bytes before → after | Tracked COW copy-byte change |
|---|---:|---:|---:|
| reymont-prefix-32769 | 9,192 → 5,184 | 11,532,208 → 10,731,644 | -704,372 |
| 33027-periodic-window | 900 → 502 | 9,737,755 → 9,658,395 | -69,808 |
| repeated-32769 | 322 → 228 | 9,598,351 → 9,581,410 | -14,685 |
| random-32769 | 5,718 → 4,262 | 11,388,253 → 10,603,819 | -749,490 |
| 16384-distribution-change | 4,670 → 3,176 | 10,238,392 → 9,765,214 | -437,322 |
| eight-literals | 498 → 368 | 9,091,628 → 9,059,092 | -29,416 |
| reymont-prefix-16384 | 9,630 → 5,360 | 11,050,220 → 10,216,224 | -731,516 |

Guard-proved dead references had survived conservative multi-path retirement. Moving them earlier removes those dead aliases before later concrete exclusivity checks, so some helpers can reuse backing instead of copying. This is more than eliminating counter arithmetic. Real live/saved/duplicate aliases remain protected by both the proof and current heap check. Requested capacity, initialized COW helper copies, usable peak heap and process RSS remain different measurements; copy counters are not a complete memory-traffic ledger.

Equal diagnostic heap cases:3/20. All raw counter/heap deltas remain in results.json. Peak/RSS values are not inferred from allocation count.

Generated text including all7 phases and2 word kernels changes7,669,096→5,637,108B. Root stack84,672→61,120B; root-plus-largest-direct-kernel envelope85,056→61,504B. These are static metadata, excluding allocator/host stack and RSS.

## Dynamic mechanism and fresh profiles

The separate observer executes40 complete calls. Numeric phase, word-map and word-pack execution counts match R210 on all20 inputs; computation has not been skipped to obtain the speedup. Non-null reference moves are counted individually at actually reached source sites; zero-owner observations are separate. The default timed image imports no observer.

| Input | Actual non-null owner transfers in one observed complete call | Literal pack / first copy / second copy |
|---|---:|---:|
| reymont-prefix-32769 | 4,950,603 | 4978 / 4978 / 12216 |
| 33027-periodic-window | 1,476,578 | 82 / 82 / 338 |
| repeated-32769 | 1,448,988 | 6 / 6 / 260 |
| random-32769 | 6,428,859 | 65412 / 65412 / 65454 |
| 16384-distribution-change | 4,226,438 | 32468 / 32468 / 32538 |
| eight-literals | 164,833 | 16 / 16 / 16 |
| reymont-prefix-16384 | 2,962,521 | 3614 / 3614 / 7118 |

Six wall-stack profiles execute22,415 correct complete calls; six exact-PC profiles execute64,815. Both use the exact timed default executable; native phases and word kernels count as generated code.

| Input | Generated exclusive | Host below generated code | Other worker/system |
|---|---:|---:|---:|
| 16384-distribution-change | 94.493% | 2.535% | 2.972% |
| 33027-periodic-window | 94.746% | 2.977% | 2.277% |
| eight-literals | 57.929% | 25.087% | 16.984% |
| random-32769 | 96.992% | 2.354% | 0.654% |
| repeated-32769 | 95.719% | 2.359% | 1.922% |
| reymont-prefix-32769 | 97.857% | 0.875% | 1.268% |

Fresh actual PCs show the matching word loop at0x665c/0x6670 and selection entry/finish at0x8518/0x85d8 now exposed. Natural has2393/5360 timer samples in numeric phase15296; repeated1841/5332 and periodic1874/5554 in matching phase2624. Random retains4612/5507 samples in the root, including repeated large stack-slot loads at0x15fe5c/0x15bbe0 and remaining retain/release branches. These are time samples, not instruction-event counts or removable-time promises. Preserve this baseline and begin measured ordinary-contract restoration on compact state; do not repeat the rejected R205/R206 production experiments as unexplored work. The newly received R210 review brief sets the next order: complete the explicit parallel-assignment controls, measure phase-boundary transport, then qualify one compact phase inside ordinary production. General whole-compressor contract restoration is not a prerequisite to that bounded integration.

Fresh C/Rust owner-return controls check64 owned/clone/COW cases and64 C struct returns; actual Objects are disassembled. The Rust owned-return function provides a static move comparison, while the clone function retains live aliases. The exact pinned full reference executable is separately SHA-checked and inspected; its symbol contexts alone are not a source-function ownership proof. Fresh pointwise C/Rust controls also retain224 linked map cases. These are mechanism controls, not full C/Rust compressor timing claims.

## Qualification and failure disposition

-640 new observation-record controls /16 parent/candidate original/decoded forms pass: unique/live/duplicate aliases, repeated and fresh visits, zero trips, current scalar counters, assertion→load→division→updates/overflow order, exact origins/capacities, unchanged prior publication, private cleanup and retry.364 resident value and160 resident source-error agreements;116 inherited default16Mi work-cap denials match the parent exactly. No allowance increases.
-Fresh1,480 all-width map cases,320 fixed-buffer cases,192 origin cases,48 phase cases,1,724 matching cases and40 builder cases pass;120 original/decoded/observer whole-stream smokes verify exact bytes and independent inflate. Same142 matching and24 fixed resident-cap denials remain explicit. All original/decoded image pairs agree byte-for-byte.
-Ordinary Bool/U64/I64/F64 scalar and total-map Objects, metadata and descriptors remain byte-identical before/after and original/decoded (eight ordinary images). Efficient checked scalar entry remains. No fresh production native-suite or scalar-latency claim is inferred from image identity.
-Optimized candidate libraries and consumers compile with warnings-as-errors. All30 archived compiler files pass format; maintained module/native-build layout checks and full patch application pass. Feature profile remains aot/jit/canonical-native/program/runtime. Frozen dependencies are reused; the math library is not rebuilt.
-Introduced harness failures are preserved and resolved: unsupported named-record recursive-argument fixture shape was adapted to the compressor’s explicit argument/returned-record shape; resident default-cap assumptions were inventoried against the parent; new observer keys required separating phase tags. Environmental process-inspection denial occurred before measurement workers and the authorized rerun passed. No compiler/primary cohort was silently rerun.
-The same seven inherited native exclusions remain. Full dependency/all-feature rebuilding, multi-MB timings, the separate ten-workload suite and a fresh maintained production native/architecture/workflow suite remain skipped. Historical R206677/260 counts are not fresh R211 results. Ordinary exact-head CI is cancelled by instruction; #973 stays draft pending green CI and clean review.

Production still remains R199. The complete brief is unfinished: exact resource accounting, canonical error-prefix/recovery, canonical ownership/publication and full external admission/import need measured restoration on compact state. Production exclusive-host operation and complete initialized-copy ledgers remain open. Prior rejected production experiments are preserved, not relabeled as unexplored opportunities.

Branch cleanup removes90 surplus remote experiments and54 local branches, preserving #973/#990, open prerequisite PR heads/bases, unrelated projects and uncommitted work. The portable recovery bundle is local and not uploaded. Further rounds stay on #973’s existing branch, one commit per measured round.

## Reproduction

Principal scripts: measure-three.py, measure-counter-cohorts.py, qualify-transfers.py, qualification-pipeline.py, profile-current.py and their exact command receipts. From an extracted packet, heap-summary.py then analyze.py reproduce numerical results without compiling or running a compressor. Native reproduction needs the recorded pinned toolchain and dependencies. Native binaries, bytecode, executable descriptors and raw trace stores are excluded from public evidence.

## Newly received review brief

R211 implements the main dead-source transfer experiment identified by that review. Same-site source/destination overlap currently declines transfer and retains the existing parallel snapshot implementation. Dedicated self-copy, swap, overlap, metadata-only and predicate-redefinition witnesses remain to complete before broader transfer changes. No claim of full brief completion is made.

Next: finish those witnesses, measure fixed-buffer phase-boundary transport, then qualify one compact phase through ordinary admission/ownership/funding. Do not postpone that integration until the whole diagnostic wins everywhere. The separate coverage table compares numeric phases, packed reads, total maps, scalar leaves and owner transfers. Stable owner pooling and payload-site census remain later isolated work.

The timed old Object materializes source owner-error origins2338/2339/2340; none remains in the new Object. The retained source/Plan mapping and observed lane counts separately establish why those specific owner-release failure paths disappear. This is not a claim that all root reference counting disappears.

[Exact source patch, all20 timings/counters, controls, Object excerpts and numerical replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r211-owner-transfers-0c1694e4a6e1.tar.gz) (5321428 bytes;1560 files). SHA256 `0c1694e4a6e1f5876109a99405b25a765fbc717fc9a8abc69f64107cb914fe55`. Fresh public verification checks server digests, every file and byte-identical numerical replay.
