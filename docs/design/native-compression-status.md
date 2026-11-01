# Native compression: current status

Raw observations, profiles, historical scripts and failed attempts are archived
outside the source tree. Source keeps this status, a maintained harness and the
[compact hash-addressed evidence index](../../benchmarks/mech-deflate/EXPERIMENTS.md).

## Current measured checkpoint: R90

The complete pinned eight-byte level-12 call is **9.296402× Roc** in the current
matched cohort. This round fixes an inherited bounds-error bug; it is **0.319946%
slower** than its freshly measured R89 baseline. No performance win is claimed.
The target remains1×; larger-input throughput is unestablished.

| Metric | R89, freshly measured | R90 |
|---|---:|---:|
| Elapsed |2.6502915ms|2.658771ms (+0.319946%)|
| Elapsed / fresh0.286ms reference |9.266753×|9.296402×|
| Instructions, separate32-call cohort |29,288,870|29,290,833.5 (+0.006704%)|
| Cycles, same separate cohort |8,410,194.5|8,476,981.5 (+0.794120%)|
| Requests including reallocations |6,399|6,399|
| Requested bytes |98,074,788|98,074,788|
| Incremental peak live allocator bytes |17,851,960|17,851,960|
| Generated text / entry stack |15,032,784 /134,816B|unchanged|

Four prepared pairs contain3,968 primary/3,984 total observations. One pair and
24 descriptive blocks improve; three pairs and40 blocks are adverse. Opposing
geometric block results remain preserved; no significance claim. The first
correct implementation was also adverse (+0.502134%) in a separate cohort.
Hardware, allocation and sampling are separate cohorts, never multiplied.
Whole-executable RSS/physical footprint are adverse by409,600/327,616B.
Requested capacity is not initialized bytes copied or live memory.

R89's earlier retained optimization improved its own matched elapsed by5.572531%,
instructions by4.086419% and incremental allocator peak by12.805932%. Its9.080397×
ratio used a0.291ms reference. Comparing that ratio directly to this round's
0.286ms reference would overstate the source regression.

Base/prior published head: `7ef95a8a22977d2436e2ad207dae3bcba7e916a2`.
Retained/measured compiler: **`254ac339b64ab71dd374cc20c3873cd6ae766921`**.
First correct variant: `3444655203c554824153771782d3b81bf3cc1edc`.
Baseline compiler: `460c9cb748fc377c1cbcfd57565a5373de05de14`.
Regression-first: `6597015d8`; separate CI inventory: `a1cf08ea2`.
Stack base: `eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`.
[PR #973](https://github.com/mech-lang/mech/pull/973) remains draft until green CI
and clean review. Publication CI is cancelled by instruction, not counted as green.

## Correctness repair

The new changing-buffer/recursive-child witness reproduced an ordinary checked
indexed update followed by a scalar admission group. Its empty read exposed an
inherited bug: the ordinary reference returned Index at its original source
origin, while the optimized private read returned Value(StaleHandle).

The retry now retains its proved current operand before ordered retirement,
installs it using the existing funded alias escape, and calls the original
reader. Cold retry handling is outlined; normal and cold paths share the same
retirement loop. No fabricated error, permission, ledger, allowance or pricing
change is introduced. The compressor Object, descriptor and metadata remain
byte-identical. IR/work/private ABI/MBC remain **136/97/38/14**, scalar ABI1.
Logical work remains42,869,883,099; it is not a hardware instruction count.

Two regressions compare native and ordinary ScalarLoops-disabled execution for
original/decoded artifacts and original/restored descriptors:48 scenarios cover
changing arguments, empty reads, overflow, recursion, prior publication, aliases,
cleanup and retry. Six finite resource dimensions keep the same minima. Linked/AOT
adds12 original/decoded cases including four failures and recovery.

## Validation and remaining work

Fresh overlapping passing selections: runtime347;work/memory33;native CI53;image1;
unchanged components236;connected8;path1;existing linked16;new linked12;scalar12
(maximum1.025267 against unchanged1.05, equal pure-scalar allocations, zero loop
allocation slope);frame8;workflow132;architecture/type/memory15. GPU-only release,
optimized units and runtime-only config compile with warnings-as-errors against
the existing coherent selected-operation dependency graph. Core/Engine coverage
is inherited; no fresh full math or minimal/no_std build is claimed.

Weak initial witnesses, accidental patch context, a wrong import, a numerical
replay stdout/stderr mistake and the network-denied evidence upload are preserved
and corrected. The upload resumed the exact tested packet. No required local
failure remains for the retained R90 source. Hosted green CI, giant complete
knucleotide, larger/full-corpus timing and fresh minimal/no_std coverage remain
skipped. Failed/adverse observations remain independently retrievable. Fresh retrieval verifies
46 packets/2,017 file versions/544 paths, including32 multi-version paths, and
reproduces all four numerical reports byte-identically. Object-binary upload was
rejected by automatic approval review; that binary remains local. Numerical replay
checks its recorded hashes against both build bindings and retrieves the actual
descriptor/metadata bytes; it does not claim independent Object-binary retrieval.

The next bounded optimization is the proved scheduling conflict:97 otherwise
eligible parent pieces start in an ordinary scalar admission group, containing
264 currently executed library callbacks. Parent25's31-site entry2366 accounts
for136 of those callbacks. The maintained witness now reproduces the overlap.

Admit only a fully contained ordinary scalar prefix; retain its existing debit,
fund the larger piece through the same account, and restore scalar readiness on
decline. Preserve owner-bearing groups, partial overlaps, child boundaries,
source error order and finite recovery. More selected pieces are not acceptance:
measure actual helper reductions, instructions and matched elapsed. No speedup
is forecast from the candidate counts; debit-function tuning stays closed.

## Fidelity

The [component assembly](../../hosts/gpu/tests/fixtures/native/deflate_assembly.rs)
performs all compression as one compiled computation. Source SHA-256
`331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`;
MBC SHA-256
`a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`.
Algorithm, tables, source allocation sites, depth300, nice258, near-EOF checks,
tree262144 U16/cache1500513 U32/path305257 U64, input aliases, pinned allocator,
output capacity13 clear/rebind and evaluate_into-only timer remain unchanged.
Search and skip remain separate; skip has no cache/recording path. Exact bytes
`[75,76,74,78,73,77,75,207,0,0]` and independent inflation agree. Final managed
bytes/nodes are zero. Physical public-output backing still changes.
