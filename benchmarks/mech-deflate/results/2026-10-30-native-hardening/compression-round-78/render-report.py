from pathlib import Path
import json,sys
s=json.loads(Path(sys.argv[1]).read_text());p=Path(sys.argv[2]);b=s['primary']['medians']['before'];a=s['primary']['medians']['after'];c=s['changes_percent'];code=s['callback_code'];binding=s['implementation'];static=s['static'];checks=len(s['validation']['checks'])
text=f"""# R78 — adjacent admission/helper transport

Retained with a small observed elapsed improvement and an explicit contrary instruction result. The complete eight-byte level-12 call moves **{b['multiple_of_reference']:.6f}× → {a['multiple_of_reference']:.6f}× Roc** ({c['primary_seconds']:.6f}%). Instructions rise **{c['instructions']:.6f}%**. This is a callback/code-size improvement, not a demonstrated instruction win or a claim of statistical significance.

The important new finding is in the actual host binary: the ordinary helper's own frame grows144→320 bytes and its unconditional entry/exit register-pair instructions grow4→12. Across8,836 helper callbacks, offset by3,597 removed admission callbacks, this family adds41,912 instructions. The separate whole-call hardware cohort rises41,211.5 instructions. This is a verified family calculation, not a complete function attribution or proof that other effects vanish. The independent instruction-window cohort is also adverse (+50,063.53125 mean instructions, +49,264.34375 inside entry). Preserve all three observations independently.

| Complete-call metric | Fresh reference | Accepted baseline | R78 | Change |
|---|---:|---:|---:|---:|
| Primary elapsed |0.290000ms |{b['seconds']*1000:.7f}ms /{b['multiple_of_reference']:.6f}× |{a['seconds']*1000:.7f}ms /{a['multiple_of_reference']:.6f}× |{c['primary_seconds']:.6f}% |
| Retired instructions, separate cohort |1,364,372 |40,126,037.5 /29.409895× |40,167,249 /29.440101× |+{c['instructions']:.6f}% |
| Cycles, separate cohort |910,563.5 |11,229,745.5 |11,239,286.5 |+{c['cycles']:.6f}% |
| Requests including reallocations |95 |6,791 |6,791 |unchanged |
| Requested bytes |9,021,923 |98,112,208 |98,112,208 |unchanged |
| Incremental allocator peak usable bytes |9,257,328 |20,482,072 |20,482,072 |unchanged |
| Generated compressor text |— |11,800,572B |11,753,624B |−46,948B |
| Generated entry stack |— |107,344B |107,280B |−64B |

Post-call RSS319,012,864→319,307,776B and physical footprint228,771,520→228,951,808B are separate observations. They include retained compiler/catalog/image storage and are not peak RSS or minimum standalone executable memory. Requested capacity is not copied bytes. Relocating reallocation copy upper bound remains8,571B; this is not a complete memcpy ledger.

Four independently prepared pairs contain3,968 primary timed calls and3,984 total observations. Three pairs improve; one is adverse (+0.165241%).38/64 blocks improve,26 are adverse. Median block ratio0.997351116 and geometric block ratio0.997893851 are descriptive; within-pair blocks are not independent experiments. The CPU-counter cohort's elapsed moves3.503729→3.504083ms (+0.010104%), also contrary and excluded from primary. No measurements are discarded or pooled across independent cohorts.

The exact mechanism count is807 combined managed moves plus2,790 combined structural observations. Admission callbacks fall12,450→8,853; helper callbacks remain8,836. Total crossings21,286→17,689 (−16.898431%). Logical admission attempts remain12,450 and shared compute debit attempts39,030. The original split count follows the preserved visit/helper sequence and retained baseline trace; R78 does not claim a fresh independently instrumented baseline census. All four new diagnostic calls agree exactly. Instrumentation is excluded from primary, hardware and allocation acceptance.

The compiler selects only ordinary structural actions and plain managed scalar-copy actions with no intervening specialized operation. Initializers, latches, counter guards, argument-lane transport, aggregate memory actions, numeric-segment entries and selected string/map/admission paths retain their original boundaries. Selection uses existing immutable metadata; it grants no execution permission. The combined callback runs the same complete visit and optional scalar admission-result handling before helper pointer/operand validation, then the same helper. No new ledger, invocation authority, cached funding or alternate algorithm is introduced.

Private program ABI38 distinguishes the new helper tags; old private ABI37 images remain rejected. Frozen scalar ABI1 is unchanged. Typed IR132, logical work policy95 and canonical bytecode14 remain. Independent source-backed build-image restoration, Object linking and original/decoded tests pass.

Qualification has **{checks} passing required receipt checks**. Selections overlap and are not summed as unique tests:

- Final actual GPU runtime331; work/memory/value32; new two-case selection and existing address/first-change/snapshot controls.
- Unchanged component236, connected8, path-native witness1, actual linked original/decoded recovery12, scalar controls12 at unchanged ceiling1.05, frame allocation/alias cases8.
- Final exact GPU CI inventory/execution37, Python workflow/inventory132, Engine projection12 and shape6 on the unchanged accepted Engine binary, ten architecture and five type/memory checks.
- Actual runtime-only GPU cfg compilation with warnings-as-errors against pinned feature-rich dependencies. This is not a freshly built minimal dependency graph. Global formatting/whitespace and exact unsafe allowlist pass.
- Clean accepted parent executes the three pre-existing controls that initially needed observer/ABI migration; all three pass. Existing child counts/work/outputs remain unchanged. Raw initial failures and fixes are preserved in prior-failures.json.

The new recurrence witness carries an array, number and Boolean through three fresh recursive visits. It compares the production combined callbacks with the original two-call sequence, on original/decoded artifacts and original/restored images, for success→arithmetic failure→recovery, changing arguments, aliases and prior publications. Five injected cases compare bad result pointers, malformed structural words, zero relative funding before each malformed case, and invalid sites; charged prefixes and typed errors agree. Pointer handling stays in the existing audited runtime test owner, while the new test case file is safe. No architecture allowlist or warning policy is relaxed.

No remaining local failure. Initial introduced failures include a missing optional-layout guard before timing, test input/fixture/error-expectation mistakes, observer/ABI migration and test-owner placement. They are fixed and their failed receipts retained. Core371 and Budget108/planner22 coverage are inherited from the unchanged accepted source/artifacts, not fresh R78 test executions. The existing large complete-knucleotide exclusion, full hosted/all-feature CI and larger/full-scale compression timing are not claimed. Publication CI is cancelled by user instruction and is not green CI.

The complete computation keeps source/bytecode, algorithm, tables, depth300, nice258, near-EOF checks, allocation sites, tree262144U16/cache1500513U32/path305257U64 capacities, input aliases, pinned mimalloc, output capacity13 with clear/reuse and evaluate_into timing. Raw output is[75,76,74,78,73,77,75,207,0,0], independently inflated. Logical work60,260,219,686; comparison47,612; final managed bytes/nodes0. Only the changed GPU crate rebuilds; unchanged Engine/Core/math/syntax/stdlib/runtime and external artifacts are hash-pinned. All before/after allocation requests are stack-censused and reconcile to independent heap counters.

PR base:eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b.
Round base:0816dc17aee0f009fa63577a37ebc30bc8693c36.
Production change:453bb17592bcc843c367cc70312496bda0d940a4.
Actual baseline release:{binding['baseline_build_source']}.
Actual candidate release:{binding['candidate_build_source']}.
Observation-only migration:d176bbd4be32ab1c500f6eda907038c88dfc30f4.
Audited test placement/final qualification source:84f180ca2bdadf139eb570912d8c01aedb08c971.
The qualification-only commits leave measured production identical; exact source proof is archived. The final publication head and user-directed CI cancellation are recorded in the PR.

Use reconstruct.py with a fresh output directory to verify every archived version and reproduce the derived reports without a compiler, compressor or external scratch files. Exact actual commands, cfg features, environments, raw outputs, manifests and before/after code excerpts are retained. The new candidate Object/image are archived as compiled-artifact.

Next bounded target: keep the ordinary helper out of the larger admission frame, preserving the same visit/result/error order. Verify emitted prologues and exact eligible counts, then rerun the same small complete-call timing/instruction cohorts. Ownership-plan transfer already retained in R70 and the rejected context/debit experiments are not reopened as missing work.
"""
p.write_text(text)
