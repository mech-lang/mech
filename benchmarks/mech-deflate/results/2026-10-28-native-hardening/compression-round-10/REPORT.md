# Round 10: reuse exactly aligned managed-projection keys

Base: `648aa8b149978ab703595d6d6b84e58eb632fdd5`.
Native-loop baseline only: `0a0dae3b5a3ffc319ce3d948063177194794c45f`.
Key protocol/adversarial image test: `618d60eaa99802b842618d27f258ac3bc1958ec5`.
Implementation: `89a48f961a0905a83d81c3acd97a9e93fa40b691`.
Compiler comparison before: unchanged production compiler `4af4662097dd570f0821fc105a1c4d9de4eb5d89`.
Stack base: `eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`.

This patch reuses an existing prepared slot key at a managed frame projection.
It does not add a type cache, alter NativeProgramType or weaken definition checks.
Preparation and independent restoration validate count, exact slot index/schema,
complete definition and authoritative SchemaEntry key. The hot path compares the
actual Value against that retained slot and keeps fresh frame owners and exact
kind checks. It no longer canonical-encodes or hashes a definition per projection.
The original key API and Engine definition-corruption tests remain unchanged.
Physical frame transport, admission and logical work are unchanged.

The focused new test fails on the retained pre-change libraries: swapping two
individually valid schema/key slots is accepted by old restoration. This is an
inherited descriptor-consistency gap. The optimized path requires stronger slot
alignment, so the shared validation establishes that missing relationship.
Wrong keys and changed definitions are tested alongside slot permutations.
No restored corrupt descriptor is executed.

The stage-2 numeric recurrence regression is recorded but remains intentionally
unimplemented at this checkpoint. Its generic baseline separately demonstrates
57 ordinary runtime visits, 83 requests and 2440 requested bytes per element.
The original 0/1/8/64/256 diagnostic uses a separately declared 1T finite work
profile, unchanged existing component allowances, and an independent reduction
oracle beyond the resident default allowance. That authoring/default-limit
failure is retained in its own stage-2 evidence, not attributed to this patch.
The unvalidated stage-2 implementation is preserved separately and excluded
from this build and timing. It supplies no native selection claim here.

The optional table-source variant remains separate; this comparison uses the
pinned default source and artifact, all 25 components, real tree/cache, search
limits, passes, six Huffman initializers, allocation sites, reserve13 output
reuse and unchanged evaluate_into boundary. No larger complete input is run.

## Paired eight-byte complete compression

Apple M1, level 12, **Roc = 1×**:

| Subject | Median seconds | Multiple of fresh reference |
| --- | ---: | ---: |
| Reference, 20000 observations | 0.000289 | 1× |
| Retained before compiler | 0.129673750 | 448.70× |
| Key reuse compiler | 0.117005125 | 404.86× |

The patch uses **9.77% less elapsed time** (1.1083× the paired baseline's speed).
This is a small isolated improvement, not the larger native-region result.
The prior compiler and optional source-variant denominators remain their own
cohorts; no cumulative experiment is synthesized. Every primary call checks
exact raw bytes, independent inflation, immutable aliases, capacity 13 and zero
terminal managed bytes/nodes. Logical work remains exactly **875610515844**.
There are three retained calls per subject, excluded warmup/extra calls, and a
separate two-second diagnostic profile. No compiler or qualification overlaps
primary/protected scalar timing. No larger complete input is run.

Before executable SHA-256:
`0fa38254f8f5773f49749201a07ad2032356d38b5d7f47a44f4c581a6bdc6570`.
After executable SHA-256:
`30a5ab3e35fc9a4d68902ab73a6686a28e9b985bdbb0030dbabeec04c1022625`.
Source SHA-256: `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`.
Artifact SHA-256: `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`.
The timing harness, pinned mimalloc 2.1.7 object, selected catalog, Diagnostic
NoLimits mode and evaluate_into-only boundary are unchanged.

## Qualification, controls and failures

**41 scoped Rust tests pass:** 30 frame/argument/shared/state/closed-tail/cost/
assembly/weighted-diagnostic tests, eight native scalar tests and three bits tests.
Original/decoded and restored images retain exact type checks. Twelve corruption
observations cover swapped valid slots, wrong keys and modified definitions.
Existing finite denial, unchanged publication, aliases, cleanup and retry pass.
The weighted diagnostic's fixed 256-element inputs have 245/252/301/693/2037
requests for 0/1/8/64/256 steps, versus 283/366/947/5595/21531 before. Per-element
requests fall **83 → 7**, requested bytes **2440 → 128**, while genuine ordinary
visits remain **36 + 57×steps**. These are System-allocator diagnostics, not the
mimalloc complete-program timing or hardware traffic measurements.

The protected scalar cohort is **not fully green**. Seven of eight direct
original/decoded Bool/U64/I64/F64 timing gates pass, but decoded U64 is
**1.0526036×**, exceeding the unchanged 1.05 gate. All direct allocation requests
and bytes match exactly. Scalar-loop 0/64-step allocations have zero slope and
its timings pass. All rows are retained; the failed unchanged cohort is not
rerun for acceptance. Its cause is unresolved; it is a newly observed timing-gate
failure, not an established source regression or a proven environmental failure.
The pure-scalar execution source is unchanged, which does not erase the failed
measurement. No blanket scalar-latency acceptance is claimed for this patch.

The selected Cargo check passes in 33.27 s. Release succeeds in 452.14 s (exact
receipt retained), rebuilding Engine/compute/GPU/stdlib after restoring the
separate WIP files; all machine crates are reused. Engine tracked source is
identical to the prior compiler, but its rebuilt library bytes differ; actual
hashes are retained instead of assuming filename identity. Twelve static
routing/catalog/layout/factory/warning/planning/inventory/unsafe/format checks
pass in 195.44 s. Warnings remain errors and no unsafe allowlist changes occur.
An introduced diagnostic-script Path-expression typo fails before test
compilation; the corrected script's 30-test receipt is separate. The original
swapped-slot rejection failure is inherited and fixed.

The stage-2 native-selection regression remains pending and is deliberately
outside this focused key-patch cohort. Other filtered tests, linked subprocess,
generated runtime-only applications, Engine unit suites and larger/full-file
cases are not refreshed; the inherited 109-test receipt retains its own revision.
The selected catalog deliberately omits the all-primitives not/xor case.
CI is cancelled by request, the PR stays draft and parity remains unmet.

## Next assigned capability

Resume the preserved read-only numeric leaf implementation: prove invariant
array fields across all recursive edges; retain current owners and typed views;
keep scalar state native; support an ordinary suspended parent; synchronize
current state at finite fallback; independently rederive image proofs. Select
from canonical facts/original factories, never compressor names/sizes. Demonstrate
zero per-element metadata allocation and ordinary runtime transitions in the
funded loop, actual typed loads, the unchanged literal/extra true-cost recurrences,
and a generic changing-parent reduction. Private writes and growth stay separate.


Profile ancestry retains 1363 samples. The successful managed-projection branch directly reads the retained key; no Schema::key call remains in that branch. Inclusive counts overlap and are not additive time buckets. Full sampled stacks and analysis are retained.
