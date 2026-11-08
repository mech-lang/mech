# Fixed-buffer nested native phases (R209)

**Production remains R199 / IR214.** The diagnostic now executes three additional existing matching phases through the shared numeric emitter. Periodic/repeated complete-call elapsed falls 43.72%/43.78%, natural32 falls 21.27%; the diagnostic still fails six production gates. This is a measured experimental improvement, not production adoption, parity, or a cost-of-contract floor.

## Exact checkpoint and change

Base `3196185106203540f3eade734010125604ace5bc`; retained compiler `7748204bd12e57a20369ce7970f31f8d15c3901f`; PR base `92f75db84e2523693648084bea16cd52ad60777a`. Production IR/work/ABI/MBC remains **214/107/39/14**. Source SHA-256 `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`, MBC `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`. Reference package `671077457ff7ef3a3d44caf715a3fc595addf379`, compiler `d99c7b838fb05bb1047eb0c18d4d06d0f86f345d`, mimalloc2.1.7 unchanged. Existing draft #973 only.

The broad matching parent837 has cold/child exits and is correctly declined. Its selection entry nevertheless hid complete disjoint child proofs1279/2624/3835. Independently, the diagnostic required append-capacity coordinates even for fixed U16/U32 updates. The new selector chooses the largest complete disjoint existing proofs and passes their explicit region identity to the same emitter; ordinary selection is unchanged. No proof or resource condition is relaxed.

The boundary retains actual nonnull/unique writable-owner checks, then checks actual logical length against actual capacity. Refusal uses the original source CFG. Fixed capacities travel as extra typed input words and are returned unchanged; they are not invented append coordinates. Append phases retain their original layout/growth. A numeric bounds error is normalized to the raw diagnostic bounds status while preserving the reached origin. New generic controls establish exact old/new status, origin, values and capacities.

The archived full prototype changes 27 compiler files; this round changes four: numeric_minimal.rs, numeric_outlined.rs, readonly_numeric.rs and compact/phases.rs. Full patch SHA-256 `5ee09927eb1650d6510e973b59da5657263ef41f22d90da542cadc3a6b8b6c33`. No compiler source or timed artifact changes after primary. The patch applies cleanly to the reported base. Production source/dependencies remain unchanged.

Algorithm, tables, depth300/nice258, original near-EOF/search behavior, 262144-U16 tree,1500513-U32 cache,305257-U64 path capacity, source allocation sites and eight-byte output capacity13 remain pinned. The measured complete call includes actual allocation/growth/copying/output transfer/private cleanup; preparation/clear/prefix/import/rebind/output slots/validation/inflate/drop remain outside.

## All twenty matched complete-call results

8,616 complete Mech calls /8,496 primary observations, twelve balanced order blocks per case, plus5,120 separate verified reference compressions. R208 is the previous diagnostic; production is freshly measured unchanged R199. No successful primary or counter cohort was rerun.

| Input | Production / Roc | R208 diagnostic / Roc | R209 diagnostic / Roc | Diagnostic elapsed change |
|---|---:|---:|---:|---:|
| reymont-prefix-32769 | 3.368571× | 6.927688× | 5.454108× | -21.271% |
| 33027-periodic-window | 3.521251× | 12.365468× | 6.959786× | -43.716% |
| repeated-32769 | 2.952729× | 11.432981× | 6.427938× | -43.777% |
| random-32769 | 5.051964× | 19.488120× | 19.059515× | -2.199% |
| 16384-distribution-change | 6.449541× | 15.162178× | 14.926120× | -1.557% |
| eight-literals | 4.498693× | 2.208044× | 2.200206× | -0.355% |
| reymont-prefix-16384 | 4.080072× | 6.809875× | 5.666882× | -16.784% |
| empty | — | — | — | +19.617% |
| one-stored | — | — | — | +11.200% |
| seven-stored-prefix | — | — | — | +11.200% |
| eight-overlap | 4.431402× | 2.166883× | 2.151857× | -0.693% |
| 320-overlap | 4.133848× | 2.442030× | 2.291806× | -6.152% |
| eight-literals-prefix | 4.485627× | 2.186774× | 2.201582× | +0.677% |
| 128-biased-dynamic | 3.739800× | 2.030602× | 2.050503× | +0.980% |
| 32-high-entropy-stored | 4.123984× | 2.460439× | 2.444395× | -0.652% |
| random-64 | 4.188446× | 2.658726× | 2.659164× | +0.017% |
| random-256 | 4.306651× | 3.405750× | 3.405964× | +0.006% |
| random-1024 | 3.878507× | 3.929112× | 3.930554× | +0.037% |
| repeated-1024 | 4.028887× | 3.186606× | 2.599821× | -18.414% |
| reymont-prefix-1024 | 4.577033× | 4.177769× | 4.017401× | -3.839% |

**12 medians fall /8 rise; equal-case-weight geometric change −7.346585%.** All twelve blocks improve for natural32, periodic33 and repeated32. Only17/20 unchanged1.05 gates pass versus R208: empty209→250ns, one-stored375→417ns and seven-stored-prefix375→417ns fail. These introduced experimental regressions are preserved, not rerun away. Their reference clock rounds to zero, so no elapsed multiples are defined. Fourteen gates pass versus production; the same six larger cases fail. Eight literals is 2.200206× reference in this cohort, a small mixed-pair −0.355% change (seven blocks lower/five higher).

## Separate counters and memory

2,400 complete Mech CPU/heap calls and fresh same-input reference1,280 counter events /10,240 verified compressions use the same exact forty SHA-checked reference binaries. Whole-window counters include unadjusted probe overhead; CPU-probe allocation zeros are uninstrumented.

| Input | R208 instructions | R209 instructions | Change | R209 instructions / Roc | Cycle change |
|---|---:|---:|---:|---:|---:|
| reymont-prefix-32769 | 344,627,116.0 | 285,791,225.5 | -17.072% | 6.006442× | -21.828% |
| 33027-periodic-window | 252,053,019.0 | 111,081,676.5 | -55.929% | 5.766015× | -43.702% |
| repeated-32769 | 251,935,216.5 | 111,386,424.5 | -55.788% | 5.825130× | -43.968% |
| random-32769 | 227,280,512.5 | 225,888,502.0 | -0.612% | 19.219998× | -2.561% |
| 16384-distribution-change | 137,172,595.5 | 136,870,562.0 | -0.220% | 14.609807× | -1.457% |
| eight-literals | 5,136,409.0 | 5,189,547.0 | +1.035% | 3.803616× | -1.364% |
| reymont-prefix-16384 | 164,473,195.0 | 143,169,190.5 | -12.953% | 5.768499× | -16.981% |

Nine instruction medians fall/eleven rise; eleven cycle medians fall/nine rise. Eight instructions rise1.03%, despite its small favorable elapsed pair. These cohorts are independent and are not multiplied together.

All20 diagnostic allocation-request, requested-byte, reallocation and incremental usable-peak medians are exactly equal. Natural32 remains9,224 requests /12,057,256 requestedB /10,013,032 incremental usable-peakB; periodic932 /10,262,803 /9,972,104; eight498 /9,091,628 /9,193,416. Whole-process/RSS/physical and setup-retained heap differ from incremental peak and remain in raw receipts. Capacity is not initialized copying; the copy counter covers concrete COW helper copies only. Physical output backing reuse and zero input copying are not established.

Generated text including phases grows **7,534,880→7,668,288B**; root stack83,744→84,736B and root-plus-maximum-phase static envelope84,048→85,120B. These exclude allocator/host stack and RSS. Original/decoded image generation88.04s/88.15s are separate build observations, not matched compile-time claims. Timed Object SHA-256 `43e9f4fc49e481f3d21eb51b0e2742dd7ed71bfa1c77b2f86e86c3d17167786f`; worker `02895f38a00fb789dcaffe7b5fe06735141e547e79eaee803dce4e7035f87059`.

## Real execution and fresh executable profiles

A separate observer image executes40 complete correct streams. Seven existing complete proofs have disjoint source-site sets. Natural32 reaches1279 fast27,063 times and2624 twice; periodic1279 fast128 times and2624 fast32,854 times; repeated1279 fast127 times and2624 fast32,636 times. Those new actual entries have no fallback. The observer contributes no primary timing or instruction claim.

9,296 fresh wall-profile calls and22,297 exact-PC calls use the exact timed default worker, not the observer image. Timer weights are disjoint CPU-time samples, not instruction shares or dynamic visit counts.

| Input | Generated exclusive | Host below native | Other worker/system |
|---|---:|---:|---:|
| 33027-periodic-window | 97.814% | 1.487% | 0.700% |
| eight-literals | 82.140% | 11.878% | 5.983% |
| repeated-32769 | 97.901% | 1.268% | 0.831% |
| reymont-prefix-32769 | 98.428% | 1.266% | 0.306% |

The matching word loop now has direct two-word loads, XOR/rbit/clz mismatch extraction and register-held induction in the actual current Object. Fresh inspection of the exact periodic reference executable confirms its compact register-held word loop. Current addresses are derived from each actual executable; historical reference addresses are not reused. The C/Rust analogous checked-origin/fill Objects and10/48 controls are unchanged historical R207 evidence, not fresh R209 executions or complete compressor comparisons.

The outer root still accounts for1,839/2,287 periodic and1,840/2,287 repeated wall weights (about80%), distinct from the faster outlined matching phase. Hot current root instructions include the sliding-window U16 compare/subtract32768/narrow, then per-element checked collection length/capacity/owner branches and deep stack reloads. The tree-rebase attribution is inferred from the pinned source and matching scalar instruction pattern, without a source-PC map. This identifies a concrete collection-builder target; it does not prove that all root time belongs to tree rebasing. Natural text also retains a significant path-selection reduction/finish cost. Eight-byte root code still shows adjacent owner increment/decrement work.

## Qualification and disposition

- **320 new controls /eight forms** compare R208/R209, original/decoded, source-created private and saved-alias variants. Current U16/U32 contents, exact capacities (including262144/1500513 reservations), recursive child observations, parent rebasing, zero trips, selected assertions/division/overflow/bounds order, partially changed private buffers, unchanged previous published backing/output lanes, cleanup and success→failure→retry agree. There are224 resident value agreements,72 resident error agreements and24 unchanged default16Mi resource denials. No allowance is increased.
- Actual unique fast entries and live-alias fallback execute. The source-shaped no-saved-alias fixture itself has one initial shared fallback before first change; this is preserved rather than weakening exclusivity to meet an invented hit pattern. Main compressor entries above take the fast route.
- Fresh192 exact-origin cases,48 phase cases,1,724 actual matching cases and40 original/decoded builder cases pass. Existing48 phase/1,724 matching outcomes match R208 exactly, including1,566 resident values /142 unchanged default-cap denials /16 ordered source errors. Standalone packed-read and no-pack fallback controls remain separate.
- Eighty original/decoded whole-stream smokes and forty separate observed smokes pass exact bytes, independent inflate, input/prepared-output aliases, capacities and zero nonreturned private cleanup. All thirteen original/decoded Object/metadata pairs are byte-identical.
- Candidate library/linked consumers build at opt3, nightly2026-03-03, Cranelift0.131.3 with unchanged R203 sparse-SSA frontend and the recorded aot/jit/canonical-native/program/runtime feature profile, with warnings-as-errors. All27 candidate files pass format; maintained module/native-build layout checks pass. No fresh claim for the historical R206677 native tests/260 selections,15 architecture contracts or132 workflow tests.

Six development/harness failures are preserved with their resolutions: the clean parent rejects the original direct-U16 arithmetic toy; its replacement uses the actual compressor widen/mask/narrow idiom. Inventory receipt/output collision is prevented. A mutation-inside-child toy correctly required funding and was replaced with the real operation placement. A mistakenly repeated observed build was terminated before final writes, preserving the first successful image. A zero-capacity test oracle was corrected to the existing64-byte initial append growth. The observer expectation was corrected to retain the observed initial shared fallback. Successful value controls were reused; successful primary and counter cohorts were not repeated. No compiler stream/value failure remains.

The same seven inherited native exclusions remain listed exactly in native-exclusions.json. Full dependency/all-feature rebuild, multi-MB timing, the separate ten-workload suite and another maintained production native-suite run remain skipped. Ordinary exact-head hosted CI is cancelled under the standing instruction, automated review remains enabled, and #973 stays draft.

## Decision and next bounded step

Preserve this diagnostic candidate and all adverse evidence externally; do not adopt it as production. Exact resource accounting, precise canonical error-prefix/recovery, canonical ownership/publication and complete external admission/import remain unimplemented here. R205 direct-return and R206 visit-accounting production tradeoffs remain rejected; the full brief is not complete. Production host-operation exclusive-family and complete initialized-copy ledgers remain open.

The next bounded experiment should keep one typed numeric collection builder live across the pinned rebase-node(nodes) broadcast, using existing same-Plan scalar semantics, checked entry geometry, current private ownership, real allocation/growth, initialized output prefix and cold failures. Establish a minimal reproducer and alias/error acceptance test first, then repeat all20 timings/counters/profiles. Do not alter the compressor source or infer a speedup from this profile alone.

## Reproduction

Exact build commands, feature flags, compiler/source/artifact hashes, wrappers and all raw receipts accompany the external packet. Principal commands are measure-three.py, measure-counter-cohorts.py, qualify-fixed.py, qualify-decoded.py, qualify-origins.py, qualify-phases.py, qualify-matches.py, qualify-builders.py and profile-current.py. From an extracted packet, heap-summary.py then analyze.py reproduce numerical outputs without compiling or executing a compressor. Native reproduction requires the pinned toolchain/dependencies in build specs. No native binaries, bytecode, executable descriptors or raw trace stores are uploaded.

[Download the full source patch, four-file delta, all20 observations, controls, current assembly and numerical replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r209-fixed-nested-phases-5ea730e968ed.tar.gz) (2623051 bytes;908 files). SHA-256 `5ea730e968ed56c6f51b73d81600010ad228963b35c5cb6b348ae4b14d238ab6`. Server digests, every file, fresh download and byte-identical numerical replay verify.
