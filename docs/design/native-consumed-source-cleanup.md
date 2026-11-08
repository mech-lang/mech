# Finish consumed sources after checked indexed updates (R218)

**One emitter file honors the existing guarded consume permission after successful indexed updates, including unchanged results. All640 compiled controls and all20 exact streams pass. The variant is preserved as an alternative; the faster R217 strategy is retained after the separately documented R219 measurement qualification. Normal compilation remains R215/IR215.**

Base `5846c50ccc12aa756ecf063da26c3cca1a2b7fb4`; normal compiler `42c95cfd79e0dc3773d03aeb9f553d32ff3922c5`; frozen comparison diagnostic `efd9069b589512630891810191e7de35d57325d8`. Normal IR/work/ABI/MBC215/107/39/14 stays. Same draft PR973/branch.

## Real operation and design

At the real two-store sequence17002→17008, source home45280/value9627 has consume permission and existing guarded-last-use proof while union liveness still holds it. An unchanged first store returns the same owner. Retaining the result without ending the old source creates a second token and forces the next store to copy. A fresh query verifies the permission at17002; the query is diagnostic-only and does not create a new ownership proof.

R218 uses consuming=true AND live_after(source)=true, which requires the already existing guarded proof. It excludes any source defined by the same operation. After the entire checked update succeeds, its result owner has been retained and old destination owners released; only then clear/release the old source home. Later retirement sees zero. Genuine aliases, the original uniqueness/capacity check, same-site output exclusions and predicate-redefinition refusals remain. All fallible source checks/preparation precede this ownership commit.

This removes R217’s other-home family filter and its change-edge hook: only compact/emit.rs changes, and the runtime/allocator stay byte-for-byte unchanged.153 static source-cleanup sites;752 nonnull visits in the small controls are distinct quantities. No full-compressor visit census was collected.

The minimum witness is unchanged from R217: an unchanged indexed write followed by a changing write, while a later contradictory branch keeps an older view union-live. First successful three-step call still uses2 allocations/zero writable-copy bytes, versus5/12 before. Private,alias,duplicate,duplicate-alias modes preserve exact status/origin/value/capacity rows, saved output/backing and retry across original/decoded images.

## Original four-way timing cohort

Same24 balanced side orders;192 retained calls/side for inputs<=1024B,24 for larger inputs. NormalR215, retainedR211, previousR217 and R218 execute identical fixtures. Every call checks exact reference bytes and independent inflate. This original per-call handshake/median result is preserved, including its failure.

| Input | Normal / Roc | R211 / Roc | R217 / Roc | R218 / Roc | R218 vs R211 | Gate |
|---|---:|---:|---:|---:|---:|---|
| reymont-prefix-32769 | 3.466989× | 2.752560× | 2.738866× | 2.743718× | -0.321% | pass |
| 33027-periodic-window | 3.703781× | 2.598319× | 2.596143× | 2.614168× | +0.610% | pass |
| repeated-32769 | 3.165812× | 2.397093× | 2.400124× | 2.415749× | +0.778% | pass |
| random-32769 | 5.228096× | 4.542341× | 4.479960× | 4.488602× | -1.183% | pass |
| 16384-distribution-change | 6.776095× | 4.210885× | 4.161800× | 4.160590× | -1.194% | pass |
| eight-literals | 4.482255× | 0.815157× | 0.819683× | 0.812214× | -0.361% | pass |
| reymont-prefix-16384 | 4.234592× | 2.788216× | 2.766512× | 2.753920× | -1.230% | pass |
| empty | unresolved | unresolved | unresolved | unresolved | +0.000% | pass |
| one-stored | unresolved | unresolved | unresolved | unresolved | +16.400% | FAIL |
| seven-stored-prefix | unresolved | unresolved | unresolved | unresolved | -7.045% | pass |
| eight-overlap | 4.358564× | 0.789936× | 0.810121× | 0.798516× | +1.086% | pass |
| 320-overlap | 4.161371× | 0.873607× | 0.892418× | 0.891721× | +2.073% | pass |
| eight-literals-prefix | 4.465482× | 0.796170× | 0.834691× | 0.813537× | +2.181% | pass |
| 128-biased-dynamic | 3.832234× | 0.767239× | 0.774428× | 0.766651× | -0.077% | pass |
| 32-high-entropy-stored | 4.151535× | 0.871491× | 0.867837× | 0.864695× | -0.780% | pass |
| random-64 | 4.155784× | 0.912945× | 0.901562× | 0.908435× | -0.494% | pass |
| random-256 | 4.293887× | 1.163515× | 1.103667× | 1.105814× | -4.959% | pass |
| random-1024 | 3.836862× | 1.227269× | 1.162796× | 1.154451× | -5.933% | pass |
| repeated-1024 | 4.021087× | 1.011081× | 1.029951× | 1.034574× | +2.324% | pass |
| reymont-prefix-1024 | 4.543464× | 1.393043× | 1.364585× | 1.362661× | -2.181% | pass |

Original19/20 point gates pass; one-stored250→291ns fails. Its raw means283.625→285.390625ns differ0.623%. The platform tick is125/3ns; a single median-bin jump is larger than the5% bound. R219 separately investigates this inherited measurement issue without hiding this cohort or increasing the allowance.

## Instructions, allocations and tradeoffs

| Input | R211→R218 requests | Writable-copy bytes | R218 instructions vs R211 | R218 instructions vs R217 | R218 cycles vs R211 |
|---|---:|---:|---:|---:|---:|
| reymont-prefix-32769 | 5184→608 | 949428→14932 | -0.457% | +0.360% | -0.659% |
| 33027-periodic-window | 502→208 | 75316→2188 | +0.668% | +0.770% | +0.094% |
| repeated-32769 | 228→208 | 5192→2188 | +0.820% | +0.770% | +0.288% |
| random-32769 | 4262→276 | 916352→3352 | -0.520% | +1.111% | -0.871% |
| 16384-distribution-change | 3176→522 | 589360→8880 | -0.843% | +0.720% | -0.742% |
| eight-literals | 368→230 | 27260→2188 | -0.863% | +0.810% | -0.198% |

R218 and R217 have identical payload/request/copy/zero/peak counters on every input. Natural payloads2588→300/global5184→608; random2127→134/global4262→276; distribution1584→257/global3176→522. R216’s owner pool is absent.12 R218 instruction medians fall/eight rise vs R211; only one is lower than R217.16 cycle medians fall vs R211. Counters are independent cohorts with unadjusted overhead.

Reported code: R2115597664B, R2175608036B, R2185589884B. Root stack61120/61344/61520B respectively. A smaller patch and smaller code do not establish a faster executable. The measured R217 alternative is preserved.

Writable-copy counters exclude generated stores/direct memmove and are not all memory traffic. Requested capacity, usable peak, process RSS and static stack are different metrics.

## Validation and current profiles

-640 new before/after original/decoded controls pass;364 resident values/160 errors/116 inherited default-limit denials. Small original/decoded Objects and metadata are byte-identical. Four modes preserve genuine aliases, fresh recursive arguments, error ordering, saved publication, cleanup and recovery.
-40 full smokes/11488 original primary calls/11328 retained/4160 separate hardware+heap calls; fresh reference1280 windows/10240 compressions. All20 outputs agree exactly. Largest33027B.
-Six actual-candidate wall profiles22464 correct calls and six current-PC profiles65519 calls. Native symbol lists, exact Object/executable digests and current ARM64 excerpts are retained. Samples include surrounding worker/validation and are not per-function instruction counts.
-No new compiler/test failures; original tiny point failure is preserved and separately qualified. No allowance/expected value changes. Normal source/fixtures are unchanged. Historical normal678 tests/seven exclusions and15 architecture/132 workflow checks are not claimed newly rerun.
-No multi-MB, ten-workload suite, full dependency rebuild or second full-MBC Object build. Full compression uses strict decoded MBC14. Ordinary CI is cancelled by direction, review retained, PR draft.

## Fidelity and disposition

Source SHA `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`; strict MBC14 SHA `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`. Same nightly2026-03-03/Cranelift0.131.3/mimalloc2.1.7. No LLVM or source change. Preserve algorithm/tables/search/EOF, source allocation sites, real262144-U16 tree/1500513-U32 cache/305257-U64 path capacity and eight-byte output capacity13, input/output aliases and timing boundaries.

Both native variants omit ordinary exact admission/accounting/canonical publication/recovery contracts. They are not qualified normal entry or managed NoLimits. NormalR215 remains unchanged and parity is unachieved. Physical final-output backing reuse is not established.

Do not promote the simpler variant solely because its patch is smaller. The R219 refined inventory favors R217 by a0.184% geometric point estimate and most instruction medians. Keep R218’s source-consumption insight and every contrary result for a future conditional ownership-token transfer experiment; do not discard the measured faster alternative.

## Public evidence

[Source patches, every timing cohort, clock qualification, controls and exact-executable profiles](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r218-r219-owner-timing-80d995f4da67.tar.gz) (3230292 bytes;877 files). SHA256 `80d995f4da6724048919dab20275431d07b9dbe7fa06e686a1f899770f9dbf22`. [Publication verification and exact heads](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r218-r219-publication.json).
