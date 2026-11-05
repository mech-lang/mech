# R155: selected bounded loop, before any compiler change

The selected first loop is **OptimalPathLengths**. In this current exact-PC cohort, its four source clones account for 23.347639% of the 32,769-byte natural-text case, but only 3.133011% periodic / 3.494701% repeated. The word-extension source loops instead account for 19.396183% periodic / 19.936981% repeated and only0.286123% natural. These are CPU-time sample weights attributed to source locations, not per-function retired instructions or a predicted elapsed gain.

The natural case therefore supports the brief's research hurdle: halving work in a computation contributing roughly23% offers a plausible route to roughly10% complete-call improvement. This is a selection hypothesis, not an achieved or guaranteed speedup. The latest primary timings remain R154: natural32KB33.594437ms/7.708682×reference; periodic8.754792ms/7.015057×; repeated8.325417ms/6.226938×.

## Exact current provenance

- Frozen reviewed base: #984, ede060a777109403306b411446687b41a44f7ffe.
- Measured compiler emitter: 58690b58dfc4ccdf44b074c15d3c140a28603866; IR178/work107/ABI39/MBC14.
- Mech Object SHA-256: 2cfd693c6d522229d022bf16dc12e3570b76c1d5bba9d6dbb0f927e24c8bc3df.
- Actual worker SHA-256: 5522853af5581db472950ab3b3137205e42d0e7c3f2cf2fa152938b0cfad799e. Linked entry0x102f0ca00, Object entry0x200. No R149 Mech addresses reused.
- Source331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9; bytecodea2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7.
- Retained periodic reference SHA-256: 52d1961240d0fa49de81f42db098dd253fbdfbd190e6daf00d9c8631e39178cf. Its path-cost attribution is inferred from the complete distinctive algorithmic pattern, not the opaque symbol name.

Diagnostic-only source-location markers generated110152labels /299550machine spans. The entire emitted Object is byte-identical to the actual primary Object, not merely similar disassembly. The primary worker is unchanged. Three fresh focused recordings verify exact output, independent inflate, the same capacities/work/aliases and zero final managed storage. Every symbolized PC is cross-checked against the raw timer table. No primary elapsed claim comes from these instrumented batches.

## What the actual loops do

The reference candidate loop0x10001a3c8..0x10001a3f0 has11scalar instructions: two U32 loads (the node load reads its low cost half), two U32 wrapping additions, encoded-item OR, cost comparison, two strict-unsigned selects, length increment, trip decrement and backedge. The incoming best wins ties; the earliest improving length wins. No SIMD or access-bound branch occurs in that selected loop. Its cached-length guard and setup precede the loop.

The dominant current Mech copy is source iteration172, header15530, emitted within selected numeric phase15296. Other clones are138/12653,302/27129 and347/31320. Current hot machine code0x65ac0c..0x65b21c includes the loop's controls, accesses, body and backedge. Source-site15538 at0x65ac3c alone has2.575107% of natural-case weights;15539 at0x65ac4c has2.432046%; first projection15537 at0x65ac2c has1.373391%.

These are enum payload projections. The code has just selected the current Try tag, then checks that same tag again for k, best and item, each with its own cold error edge. The Done path similarly repeats checks. The cost-table read's source helper tests at0x65adac/65adb0 and emitted bounds test at0x65adc8/65adcc both survive. Further node access, narrowing guards and conditional-min control remain. A concrete narrowing redundancy is `and x24,x8,#0xffffffff` at0x65b1d0 followed by `cmp x24,0xffffffff` / unsigned<= rejection guard at65b1d8..65b1e0. The masked word necessarily fits U32. Generic current-word range facts can eliminate these checks alongside the window facts; do not mistake scalar `speed` selection for a proof that they already disappeared. Actual source and machine excerpts are retained, including the current backedge at0x65b21c→65ac0c. Inspecting the whole loop exposes substantially more than the earlier isolated packed-read guard.

**Funding correction:** this path phase already has checked_children=false, no cold exits, a prepaid compute bound397217883291064, and the child debit92480. The bound currently uses conservative65537trip bounds, not the source maximum256. The backedge still updates credit, a visit counter, and spill homes at0x65b1f8..65b218, but there is no per-candidate balance comparison here. Do not implement another child loan or claim that zero source-attributed funding samples mean zero accounting instructions. Reuse the existing envelope and later consider exact total-body settlement once safety is established. Word extension does have material incremental funding weights in this cohort; it is a different path.

## Existing architecture to reuse

The packed-loop strategy already owns a bounded current-traversal fact engine (packed_loop/edge_facts) with expression identities, selected-edge relations, successful arithmetic, conservative joins, malformed-tag partitions and latch resets. Common aggregate emission already accepts a GuardPolicy backed by current-site proofs. Numeric regions currently pass CheckedGuards unconditionally. This is an existing capability that has not reached the hot numeric projection path.

The next implementation should share that fact machinery through a numeric action adapter; avoid introducing another expression-specific recognizer. Start by proving current selected-variant projections and keeping all unknown/invalid paths checked. Then connect the same current-traversal representation to the length-cost/node windows and candidate range from the brief. The candidate's child is only 116 source sites, so using a local traversal can respect the existing analysis bounds without imposing a smaller source-program limit or increasing allowances.

Before expansion, add a small Try/Done three-field loop witness that reproduces repeated guards on the reviewed parent. Cover changed tags on fresh visits, unselected variants, nested/repeated calls, original/decoded/restored execution, malformed admission/guard controls, finite error prefix, unchanged publication/aliases, cleanup and retry. Reconstruct facts from the current plan; no saved permission or benchmark-name matching. Confirm actual instructions disappear, then immediately screen natural/periodic/repeated and an eight-byte regression control. Preserve all adverse results. Use the fixed20-case/scalar/frame gates only after a credible mechanism and focused gain.

## Limits of this selection

The source-location map does not partition every generated instruction: untagged locations and backend-created gaps remain explicit. In natural input,32.961373% of all weights have a nonnumeric/default source tag and16.108727% have no source span; do not silently assign them to these loops. Backend merging/hoisting can select a shared origin, so these are source-attributed weights, not a complete exclusive wall-time ledger. Hardware instructions, cycles, allocations and primary timing for a future candidate still need their own cohorts. No compressor source, allocation site, algorithm, search limit, buffer capacity or timing boundary changed in this research checkpoint. No optimization has yet been retained in R155.
