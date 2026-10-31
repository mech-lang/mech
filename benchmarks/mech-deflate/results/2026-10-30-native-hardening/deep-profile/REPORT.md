# Complete eight-byte executable comparison

The current Mech computation executes **37.13× as many CPU instructions** and
requests **12.01× as many allocation bytes** as the pinned Roc computation.
Backtraces identify repeated path-buffer allocation and tree copying. The fresh
Mech profile also keeps ordinary frame transport, dispatch and accounting ahead
of compression arithmetic as the next area to simplify. No new optimization is
implemented by this profiling packet.

The case is complete level-12 compression of `abcdefgh`, producing the same ten
raw bytes, with the real 262144-entry U16 tree, 1500513-word U32 cache and
305257-entry U64 path capacity. This small-input result includes their required
initialization; it is not a larger-input throughput prediction. Source algorithm,
tables, search limits, depth 300, nice length 258, near-EOF behavior, source
allocation sites, allocator and timing boundaries are unchanged.

## Scorecard

MB below means decimal million bytes. CPU, allocation and primary elapsed-time
rows come from **separate cohorts**. Allocation instrumentation is deliberately
excluded from instruction/time comparisons. All ratios use the reference in the
same row as 1×.

| Metric per compression | Roc | Mech round 37 | Mech/reference |
| --- | ---: | ---: | ---: |
| Primary elapsed median, common reference | 0.282 ms | 4.6539165 ms | 16.50325× |
| Actual retired CPU instructions | 1,364,372 | 50,663,871 | 37.13347× |
| Actual CPU cycles | 939,783.5 | 14,760,138 | 15.70589× |
| Instructions/cycle, ratio of medians | 1.45179 | 3.43248 | different instruction mixes |
| Allocation requests, including realloc | 95 | 14,002 | 147.38947× |
| Requested allocation bytes | 9,021,923 | 108,355,555 | 12.01025× |
| Additional peak live usable heap above entry | 9,257,328 | 25,819,968 | 2.78914× |
| Reallocation calls | 0 | 1,213 | — |
| Relocating reallocation calls | 0 | 458 | — |
| Copy upper bound for those relocating reallocations only | 0 | 13,905 bytes | not total copying |

The elapsed row is the already recorded four-pair comparison: 3,968 primary
Mech calls plus 16 warm-up/profile calls. CPU medians use 32 compression calls
for each implementation; Mech warm-up/final calls are excluded. Allocation
medians use eight steady Mech calls and 32 reference calls. Histograms agree
across every steady call, and round 36 and round 37 have identical allocation
counts/bytes/peaks.

The CPU counters are `proc_pid_rusage(RUSAGE_INFO_V4)`'s `ri_instructions` and
`ri_cycles`, read at compression boundaries. These are **hardware instruction
counts**, not Mech's logical work receipt of 44,443,887,150. The 101 empty-boundary
observations report a median 5,010 instructions, about 0.37% of the reference and
0.01% of Mech. The values above are uncorrected; no silent subtraction is made.
Cycle counts depend on core/scheduling conditions and remain diagnostic, not a
replacement for paired elapsed timings. Every captured compression has zero
page-ins during its boundary.

The same pinned mimalloc object is used throughout. The allocation wrapper tracks
requested bytes and `mi_usable_size` at allocation/free boundaries, without an
allocating pointer map. Its focused calibration passes: three requests, 176
requested bytes, one realloc, two explicit frees, zero final live bytes and 144
peak usable bytes. The peak excludes allocator-private metadata, direct VM
mappings, libc-private heaps and transient old/new overlap internal to realloc.
It is an observed tracked-heap peak, not an exact peak of every byte in the task.

## What is being allocated and copied

| Scratch backing | Roc requests per compression | Mech requests | Observed Mech origin |
| --- | ---: | ---: | --- |
| U64 path buffer | one × 2,442,064 bytes | 39 × 2,442,056 bytes | `sequence_buffers::copy_buffer_with_sublist` |
| U16 tree | one × 524,336 bytes | five × 524,288 bytes | one initial fill; four `native_collection_word_updates::copy` calls |
| U32 cache | one × 6,002,100 bytes | one × 6,002,052 bytes | one initial fill |

Reference sizes include its headers/alignment; Mech rows are Vec payload requests.
One separate diagnostic captured and preserved all 45 relevant Mech allocation
backtraces. This confirms the actual allocation paths, rather than inferring them
from similarly sized objects. The four tree fallbacks copy all 262144 initialized
U16 words: **2,097,152 copied bytes** total. The path fallback reserves the full
capacity but copies only initialized words, and `Clear` copies no words. Thus
**92,798,128 extra path-capacity bytes requested beyond one allocation are not
92.8 MB of proven payload copying**. Spare capacity is not zero-filled there.

The existing selected buffer call already has a consuming ownership route. Its
fresh-backing fallback is reached when consumption is not proved or actual
exclusive ownership is unavailable. The current evidence does not yet distinguish
those reasons at every visit, nor establish which remaining aliases are dead.
Those reasons must be recorded before changing transport or owner retirement.
The cache is allocated once, so repeated cache cloning is not the explanation.

Both APIs receive already loaded whole input. Neither benchmark streams input
from a file during compression. Mech's ordinary immutable `Value` handle clones
share frozen payloads; the harness retains an input alias and verifies unchanged
contents/storage. No whole-input copy is present in the pinned compression source
except the payload append needed by a stored block. This does not prove that every
possible temporary eight-byte copy is absent; total input-copy bytes and an
input-size scaling experiment were not measured. The large identified repeats
are internal scratch buffers.

## Code size and reached work

| Static quantity | Roc executable | Mech |
| --- | ---: | ---: |
| Machine-code `__text` bytes | 195,456, entire application/runtime | 13,801,616, generated compressor entry alone |
| Four-byte instruction slots, including padding | 48,864 | 3,450,404 |
| Generated entry text symbols | included in 225 executable text symbols | one generated entry |
| Mech JIT test-runner text, separately | — | 48,186,240 bytes; compiler/runtime/verifier included |

The generated Mech entry alone is 70.61× the reference's entire text section.
These are static sizes, not executed instruction counts; they include error paths
and paths the eight-byte input never visits. The large JIT runner is not a fair
runtime-only AOT executable-size comparison. Exact file sizes/hashes are in the
verified pins receipt. A fresh round-37 Object/descriptor/metadata comparison is
recorded separately; all three are byte-identical to round 36.

The fresh original-reference sample has 1,513 main-thread leaf weights. Its
hottest function, `roc__p5794f552e2e0fcba87a322629fdca7c0.fast`, holds 1,416
(93.59%). The sampled offsets 17,896 and 17,988 disassemble to paired stores in
the tree and cache initialization loops. Their loop advances 40 bytes through
each backing. This identifies concrete work Roc is doing, rather than treating
its binary as a black box; it does not assign every sample in that function to
those two instructions.

Fresh Mech sampling partitions 1,351 exclusive leaf weights without double
counting: frame actions 366 (27.09%), ordinary visits 201 (14.88%), other invocation
helpers 193 (14.29%), selected library calls 145 (10.73%), remaining 446 (33.01%).
The last category includes generated code, shared runtime and outer verification.
This is a sampled main-thread profile including surrounding harness work, not a
precise partition of the compression timer. `_platform_memmove` has 27 exclusive
weights (2.00%); byte-copy elimination alone is not demonstrated to remove most
of elapsed time. Existing unchanged-program callback evidence records 13,678
admissions and 9,156 helper visits, 239 selected numeric-loop calls and 39 composed
pieces across 13 parent regions. That census is retained from round 33, not
misrepresented as a new round-37 capture.

## Whole-task memory is a separate problem

The CPU-only Mech runner has median RSS **2,093,334,528 bytes** before compression;
the reference diagnostic has **14,794,752 bytes**. Mech's tracked live heap at the
memory-cohort compression entry is **148,214,600 bytes**, with a tracked peak of
**174,034,568 bytes**; the reference starts with 32 usable bytes for its retained
output and peaks at 9,257,360 bytes. These scopes must not be conflated.

Kernel physical-footprint accounting is approximately 16.8 GB for the prepared
Mech runner, including its recorded lifetime maximum, versus approximately
14.4 MB in the reference diagnostic. The machine has only 8 GiB RAM: **16.8 GB is
not 16.8 GB resident RAM**. A held prepared runner's `vmmap -summary` shows about
1.5 GiB resident and 14.2 GiB swapped writable memory, with a 15.7 GiB footprint.
The raw region label is `IOAccelerator` in both binaries; that label alone is not
evidence that the compression uses a GPU. Compilation/preparation precedes every
reported compression timer and took approximately 29–31 seconds in these cohorts.

A causal diagnostic calls the same allocator's `mi_collect(true)` after
preparation. The footprint remains 15.7 GiB and swapped memory approximately
14.3 GiB. **A simple allocator collection does not fix it.** The exact preparation
allocation/fragmentation/VM retention cause remains unresolved; these captures do
not justify claiming it is solely freed pages or current live compressor state.
No allocator collection was added to the primary benchmark. A complete
runtime-only AOT compressor's whole-task memory has not been measured here.

## Next bounded targets

1. **Ordinary frame transport and ownership:** distinguish absent consume proof
   from failed exclusive transfer for the 38 additional path allocations and four
   tree copies. Then remove only proven dead transport aliases through the
   existing owner/ledger machinery. Track requests, requested bytes, initialized
   copying, instructions and incremental peak; preserve real aliases, errors,
   work receipts, publication, failure cleanup and retry. The requested-byte gap
   is a capacity-allocation opportunity, not a promised 12× time improvement.
2. **Reached dispatch/admission work:** use the fresh frame/helper profile and
   retained callback census to select one ordinary visited region for a general
   simplification. Roc's direct initialization loops provide a concrete contrast
   to repeated runtime bookkeeping. Keep all scalar types, finite admission and
   failure ordering; avoid a second executor or benchmark-specific path.
3. **Preparation memory and generated-code expansion:** separately attribute the
   16.8 GB task footprint and 13.8 MB generated entry. A runtime-only executable
   measurement must precede any whole-task AOT memory claim. Do not disguise
   startup-memory work as compression-time improvement.

Every future candidate should repeat the existing primary comparison and this
scorecard. Instructions, allocation requests/bytes, tracked peak live bytes,
large-buffer frequency, code bytes and startup/RSS/footprint remain separate
columns. Fewer instructions are useful evidence but differing IPC prevents a
linear speedup promise.

## Revisions, reproduction and limitations

Base: `eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`; published starting head:
`4686620853914517474a23e9d8a6ff1cf11db5cf`. Measured production source round 36:
`9047e634e5964ad23ae19449bc425c2a3cf325b2`; pending layout candidate round 37:
`3a54f1a1d64db9eee01db97763a3f95a138db93a`. IR/work/ABI/bytecode stay
131/95/37/14. Reference compiler recorded revision:
`d99c7b838fb05bb1047eb0c18d4d06d0f86f345d`; package:
`671077457ff7ef3a3d44caf715a3fc595addf379`; allocator:
`8c532c32c3c96e5ba1f2283e032f69ead8add00f` (mimalloc 2.1.7).

Mech source SHA-256:
`331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`;
bytecode:
`a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`;
allocator Object:
`38c6ec82a41e90a96bb9f1ae2c89058a77909da3fbb69c16e542ae5c858bf874`.
All eleven exact binary/source/metadata hashes are in `verified-pins/receipts.tar.gz`.
Hardware: Apple M1 Mac mini, 8 GiB RAM, macOS 15.6.1 (24G90); Instruments 26.0
(17A400). Rust nightly 2026-03-03, aarch64, opt 3, warnings as errors, unchanged
selected-operation libraries; no full-math-library rebuild for this comparison.

The source-derived reference package had been removed by prior cleanup. All 14
files were recovered at the fixed revision and verified against their Git blob
hashes, preserving their exact bytes. The original Bench program, eight input
bytes, expected bytes, prefix and repetitions are unchanged. Only separate host
timer/allocation probes were added for diagnostic binaries. All 20,000 timed
reference calls per run validate the raw bytes; its original preflight also
round-trips them. Every Mech observation validates raw bytes, independent
inflation, input aliases/contents, capacity 13, exact work and final zero cleanup.

Each cohort has a manifest and compressed source/command/stdout/stderr receipts:
`cpu-round36`, `cpu-round37`, `reference-cpu`, `memory-round36`, `memory-round37`,
`reference-memory`, `allocation-attribution`, `binary-inspection`,
`retained-memory`, `generated-object`, `pins-and-assembly` and `verified-pins`.
Each measurement cohort was committed immediately. The exact Rust link commands
and frozen dependency hashes are in its receipts. C probes compile with
`clang -O3 -std=c11 -Wall -Wextra -Werror`. Reference diagnostic builds retain
`roc build app.roc --opt=speed --target=arm64mac --jobs=1`.

To regenerate the numeric analysis, unpack all receipts into one scratch root
and run `python3 analyze.py ROOT`; [scorecard.json](scorecard.json) preserves
medians/minima/maxima and provenance, and [scorecard.csv](scorecard.csv) is the
compact numeric baseline. This step runs no benchmarks. Exact diagnostic commands
and source are preserved in the receipts; no builds/tests/profiles overlapped
primary elapsed timings.

An independent fresh extraction of all 13 receipt groups verifies **215 file
hashes** and reproduces both scorecard files byte-for-byte. Its exact output
digests and checked inventory are in [reconstruction.json](reconstruction.json).

No compressor/runtime correctness failure occurred in these diagnostic cohorts.
Setup failures were missing cleaned-up sources/header/Git metadata, an initial
extra EOF newline caught by strict blob verification, and unprivileged hardware
tool cache/sysctl access. Sources/header were restored and verified; approved
tool access succeeded. No fresh Git-HEAD claim is made for the deleted reference
checkout. The initial missing pins-file archive omission was corrected in the
separate `verified-pins` receipt, and its helper now rejects missing inputs.
The memory harness's inherited free-text allocator label says “no allocation
probe”; its explicit `memory_instrumented: true`, source and link commands are
authoritative. Its time/instruction fields are not used for the CPU scorecard.

The original reference CPU Counters trace and exported thread/core tables are
preserved. No unverified interpretation of their bottleneck array is used, and
no matched cache-miss, branch-miss, bandwidth, energy or total-memcpy comparison
is claimed. Larger-input timing remains excluded. Hosted CI stays cancelled by
user direction; draft/review status and remaining inherited coverage exclusions
are documented in the accompanying round-37 closure.
