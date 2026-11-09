# Scalar-first vector loop evaluation — R232

Qualify and archive this two-file compiler experiment, but keep R231 as the retained standalone baseline. The candidate removes a repeated eight-byte guard from successful wide iterations and compares one original scalar word first. It reduces retired instructions, yet pooled geometric block means rise **0.959%** across all 25 inputs. Natural32 improves 0.584%, while periodic/repeated rise 0.537%/0.715% and eight literals rise 2.540%. Both predeclared cohorts are adverse geometrically (+1.285%/+0.532%). The saved instructions are a real, retained candidate result; this is not a discarded small elapsed win.

## Complete-call measurements

| Case | Retained R231 / Roc | R232 candidate / Roc | Pooled block change | Instructions | Cycles |
|---|---:|---:|---:|---:|---:|
| reymont-prefix-32769 | 2.304793× | 2.291339× | -0.584% | -0.656% | -0.079% |
| 33027-periodic-window | 1.730550× | 1.739836× | +0.537% | -1.399% | -0.239% |
| repeated-32769 | 1.622129× | 1.633727× | +0.715% | -1.404% | +0.552% |
| random-32769 | 2.832036× | 2.825072× | -0.246% | -0.018% | -0.617% |
| 16384-distribution-change | 2.782063× | 2.755763× | -0.945% | -0.020% | -0.329% |
| eight-literals | 0.485619× | 0.499217× | +2.540% | -0.000% | -0.272% |
| reymont-prefix-16384 | 2.250221× | 2.236599× | -0.605% | -0.482% | -0.614% |
| empty | 1.976190× | 1.976190× | -0.019% | +0.000% | +1.347% |
| one-stored | 1.976190× | 1.976190× | +0.784% | +0.000% | -2.063% |
| seven-stored-prefix | 2.000000× | 2.000000× | +1.091% | +0.000% | -6.584% |
| eight-overlap | 0.478986× | 0.494681× | +4.291% | -0.003% | -1.286% |
| 320-overlap | 0.549936× | 0.561924× | +2.144% | -0.129% | -4.696% |
| eight-literals-prefix | 0.480950× | 0.499572× | +3.454% | -0.000% | -1.012% |
| 128-biased-dynamic | 0.493500× | 0.508643× | +3.296% | -0.004% | -6.860% |
| 32-high-entropy-stored | 0.518182× | 0.533949× | +2.573% | +0.002% | +0.731% |
| random-64 | 0.557559× | 0.569924× | +0.891% | +0.002% | -1.945% |
| random-256 | 0.661107× | 0.675686× | +1.781% | +0.003% | -0.845% |
| random-1024 | 0.753662× | 0.767431× | +1.458% | -0.000% | +0.929% |
| repeated-1024 | 0.666559× | 0.678460× | +1.292% | -0.559% | -0.296% |
| reymont-prefix-1024 | 0.970177× | 0.977415× | +1.405% | -0.143% | -2.859% |
| heldout-window-32767 | 2.312801× | 2.300350× | -0.538% | -0.657% | -0.141% |
| heldout-window-32768 | 2.313969× | 2.307379× | -0.285% | -0.659% | -0.446% |
| heldout-slides-65537 | 2.392044× | 2.385422× | -0.277% | -0.759% | -0.172% |
| heldout-random-65537 | 3.137025× | 3.129258× | -0.248% | -0.021% | -0.105% |
| heldout-mixed-131073 | 2.581231× | 2.573958× | -0.282% | -0.460% | -0.109% |

Fresh matched reference denominators apply to both sides in this table. R231's current-cohort 0.485619× eight-literal result versus its previous 0.499426× is measurement/reference movement with the same executable, not another compiler improvement. R232 remains an unadopted alternative, not the new default timing table.

The first cohort passes all 25 point gates but only 22 block-mean gates: eight-literals, eight-overlap and eight-literals-prefix fail the original 1.05 allowance. The second complete predeclared cohort and all 48 pooled balanced blocks pass all 25 point/block gates. Keep the first failures and both adverse geometric means; no third confirmation, changed gate or selected-input rescue is used.

324,444 complete primary calls / 324,144 retained observations, plus 3,900 separate CPU/heap observations and 50 immediate full-stream smokes. All 20 original inputs and five held-outs through 131,073 B retain exact reference bytes, independent inflation, input/output aliases, output prefix/capacity and zero final nonreturned private payload/owner records. Both sides use fixed level12/runtime input, held borrowed outputs, equal200ms warmup, balanced order, pinned mimalloc and the same Rust clock/validation library. Full compression/scratch construction/output transfer/cleanup are timed. Validation/preparation/logging and old output drop stay outside. Builds/tests/profiles do not overlap primary/counter collection.

## Compiler mechanism and proof

Only codegen/wide_equal.rs and its unit test differ from R231's archived compiler source. Keep the same current-window certificate and structural selection: exactly two immutable byte windows, one changing I64 induction phi, two packed eight-byte loads, pure XOR equality, exact step8 and no calls/stores/debits/extra carried state. There is no program name, source ID, field name or input size selector.

The original scalar header and first scalar comparison execute unchanged. Only the proved-equal backedge enters a new vector header carrying the current next position. Successful vector probes advance16 directly to that header. Short remainder or mismatch returns to the original scalar header with an explicit current-position argument. Original first-difference/ctz calculation, source errors/origins, scalar words and byte tails remain. A mismatch never reuses the old header's induction value.

Entry scalar equality proves next<=limit. Each wide iteration requires limit-at>=16, so subtraction and advance remain bounded. Existing fresh current-window admission proves both readable ranges, start<=limit and representable limit+8; unknown/invalid geometry keeps the original checked path. Bases/limit dominate the private loop, arrays stay immutable, and no permission/pointer survives an invocation or backing change. Vector high/low-bit branch behavior remains R231's direct U64 OR. No source Bool representation changes. Negative store/stride/extra-state/endianness/predicate/missing-scope controls decline.

Exactly three existing numeric bodies change; the complete root and 133 other bodies remain byte-identical. Text changes 10,308→10,292 B in two bodies and 9,772→9,760 B in the third:44 B less in total. Two frames decrease400→384 B; the third stays304 B. Root text5,649,872 B and stack55,088 B stay equal. Full executable size remains6,623,288 B including padding. Heap request counts/bytes/reallocations/incremental usable peaks and all payload request/copy/zero/private-peak fields match exactly on all25 cases. No streaming, whole-worker RSS or allocation improvement is claimed.

## Qualification and failure disposition

686 maintained tests pass under the same six-feature -Dwarnings configuration and seven inherited exclusions. The expanded native unit domain covers1,057,536 distinct alignments/starts/limits/first-mismatch/high-or-low-bit cases, including boundaries around8/16/32/64/80 and start>limit scalar exits. Focused/full execution totals2,115,072 value checks. Unit fixture controls are not substituted for the source-level gate/overflow tests.

All5,788 source controls rerun across four programs, parent/candidate and original/decoded forms. All16 small images form eight byte-identical Object/metadata pairs. Exact values/status/source origins/shapes, aliases, failure publication, cleanup and retry match the independent source-operation oracle and pinned prior rows. Fallible and stride12 controls select no wide loop. Fresh focused resident calls remain0: unchanged R230 source/engine resident rows are explicitly pinned and reused, not recounted. The maintained suite is freshly rerun including its resident tests. Existing resident compute denials/allowances remain unchanged. Final managed bytes/nodes and private payload are zero.

Three experiment setup mistakes are retained and fixed: a missing copied client module, an old frozen-row filename and a command receipt overlapping the code-comparison data filename. They caused build/missing-file/data-shape rejection, not a compiler semantic mismatch. Initial failed receipts remain; the corrected client dependency is byte-identical and no compiler/source/allowance/measurement changes were needed. There are no introduced compiler build/test/semantic failures or new resource denials. Ordinary full-compressor timing, a second complete-image reemit, the historical packet/write census and multi-MB inputs are not claimed newly run.

Fresh candidate wall/current-PC profiles contain 25,160 / 102,758 correct calls, with actual executable/Object hashes and resolved current symbols. Samples are CPU-time attribution, not per-function retired instruction counts. Nineteen instruction medians decrease, three tie and three rise;21 cycle medians decrease/four rise. Periodic/repeated instructions fall1.399%/1.404%, but their elapsed/cycle cohorts do not establish an elapsed win. Preserve all independent cohorts.

## Pins and scope

- base: `14ae29af688a5372cc1a7fe5712f0efb77ae414b`
- experimental_parent: `14ae29af688a5372cc1a7fe5712f0efb77ae414b`
- normal_compiler: `42c95cfd79e0dc3773d03aeb9f553d32ff3922c5`
- compiler_source_manifest_sha256: `84b476792376bf1dc805eeb89ffd0913b5f1d4510efef62f990f2f7d548de466`
- compiler_patch_sha256: `0e3ba7f8931d5c9d97be34a1410842a3a28d8d03152f214c97e700db3c5dba2f`
- source_sha256: `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`
- bytecode_sha256: `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`
- object_sha256: `3453ec0f1538f218f5e163756b6ae6aa3ee583feddf19d14f4961d99b7675196`
- worker_sha256: `b09f31c0ab5bda2e217cffb61835e695dd00cfb24ebc07f4f7b4b605f21ba73d`
- reference_worker_sha256: `ed1ab9175cd175d26d453afc26ba1fa7adb4827ec705aa1cf9bcd231ac84860c`
- allocator_sha256: `38c6ec82a41e90a96bb9f1ae2c89058a77909da3fbb69c16e542ae5c858bf874`

Normal compiler remains R215 at42c95cfd79e0dc3773d03aeb9f553d32ff3922c5, IR/work/ABI/MBC215/107/39/14. The retained standalone compiler remains R231 at14ae29af688a5372cc1a7fe5712f0efb77ae414b. This experiment does not restore the additional ordinary exact resource/admission/accounting/publication/recovery contract, add a native operation class, modify source, invoke LLVM, combine R225 or create another branch/PR. Algorithms/tables/depth300/nice258/near-EOF behavior,262144 U16 tree,1500513 U32 cache,305257 U64 path capacity and output cap13 clear/reuse remain unchanged. Full parity stays open.

Next measure the dedicated vector loop without forcing the scalar first word. Keep the same current-window admission and exact scalar mismatch/tail state. Separate follow-up evidence identifies repeated guarded packed reads and four actual reference pstl1keep hints absent from this Object; no dynamic miss attribution or promised saving is claimed. See proof-notes.json, followup-read-reuse-audit.json and reference-prefetch-audit.json. No unrelated compiler change is combined here.

## Verified evidence

[Full candidate compiler source, minimal patch, both cohorts, counters/profiles and tests](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r232-peeled-vector-668b107c67f5.tar.gz). SHA-256 `668b107c67f545d173a739c6173c8dab60cd7c4cd2740a594ed99c14ad1eec38`; 6600887 bytes. All1,434 downloaded file hashes and numerical replay verify. Native binaries/Objects, MBC, full executable metadata/descriptors and raw traces/stack samples are excluded.
