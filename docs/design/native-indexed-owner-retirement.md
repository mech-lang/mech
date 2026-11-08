# Retiring unused owners before indexed updates (R217)

**The compiler experiment removes most indexed-update copies using an existing path proof. It is not adopted: empty input fails the unchanged elapsed gate. Normal compilation remains R215/IR215; the retained whole-native diagnostic remains R211.**

Base `f2d890a0e0288ebe66703c360ce17e29f1b8f269`; normal compiler `42c95cfd79e0dc3773d03aeb9f553d32ff3922c5`; retained diagnostic `efd9069b589512630891810191e7de35d57325d8`. IR/work/ABI/MBC215/107/39/14 stays unchanged in normal compilation. The four-file diagnostic patch is archived, not installed as a qualified normal entry. Same existing draft PR973 and branch.

## Real operation and evidence

R216 attributed2306/2588 natural32 payload allocations to indexed updates. A fresh observed image resolves the refusal: ALL these indexed-copy events already have consume permission but see exactly two current references. The two hottest sites17323/17008 hold an older array and an intermediate result that can return the same buffer after an unchanged first update. Exact homes: slots45623/45734 and45280/45400. The old home remains physically retained even when the current must-facts make all its future dereferences unreachable.

Eighty static home queries distinguish34 already-proved-dead other homes from six conservative refusals. At the two hot sites, the EXISTING guarded-last-use proof succeeds for the older home and it is not read at the current operation. No new Boolean equivalence, changed fact-kill rule or runtime arena scan supplies ownership permission.

The actual pinned reference binary also checks reference count1 in `_list.RocList.makeUnique` at0x100031f74, returns the original view on the unique edge, and calls `_roc_alloc` followed by `_memcpy` on the shared edge. The indirect-symbol table resolves memcpy at0x100032380. Disassembly/pins are retained. This proves that helper pattern, not its dynamic visit counts or every caller lifetime. The target is fewer stale references, not ignoring real aliases. No new C/Rust measurements are claimed.

## Compiler change

Reuse OwnerFlow field liveness and current must-facts. The existing owner-copy edges plus possibly unchanged buffer results form a conservative family filter; connectivity is only a candidate filter and never an ownership proof. Select another currently held home only if it is in that family, is not a current read, definition or same-site output, and existing guarded-last-use proves it dead on every possible continuation.

For a changing indexed update, perform bounds validation, old element loads, value evaluation and unchanged-result selection first. Retire the proved-dead other home only on that reached changing edge, then use the ORIGINAL refcount/capacity gate and writable fallback. Old scalar metadata stays available. Source owner words become zero so later ordinary retirement cannot release them twice. Checked fallback, genuine shared readers, current output uses and predicate redefinitions remain conservative.

Seventy-three eligible static sites each retire one candidate home. There is no site/name/field-name specialization. Four compiler files change: OwnerFlow, concrete array emission, root emission and numeric storage construction. Runtime/allocator and the compressor source stay unchanged. Whole-compressor retirement visit counts were not collected;744 nonnull retirements in the small compiled controls and exact full-call allocation deltas are separately reported.

A minimal source witness reproduces the real operation: an unchanged indexed store, then a changing store, with an older view retained by a later contradictory branch. Before/after40 exact controls agree; the first successful three-step call falls5→2 payload allocations and12→0 tracked copy bytes. Four private/alias/duplicate combinations then supply640 before/after original/decoded controls.

## Complete timing matrix

Same24 balanced blocks / six side orders repeated four times;192 retained observations/side for inputs<=1024B,24 otherwise; fresh256 reference compressions/case. Real complete compression, actual allocation/copy/final transfer/private cleanup are inside the diagnostic timer. Caller clear/prefix/arguments/output slots, verification/inflate and final output drop retain the original exclusions. Normal evaluate_into is separately measured with its original guarantees and boundary.

| Input | Normal R215 / Roc | Retained R211 / Roc | Candidate / Roc | Candidate vs R211 | Gate |
|---|---:|---:|---:|---:|---|
| reymont-prefix-32769 | 3.437758× | 2.742809× | 2.738871× | -0.144% | pass |
| 33027-periodic-window | 3.668254× | 2.609824× | 2.603523× | -0.241% | pass |
| repeated-32769 | 3.112251× | 2.400999× | 2.398440× | -0.107% | pass |
| random-32769 | 5.061770× | 4.511857× | 4.437717× | -1.643% | pass |
| 16384-distribution-change | 6.613204× | 4.187816× | 4.126050× | -1.475% | pass |
| eight-literals | 4.429884× | 0.816021× | 0.799138× | -2.069% | pass |
| reymont-prefix-16384 | 4.203414× | 2.786424× | 2.756066× | -1.090% | pass |
| empty | unresolved | unresolved | unresolved | +19.617% | FAIL |
| one-stored | unresolved | unresolved | unresolved | +0.000% | pass |
| seven-stored-prefix | unresolved | unresolved | unresolved | +0.000% | pass |
| eight-overlap | 4.288302× | 0.802978× | 0.795390× | -0.945% | pass |
| 320-overlap | 4.113642× | 0.889980× | 0.874303× | -1.762% | pass |
| eight-literals-prefix | 4.453287× | 0.815239× | 0.805076× | -1.247% | pass |
| 128-biased-dynamic | 3.675555× | 0.757679× | 0.741041× | -2.196% | pass |
| 32-high-entropy-stored | 4.129388× | 0.872660× | 0.851170× | -2.463% | pass |
| random-64 | 4.095935× | 0.933634× | 0.893469× | -4.302% | pass |
| random-256 | 4.201836× | 1.166666× | 1.072739× | -8.051% | pass |
| random-1024 | 3.746449× | 1.220122× | 1.122387× | -8.010% | pass |
| repeated-1024 | 4.040512× | 1.037115× | 1.035769× | -0.130% | pass |
| reymont-prefix-1024 | 4.567925× | 1.390906× | 1.349177× | -3.000% | pass |

17 lower medians / one higher / two equal; equal-case geometric elapsed improves1.090%. Empty209→250ns fails the unchanged1.05 gate. No acceptance allowance increases, expected-value changes or selection of only favorable cases. The candidate stays unretained while its larger-case gains remain preserved. Tiny reference medians round to zero; no reference multiple is fabricated.

## Independent instructions, allocations and copying

| Input | Global requests before→after | Payload requests before→after | Tracked copied bytes before→after | Instructions | Cycles |
|---|---:|---:|---:|---:|---:|
| reymont-prefix-32769 | 5184→608 | 2588→300 | 949428→14932 | -0.829% | -0.409% |
| 33027-periodic-window | 502→208 | 247→100 | 75316→2188 | -0.102% | -0.254% |
| repeated-32769 | 228→208 | 110→100 | 5192→2188 | +0.050% | -0.159% |
| random-32769 | 4262→276 | 2127→134 | 916352→3352 | -1.614% | -1.380% |
| 16384-distribution-change | 3176→522 | 1584→257 | 589360→8880 | -1.553% | -1.512% |
| eight-literals | 368→230 | 180→111 | 27260→2188 | -1.673% | -5.792% |
| empty | 4→4 | 1→1 | 0→0 | +0.094% | +4.786% |

Natural32 global requests fall88.272%, with requested bytes10731644→9687324 and unchanged incremental usable peak9750840B. Most large peaks remain equal; some smaller cases fall. The removed global requests reconcile as one payload and one48-byte owner record for each avoided copy. Requested-byte savings reconcile exactly with removed payload capacities plus owner records. The R216 pool is not included. Zero-initialized bytes are unchanged on every case.

Writable-copy bytes count runtime writable-helper initialized copying only; generated stores/direct memmove are not a complete memory-traffic census. Requested capacities, live-usable peak, RSS, static native stack and copied bytes are different metrics.

16 instruction medians fall/four rise;17 cycle medians fall/three rise, separate from primary elapsed. Natural32 instructions161624763→160284739; random69930145.5→68801710. Empty instructions6359→6365, cycles2256.5→2364.5. Counter overhead is not subtracted. All adverse cohorts remain.

Reported code5597664→5608036B (+10372B), root stack61120→61344B (+224B). Actual before/after entry assembly is retained; neither excerpt emits a stack-probe loop, so the stack increase is not asserted to cause the tiny-input regression. Register/transport changes remain a plausible next investigation, not a proved explanation.

## Exact timed executable profiles

| Input | Generated exclusive | Host under native | Other worker/system |
|---|---:|---:|---:|
| 16384-distribution-change | 95.031% | 1.874% | 3.095% |
| 33027-periodic-window | 95.365% | 2.930% | 1.705% |
| eight-literals | 59.140% | 24.012% | 16.848% |
| random-32769 | 97.603% | 1.046% | 1.351% |
| repeated-32769 | 95.236% | 2.972% | 1.792% |
| reymont-prefix-32769 | 98.124% | 0.524% | 1.353% |

Six wall profiles22514 correct calls; six exact-PC profiles66472 correct calls. Symbols, Object digest, sampled current PCs and ARM64 hot excerpts are taken from the exact candidate. Timer weights include surrounding worker/validation work and are not retired-instruction shares or a partition of removable overhead.

## Validation and failure disposition

-640 freshly compiled controls /16 mode-side-form combinations pass: self/current state, live original aliases, duplicate sources, fresh recursive visits, selected checks before arithmetic, bounds/overflow/denial controls, exact origins/values/capacities, unchanged saved output/backing, cleanup and retry. Original/decoded Object and metadata pairs are byte-identical.364 resident values /160 errors /116 inherited resident default-cap denials are preserved.
-40 full smoke calls and8616 primary complete calls /8496 retained all match exact pinned bytes and independent inflate;3120 separate Mech CPU/heap calls,1280 fresh reference windows /10240 compressions. Largest input33027B.
-Diagnostic refusal and home images each execute40 complete calls with the same payload/copy counts as R216’s census. The80 static home queries and exact source origins are retained. Current-path proof refusals are preserved, not overridden.
-Normal maintained source is unchanged. R215678 maintained passes/seven inherited exclusions and R21615 architecture/catalog plus132 workflow passes remain historical; no newly repeated ordinary Rust suite is claimed. Current report/index and diff checks are recorded separately.
-One harness error is resolved: added counter reporting tried to borrow Heap after moving it into retained publication. Snapshotting the two scalar counters before the move fixes it. No cloning Heap or ownership workaround. Final builds keep warnings as errors.
-One introduced elapsed acceptance failure remains: empty input. The candidate remains archived/unretained. No semantic failure remains in the tested cases; no claim of universal correctness beyond that coverage.
-No multi-MB timing, separate ten-workload suite, full dependency rebuild, second full-MBC Object emission or completed remote CI. Review automation is retained and ordinary exact-head CI is cancelled by standing direction. The PR stays draft.

## Next bounded simplification

The current candidate explicitly finds and retires another stale alias at the later changing update. A simpler follow-up is to finish the consumed SOURCE home immediately after the earlier checked buffer operation succeeds, using the consume/guarded-last-use facts already computed. When an unchanged first update returns the same owner, that should prevent the extra retained token from surviving into the second operation at all. It may eliminate the extra family plan and reduce the new transport/register cost. This is a proposed experiment, not implemented or measured here.

Reuse the new minimal witness and all20 gates; preserve failure-before-commit ordering, genuine aliases and source same-site outputs. Do not loosen the empty gate or adopt the unchanged rejected pool. A full ordinary whole-native replacement and parity remain unfinished.

Source SHA `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`; strict MBC14 SHA `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`. Same nightly2026-03-03/Cranelift0.131.3 and mimalloc2.1.7; no LLVM change. Preserve level12 algorithm/tables/search/EOF rules, source allocation sites, real262144-U16 tree/1500513-U32 cache/305257-U64 path capacity and eight-byte output capacity13. Borrowed input/output aliases stay live; physical final-output backing reuse is not established.

The native diagnostic still omits ordinary exact resource/admission/accounting/canonical publication/recovery contracts. It is not a qualified normal entry or NoLimits managed execution. Fresh normal/diagnostic multiples do not establish a contract-cost floor.

Replay `python3 heap-summary.py` then `python3 analyze.py`; this only replays preserved observations. Native reproduction requires pinned source/dependencies and recorded build commands. Public evidence excludes native binaries/Objects, bytecode, executable descriptors and raw traces.

## Public evidence

[Exact compiler patch, all20 timings/counters, mapped homes/proofs,640 compiled controls and fresh profiles](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r217-guarded-indexed-retirement-deb2e8e72b9c.tar.gz) (3249254 bytes;789 files). SHA256 `deb2e8e72b9cfebd38c2fc8d7c69e6755d2394d1f1139e64ad32c14584c472d6`. [Publication verification and exact head receipt](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r217-indexed-owner-publication.json).
