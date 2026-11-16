# Cold source-error origins in complete native computation (R208)

The current diagnostic is retained as an experiment. Sixteen of twenty elapsed medians fall, three rise and one is equal; **the geometric change is −3.039385%** with each case weighted equally. All 20 unchanged 1.05 gates pass against R207. Production remains **R199 / IR/work/ABI/MBC 214/107/39/14**; the diagnostic still fails six production gates and omits ordinary contracts. No production speedup or parity is claimed.

## Exact checkpoint and mechanism

Base `03cea977b903fb8d32a1fbf035196dc19e4d77f5`; retained production `7748204bd12e57a20369ce7970f31f8d15c3901f`; PR base `92f75db84e2523693648084bea16cd52ad60777a`. Compressor source SHA-256 `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`; bytecode `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`. Reference package `671077457ff7ef3a3d44caf715a3fc595addf379` and compiler `d99c7b838fb05bb1047eb0c18d4d06d0f86f345d` remain pinned. Only the existing draft #973 is used. The archived complete prototype patch changes 27 compiler files from this base; this round changes two files from R207. No compiler source changes after the primary cohort.

The root previously updated one shared source-origin SSA variable before each source operation. ARM64 hot code wrote that location to stack slots even when checks succeeded. The variable is now absent. Each reached source operation has a cold error target carrying its status, which returns that operation's constant location. An outlined numeric failure returns the callee's exact dynamic location directly. Header cleanup failures retain the current header location. Successful execution returns status/location zero as before.

In this closed diagnostic, the origin has no observer between source operations: concrete heap helpers carry status but do not read it; the only origin observation is the final error outcome. Final success stores and host private cleanup remain unchanged. This justifies moving location materialization without reordering predicates, loads, arithmetic, ownership effects or publication. The separate ordinary runtime's accounting/debug/recovery contracts are not changed or assumed away.

Existing phases, packed reads and allocation behavior remain. Both images report the same 58,217 static call instructions; callback count is not the claimed mechanism. Algorithm, source tables, depth 300/nice 258, original EOF/search rules, 262144-U16 tree, 1500513-U32 cache, 305257-U64 path capacity, source allocation sites and eight-byte output capacity 13 are pinned.

## Matched complete-call elapsed

8,496 primary timing observations /8,616 complete Mech calls, twelve balanced permutation blocks per case, plus 5,120 separate verified reference compressions. R207 is the previous diagnostic. Primary uses the unchanged mimalloc 2.1.7 and complete-call boundary: real allocation/copying/output transfer/private cleanup inside; preparation/clear/prefix/import/rebind/output slots/validation/inflate/drop outside. No favorable unchanged rerun replaces these observations.

| Input | Retained / Roc | R207 diagnostic / Roc | R208 diagnostic / Roc | Diagnostic elapsed change |
|---|---:|---:|---:|---:|
| reymont-prefix-32769 | 3.393281× | 7.057284× | 6.995653× | -0.873% |
| 33027-periodic-window | 3.513269× | 12.498903× | 12.300961× | -1.584% |
| repeated-32769 | 3.021483× | 11.721204× | 11.469044× | -2.151% |
| random-32769 | 4.907045× | 19.163725× | 19.526598× | +1.894% |
| 16384-distribution-change | 6.692914× | 15.360961× | 15.544029× | +1.192% |
| eight-literals | 4.616164× | 2.265164× | 2.235719× | -1.300% |
| reymont-prefix-16384 | 4.121602× | 6.891639× | 6.836152× | -0.805% |
| empty | — | — | — | -16.400% |
| one-stored | — | — | — | +0.000% |
| seven-stored-prefix | — | — | — | -25.150% |
| eight-overlap | 4.349281× | 2.152803× | 2.117672× | -1.632% |
| 320-overlap | 4.125277× | 2.463317× | 2.418259× | -1.829% |
| eight-literals-prefix | 4.519302× | 2.237108× | 2.188885× | -2.156% |
| 128-biased-dynamic | 3.819395× | 2.071575× | 2.063662× | -0.382% |
| 32-high-entropy-stored | 4.149123× | 2.475949× | 2.450219× | -1.039% |
| random-64 | 4.108589× | 2.621575× | 2.586902× | -1.323% |
| random-256 | 4.206644× | 3.384642× | 3.343961× | -1.202% |
| random-1024 | 3.890315× | 3.908320× | 3.933430× | +0.642% |
| repeated-1024 | 4.000698× | 3.215636× | 3.162982× | -1.637% |
| reymont-prefix-1024 | 4.599586× | 4.221071× | 4.200175× | -0.495% |

Only 14/20 unchanged gates pass against production. The empty/one/seven-byte reference clock rounds to zero, so no elapsed multiples are computed for them. The table preserves the adverse random32, distribution-change and random1024 observations. Current retained readings are fresh measurements of unchanged code, not compounded changes between rounds.

## Separate instructions, cycles and memory

The CPU/heap cohorts contain 2,400 complete Mech calls. Fresh same-input reference cohorts contain 1,280 counter observations /10,240 verified compressions, reusing the exact 40 R206 binaries after checking every current SHA. The original fourteen reference sources, compiler and allocator remain pinned. Whole-window counters include probe overhead without subtraction. CPU-probe allocation zeros are uninstrumented; allocator results come from the separate memory runs.

| Input | R207 instructions | R208 instructions | Instruction change | R208 instructions / Roc | Cycle change vs R207 |
|---|---:|---:|---:|---:|---:|
| reymont-prefix-32769 | 354,748,530.5 | 344,626,472.0 | -2.853% | 7.242979× | -0.835% |
| 33027-periodic-window | 266,102,816.0 | 252,051,665.5 | -5.280% | 13.083433× | -1.600% |
| repeated-32769 | 267,199,958.0 | 251,934,507.0 | -5.713% | 13.175315× | -2.310% |
| random-32769 | 223,415,309.5 | 227,285,490.0 | +1.732% | 19.338866× | +2.033% |
| 16384-distribution-change | 133,082,045.5 | 137,189,479.5 | +3.086% | 14.643849× | +1.542% |
| eight-literals | 5,258,647.5 | 5,136,412.5 | -2.324% | 3.764672× | -1.730% |
| reymont-prefix-16384 | 167,727,690.0 | 164,486,587.0 | -1.932% | 6.627422× | -1.115% |

Seventeen instruction medians fall/three rise; fourteen cycle medians fall/six rise. All 20 diagnostic allocation-request, requested-byte, reallocation and incremental usable-peak medians are exactly equal before/after. Natural32 stays 9,224 requests /12,057,256 requested B /10,013,032 incremental usable-peak B; eight literals stays 498 /9,091,628 /9,193,416. Complete per-case CPU, heap and process-memory observations are archived.

Elapsed, instruction, cycle, heap and profile cohorts are separate. Timer weights are not an instruction partition. Requested capacity, initialized copying, payload/usable heap, RSS and lifetime physical peaks are distinct. Diagnostic initialized-copy counters cover COW helpers only, not every memcpy/memmove or initialization. Capacity and live-alias checks do not establish physical output backing reuse or zero input copying.

## Actual generated code and current profiles

Generated text including phases falls **8,906,516→7,534,880 B** (−1,371,636 B). Root stack rises **79,696→83,744 B**; with the largest 304-byte phase the static envelope is **80,000→84,048 B**. Production already reports total helpers: 8,152,436 B /16,480 B. These static metadata are not process RSS or the allocator/host stack peak. The adverse stack increase is retained. The small source witness grows 644→760 B with unchanged 16-byte stack; whole-image text nevertheless falls.

Original image generation is 86.946916s versus R207's recorded 199.901847s; decoded generation is 87.085721s. These separate build observations are not a matched compile-time benchmark. Object SHA-256 `72631f01f3ab0bf19df7bf0fd148c7c36290d3f3c4bb8e1f551a0e473fa905bf`; worker `1bd559fe1fae45694bc5f790d21e4b596d6bdf24dab294f713598660a773f233`. All nine original/decoded image pairs have byte-identical Objects and metadata.

Four fresh wall-stack profiles execute 8,729 verified complete calls and four exact-PC profiles execute 20,937. The exact timed default executable is sampled; no observer-enabled image supplies timings or instruction claims.

| Input | Native exclusive timer weights | Host below native | Other worker/system |
|---|---:|---:|---:|
| 33027-periodic-window | 98.863% | 0.744% | 0.394% |
| eight-literals | 81.675% | 11.606% | 6.719% |
| repeated-32769 | 98.817% | 0.657% | 0.526% |
| reymont-prefix-32769 | 98.865% | 0.830% | 0.306% |

Large-input root code remains dominant. New exact-PC symbols/addresses are used; no historical address map is applied. The hot natural root still performs owner increment/decrement around a pure copy; repeated/periodic still repeatedly load/check array geometry around packed reads. The current compiler no longer transports the root origin through a shared SSA home, but status setup, guards, owner work and general state transport still execute. This change does not eliminate all error handling overhead.

The exact pinned reference executable still contains compact register-held word matching with XOR/rbit/clz mismatch extraction. R207's actual optimized C/Rust Objects demonstrate cold checked-error origins; their ten checked-sum and 48 fill controls are historical R207 receipts, not fresh R208 executions. The source, pinned Object hashes and disassembly are retained as mechanism evidence, not a C/Rust compressor timing comparison.

## Qualification, commands and failures

- **192 focused executions** compare R207/R208 and original/decoded artifacts on the same 48 cases each, including 19 errors per form. Independent resident values/errors and exact native status/index/canonical origins agree. They cover both assertion-versus-load orders, two failing checks, overflow while computing a predicate, zero-trip/unselected invalid work, fresh late recursive visits, nested/repeated calls, previous output unchanged on failure, input aliases, cleanup and successful retry.
- **48 phase cases** and **1,724 actual matching controls** freshly pass, with exact R207 outcomes including source origins. There are 1,566 resident value agreements, 142 unchanged default-16Mi resident-cap denials and 16 ordered matching source errors. The before standalone fixture has six pack sites; the after fixture has zero and explicitly exercises fallback. No cap or expected value increases.
- **40 original/decoded builder cases**, four additional U8 shared-rejection/sole-transfer controls, and **80 whole original/decoded smoke calls** pass. Real path/output capacities, reserve/growth/append/clear/sublist/indexed replacement/append-LE, aliases, publication, cleanup and retry remain exercised.
- Candidate library/linked consumers build at opt3 with nightly 2026-03-03, Cranelift 0.131.3, aot/jit/canonical-native/program/runtime features and warnings-as-errors. The isolated R203 sparse-SSA frontend and all frozen dependencies/allocator are unchanged. All 27 candidate files pass formatting; maintained module/native-build layout checks pass. Exact commands and source/artifact hashes are in the receipts.

The principal executed commands are `python3 measure-three.py`, `python3 cohorts.py`, `python3 measure-reference-counters.py`, `python3 qualify-origins.py`, `python3 qualify-decoded.py`, `python3 qualify-phases.py`, `python3 qualify-matches.py`, `python3 qualify-builders.py`, and `python3 profile-current.py`, through the command-recording wrappers. All final required collections succeed. No new production native-suite claim is made: R206's 677 native tests/all 260 CI selections, 15 architecture contracts and 132 workflow tests are historical. Production source outside retained tests remains R199.

Three introduced harness staging failures are preserved and resolved: the reference index was not staged; its fresh output directories were absent; and the matching helper expected before-fixture receipts from R207. One empty reference setup run (256 compressions) completed before an output-write failure and is excluded from every measurement claim. Successful Mech CPU/heap and primary cohorts were not repeated. Successful phase tests were reused when resuming matching/builder qualification. The original failing wrappers remain with their exact receipts; the recovery records identify final successes. No compiler stream/value failure or resource denial was introduced.

The same seven inherited native exclusions remain in `native-exclusions.json` and the R207 report. Full dependency/all-feature rebuilding, multi-MB timing and the separate ten-workload suite remain skipped. Ordinary exact-head hosted CI is cancelled under the standing instruction, while automated review stays enabled and #973 remains draft.

## Decision and next bounded step

Retain this small diagnostic improvement and every contrary observation. The ordinary compiler remains unchanged; exact resource accounting, precise canonical error-prefix/recovery, current canonical ownership/publication and complete external admission/import are still omitted here. An exclusive production host-operation-family ledger and complete initialized-copy census remain open. This is not ordinary NoLimits or a measured cost-of-contract floor.

The new audit identifies a more concentrated next target. The existing six-loop region at header2624 contains the actual packed matching sites2829/2895. Its broader parent837 has 52 cold/child exits and is declined, yet `closed.sites[2624]` still names that parent, so the diagnostic skips the child. Independently, the child writes a fixed U16 tree with no append-capacity field; the diagnostic currently rejects any writable array without that field. The existing proof has no cold exits or checked children, but this is a static opportunity, not a completed runtime boundary proof.

The next experiment should qualify that existing nested computation using the shared emitter, typed current entry/liveouts and real unique-owner/fixed-buffer checks. Start with a minimal parent-decline/fixed-tree witness, preserve live aliases and first-change behavior, prove nonoverlapping phase selection, then repeat the same 20-case matrix. Do not remove either refusal gate without proving its obligations. Scalar-result support alone does not unblock these Frame-result matching sites. Measured restoration of ordinary contract layers remains required before production adoption.

## Immutable review packet

[Download exact source, the two-file R207 delta, original/decoded controls, all20 measurements, current assembly and numerical replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r208-cold-source-origins-e8bfdc9c407f.tar.gz) (2,563,638 bytes; 759 files). SHA-256 `e8bfdc9c407fac154fdeabb5d7a1a70234edad10250288b75ce2620b6c88399f`. Server digests, every file and a fresh public release download verify. Both numerical outputs replay byte-for-byte; no native binaries, bytecode, executable descriptors or raw trace stores are included.

From an extracted packet, `python3 heap-summary.py` and `python3 analyze.py` reproduce numerical outputs without compiling or running a compressor. Native reproduction requires the pinned toolchain/dependencies in build specs. No native binaries, Objects, bytecode, executable descriptors or raw trace stores are uploaded.
