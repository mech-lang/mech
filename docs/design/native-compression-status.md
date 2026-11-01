# Native compression: current status

Raw observations, profiles, historical scripts and failed attempts stay outside
the source tree. Source keeps this status, one maintained harness and the
[compact hash-addressed evidence index](../../benchmarks/mech-deflate/EXPERIMENTS.md).

## Current measured checkpoint: R91

Complete pinned eight-byte level-12 compression improves **9.324090× →9.044674×
Roc**, or **2.996705% less elapsed**, against the freshly measured R90 compiler.
The target remains1×; larger-input throughput is unestablished.

| Metric | R90, freshly measured | R91 |
|---|---:|---:|
| Elapsed |2.6480415ms|2.5686875ms (−2.996705%)|
| Elapsed / fresh0.284ms reference |9.324090×|9.044674×|
| Instructions, separate32-call cohort |29,290,846.5|28,237,589 (−3.595859%)|
| Cycles, same separate cohort |8,470,510.5|8,144,220.5 (−3.852070%)|
| Requests including reallocations |6,399|6,399|
| Requested bytes |98,074,788|98,074,884 (+96)|
| Incremental peak live allocator bytes |17,851,960|17,851,960|
| Generated text bytes |15,032,784|15,911,648 (+878,864)|
| Entry stack bytes |134,816|143,488 (+8,672)|

Four prepared pairs contain3,968 primary/3,984 total observations. All four pairs
and62 descriptive blocks improve; two adverse blocks remain. No significance
claim. Reference hardware medians are1,364,372 instructions/922,027 cycles;
allocator95 requests/9,021,923 requested bytes/9,257,328 incremental live bytes.
Elapsed, hardware, heap and sampling are separate cohorts, never multiplied.
R89's9.080397× used a different0.291ms reference clock; cross-round ratios alone
cannot establish a source regression or combined improvement.

Requests and162 reallocations are unchanged. A large request grows440,256→440,272;
source max_library_inputs grows60→61. The96-byte increase is not fully attributed.
Tree/cache/path histogram entries remain5×524,288,1×6,002,052 and35×2,442,056.
Requested capacity is not initialized bytes copied or live memory. Whole-executable
RSS/footprint decrease524,288/524,224B in the separate hardware cohort; this is not
a broad memory win. Code and stack growth are retained adverse observations.

Base: `c24b50e52a8988d7272012b64bf5cdb53adc0a6e`.
Measured/retained compiler: **`31d77c7c1f5fdeecc9095664af19092353df06b3`**.
Baseline compiler: `254ac339b64ab71dd374cc20c3873cd6ae766921`.
Separate CI inventory: `fb6989d87f9d1c57c6594c8b91bf75fa44e4bf51`.
Stack base: `eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`.
[PR #973](https://github.com/mech-lang/mech/pull/973) remains draft until green CI
and clean review. Publication CI is cancelled by instruction, not counted as green.

## What changed

An existing scalar admission group excluded97 otherwise proved larger parent
pieces. Selection now accepts only an ordinary scalar group with the same entry
and every member inside the larger piece. Owner-bearing groups, partial/interior
overlaps and all child lifecycle boundaries stay ordinary barriers.

The larger piece retains current schema/account/type/slot/owner and funding checks.
The already paid scalar debit remains. Refusal leaves its marker intact; generated
code restores the previous0/1 scalar readiness word overwritten by native entry.
Successful entry clears the marker only after all fallible preparation/funding.
Restored images independently rederive containment and selection. No new ledger,
refund, renewed permission or loan across a child is introduced.

IR/work/private ABI/MBC are **137/98/38/14**, scalar ABI1. Policy98 explicitly
retains the paid scalar-prefix debit before the existing native envelope. Logical
work42,869,883,099→43,123,410,789; this is not a hardware instruction count.
No allowance, output golden, unsafe allowlist, warning or architecture rule is
weakened. The Huffman policy assertion advances to98; its88,767,602 work golden
is unchanged.

Four identical complete diagnostic calls per subject show:

| Executed mechanism | Before | After |
|---|---:|---:|
| Source admission callbacks |6,034|5,633|
| All host helper callbacks |4,959|4,911|
| Ordinary library callbacks |792|528|
| Canonical arena installations |2,047|2,010|
| Successful native piece visits |803|938|

Static pieces387→484, with none lost. Forty of the97 added pieces execute135
visits. Parent25 entry2366 executes eight visits and removes136 library callbacks;
all new pieces remove264. No actual native entry declines in this complete-call
census. These are excluded diagnostics, not instruction or timing measurements.
Fresh serial profiles contain695 calls per subject and20,000 reference calls.
Entry-body sample weights208→178, native-entry preview35→51; the added boundaries
still cost work. Samples are not retired-instruction shares; ancestry overlaps.

## Qualification and evidence

Current-source overlapping passing selections: runtime350;work/memory33;native
CI59;image1;unchanged components236;connected8;path1;linked16+12;scalar12
(maximum1.011103 against unchanged1.05, equal pure-scalar allocations and zero loop
allocation slope);frame8;workflow132;architecture/type/memory15. GPU-only release,
optimized units and runtime-only config retain warnings-as-errors against the
coherent selected-operation Core/Engine graph. No fresh full math/minimal/no_std
rebuild is claimed. Hosted green CI, giant complete knucleotide and large/full-
corpus timing remain skipped.

New regressions cover current arguments on each recursive visit, actual refusal
after scalar prepayment, restoration of readiness, one-short unfunded fallback,
original arithmetic origin, unchanged prior publication, aliases, cleanup and
recovery in original/decoded/restored forms. R90's private-read retry correction
also remains: it pins the current operand before ordered retirement, installs it
using existing funded alias escape, and invokes the original reader. R90's
+0.319946% adverse elapsed and all its failed preparations remain separate.

Intended baseline failures, an over-specific fixture assertion, a test-only shape
slice inference error and an early unit-driver call are preserved and corrected.
No required current local failure remains. Fresh external retrieval verifies
26 packets/1,919 file versions/1,137 paths and reproduces four numerical reports
byte-identically. Raw Object binaries remain local following automatic approval
rejection of an upload. Replay checks their pinned hashes against build bindings;
it retrieves descriptor/metadata bytes, not the Object binaries themselves.

## Next bounded experiments

The new priority is unused constant ownership: verify the candidate309 unread
constants through the canonical all-path read inventory, capture a fresh complete
allocation stack census, then omit only their invocation owners. The618-request
estimate is an acceptance target, not a result. Keep immutable compiler/provenance
constants, current account checks, retirement, original/decoded/restored images,
finite failures and recovery. Live scalar-only constants are a separate later proof.

R91 admits the whole larger piece after retaining scalar payment. The new brief's
proposal to leave that prefix executing and derive only a native suffix is a
separate next runtime experiment; do not claim it is already implemented. Follow
with shape-solver temporaries and independently proved exit-argument stores.
Child seed handoffs, predicate-sensitive ownership and numeric outlining follow
those bounded measurements. Ordinary debit-function tuning stays closed.

## Fidelity

The [component assembly](../../hosts/gpu/tests/fixtures/native/deflate_assembly.rs)
performs all compression as one compiled computation. Source SHA-256
`331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`;
MBC SHA-256
`a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`.
Algorithm, tables, source allocation sites, depth300, nice258, near-EOF checks,
tree262144 U16/cache1500513 U32/path305257 U64, input aliases, pinned allocator,
output capacity13 clear/rebind and evaluate_into timer stay unchanged. Search
and skip remain separate; skip has no cache/recording path. Exact bytes
`[75,76,74,78,73,77,75,207,0,0]` and independent inflation agree. Final managed
bytes/nodes are zero. Physical public-output backing still changes.
