# Hardening review packet

Reviewed baseline: `48911f96183db8ac804503a0c894b054d629382d`.
Architecture/cleanup parent: `eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`.
Last reviewed publication: `39442acb64ed47500ec26d5c4037cc37d0015ebe`.
The retained stack remains #934 → #935 → #939 → #973, draft and unmerged.

## Responsibility and implementation

| Finding | Commit / disposition |
| --- | --- |
| Packed completion could shrink/copy while claiming zero cloned bytes | `51c8f05afd77ffbab39863ed0d173b48d932896f`: transfer the Vec with capacity, price retained capacity and preserve failure cleanup |
| Definition-table estimate described as a total analysis-memory cap | `ff26c8dff763b0b5300f9199709b6ee7b46fd8fa`: precise per-table name, stage/required/allowed diagnostics; no false total-memory claim |
| Repeated old-value graph traversal | `de72b7dc8763acfcc01dfe9342befe661c0a08de`: shared mechanical query, distinct frame/consuming-call eligibility |
| Dimension relation duplication and harmless scalar-sibling rejection | `a04db530d09213a6a6ab9241d80b7fd49067e2d2`: schema-owned mappings and separate renaming/freshening/generalization policies |
| Repeated per-call selection probes and owner exclusions | `39442acb64ed47500ec26d5c4037cc37d0015ebe`: prepared route, shared owner inventory, independent image rederivation |
| Mixed frames materialize scalar Values and rebuild shape metadata | `2b4adb7289cb81ffd0c37456ce1f968c9ed3d43d`: typed lanes, managed-only owner arena, current-visit exact kinds and immutable shape recipes; `1a518f56c` strengthens negative recipe tests |
| Missing public finite-policy execution evidence | `c24984b402a1228cccf934a478965143b539f611`: finite public managed/hosted APIs and CLI, strict separate launch policy, shared pinned source and normal-main example |
| Image-restoration regression missing exact boundary inventory entry | `381fbde33bee12aab73bbddf704b3247ed52285c`: test-only provenance contract; full boundary audit passes |

The existing canonical artifact → engine proofs → shared preparation →
JIT/Object → runtime path remains authoritative. Restoration independently
rederives facts. No new executor, intrinsic, source-name/size recognition or
authority-bearing serialized proof is introduced. All 25 compressor components,
six deliberate Huffman initializers, algorithms, tables, search/EOF limits,
real buffers, fixture outputs and timing boundaries remain pinned.

## Fresh qualification

Pinned toolchain: nightly-2026-03-03; Cargo uses locked offline dependencies,
one build job, no incremental compilation, and warnings as errors. Exact
commands, environments, revisions/dirty inventories, binary hashes and complete
output accompany this report. Rust/Cargo implementation files have no difference
from c24984b40; later changes are receipts, documentation, case selection in the
Python checker, and the exact test-boundary inventory.

| Selection | Observed result |
| --- | --- |
| Canonical program: frames, collection completion, sequence capacity, proof storage, uniform words, explicit unrestricted-mode control | 49 passed, 0 failed, 0 ignored; 327 filtered out |
| Flat frame recipes, including corrected bounded/persistent/computed negative input arity | 2 engine tests passed; GPU filter selected 0 and is not counted |
| Independent route restoration and strict managed launch policy | 2 passed, 424 filtered out |
| Public CLI Cranelift build tests under default distribution + `cranelift_build` | 6 passed, 446 filtered out |
| Shared complete-source pin | 1 passed, 375 filtered out |
| Linked lexical assertion and mixed-record frame recovery | 2 passed, 374 filtered out; unchanged expected values and allowances |
| Public complete-program source/decoded Object generation | Both succeed on the normal process stack, byte-identical Objects/descriptors/identity |
| Actual linked runtime-only complete program | Eight-literal case passes in each form: exact bytes, independent inflation, zero final managed bytes/nodes |
| Runtime-only closure | Actual Cargo artifacts exclude parser, source compiler, bytecode producer, Cranelift and JIT |
| Architecture guards | Routing, catalogs, build-analysis layout, module layout, factory safety, warnings, planning quarantine, CI inventory pass; boundary omission fixed and full audit passes |
| Formatting / whitespace | Formatting and the hardening diff against 48911f96 pass; inherited whole-PR findings are listed below |

These selections execute **62 tests**, with zero failed or ignored selections.
The two public complete-program launches are additional process qualifications.

The 49-test selection covers fresh current arguments/shapes, exact kinds and
bits, aliases/self-aliasing, unused results, finite denials, prior publication,
failure cleanup and retry for original/decoded artifacts and resident/JIT paths.
The linked drivers additionally perform valid → assertion rejection → valid,
check error order and unchanged previous publication, and recover on the same
image/entry. They retain their existing expectations and allowances.

The entire 376-test program inventory was not rerun. The other eight deployed
small cases, final-head complete-program original JIT, block/window transitions,
repeated slides and full corpus are not fresh passes. Ignored long tests and
cancelled full-corpus runs remain explicit gaps. No long campaign restarted.

## Performance and provenance

The [predeclared frame comparison](../frame-comparison/primary/results.json)
passes its first balanced cohort: exclusive/aliased requested allocations and
bytes drop materially; paired median times are about 0.464–0.468 of before.
All Bool/U64/I64/F64 short controls and the scalar-loop allocation-slope control
pass their frozen gates. All 840 retained primary observations, warmups and
16 slope controls are preserved. This does not assign the full-compressor
improvement to frames.

The [small complete timing](../compression-round-01/REPORT.md) records **Roc = 1×**:
current matched-allocator uninstrumented median **5756×**, and a separate
matching-System/instrumentation compiler control **8659× → 6599×**. Compiler
setup, verification and output destruction are outside those timers. Debug
public-process qualification times are not benchmark comparisons. Raw samples,
failed builds, pinned allocator object, executable/library hashes, source pins
and post-change profiles are preserved in `af77b2840` and `3465e13cf`.

## Failed attempts and remaining gates

The introduced image-test inventory omission is fixed in 381fbde33; its original
failed audit is retained. An example-only Arc conversion, probe type inference,
hash formatting and old-library linkage setup errors were corrected or replaced
by retained exact executable controls; none is a runtime result. Initial
engine/CLI minimal-feature test commands lacked required existing source test
features. Supported profiles now execute the intended tests. The initial CLI
filename filter selected zero tests; the corrected module filter executes six.
Do not count the zero selection as qualification or claim a clean-parent
minimal-feature build that was not run.

The final staged whitespace check found trailing blank lines in 35 newly added
test logs. Their exact original bytes are now stored as gzip; the
`compressed-receipts.json` manifest preserves original paths, byte counts and
hashes. The first staged check was mistakenly followed by a receipt commit;
this packaging correction fixes the complete hardening diff, without editing
test output. A whole-PR check against eacb00fc still has **146 inherited
findings in 16 files**, reproduced directly at the clean reviewed 48911f96
baseline. They are historical logs/diff receipts and one comparison Cargo
manifest's trailing blank line. `inherited-whitespace.txt.gz` records them.
They are assigned to a separate historical-evidence packaging follow-up; this
pass does not claim the full consolidated PR is whitespace-clean.

The decoded U8-add `MissingSemanticWitness` reproducer remains a separate,
pre-runtime compiler limitation; it is not caused by packed completion or frame
execution. It is preserved in the earlier receipts and current capability
documentation. It is not needed by the pinned connected compressor path.

CI remains cancelled by user instruction, without a green-CI claim. New review
is requested after publication; the earlier clean review applies only to
39442acb64ed47500ec26d5c4037cc37d0015ebe. Draft status is preserved. The original
architecture/cleanup performance gates on parent #935/#939 are unchanged.

The next measured implementation target is generic indexed-update planning on
packed buffers, identified by the complete-program profile and real U16/U32
store witness. Preserve work charges, first-denied errors, aliases and recovery;
then repeat the same eight-byte timing and profile with no concurrent build.
Allocator changes are permitted and must be measured separately. A compressor
source change needs its own performance justification and fidelity review.
