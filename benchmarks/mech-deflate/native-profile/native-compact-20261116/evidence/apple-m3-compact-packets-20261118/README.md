# Source-backed compact phase optimization on Apple M3

## Retained M3 result

The [word equality-scan and guard-reuse build](word-scans/README.md) has **64,512 B** of encoder text and a measured **4,736 B** native stack high-water, below the unchanged 5,280 B bound. Its [complete confirmation](word-scans/confirmation/README.md) records **81/81 Roc and 61/81 libdeflate 1.26 level 12 in both orders** with the original source, canonical bytecode, runtime, allocator and peer binaries unchanged. The initial full campaign records 81/81 and 60/81; near-tied libdeflate cases account for the count variation. No improvement is attributed to a rejected challenger.

The confirmation meets the M3 comparison target and does not claim the separate v3 promotion floor. [Confirmation timings in seconds and comparator multiples](word-scans/confirmation/timings.csv) · [Binary recipe](word-scans/recipe.json) · [All screened and full comparisons](word-scans/comparisons.json)

## Earlier compact-packet campaign

Smaller synchronous phase packets, selected output words, checked fallback
reuse and empty-source route forwarding improve the complete 81-case native
DEFLATE comparison on Apple M3. The original Mech source and canonical bytecode
are unchanged. The encoder is rebuilt locally from the published compiler
source changes; its object and metadata exactly reproduce the published
67,656-byte candidate.

| Complete order | Previous Mech / Roc wins | Candidate / Roc wins | Previous Mech / libdeflate wins | Candidate / libdeflate wins |
| --- | ---: | ---: | ---: | ---: |
| Forward | 69/81 | **78/81** | 57/81 | **58/81** |
| Reverse | 68/81 | **76/81** | 58/81 | **59/81** |

The candidate does not meet the M3 promotion threshold of 81/81 against Roc and
at least 61/81 against libdeflate 1.26 level 12 in both orders. The three natural
text prefixes around 64 KiB lose to Roc in both orders; two text cases around
32 KiB also narrowly lose in the reverse order. These are contemporaneous
comparisons with unchanged peer executables, fresh per-call encoder state and
the existing pinned fixture/output contracts. No M1 timing is used as an M3
result. Runtime ratios below one are faster than the named comparator.
[Every case in seconds and comparator multiples](timings.csv) ·
[Complete order counts](results.json) · [Raw-record replay](raw-replay.json)

The candidate passes 162 exact-output and independent-decompression calls,
borrowed input/output alias checks, checked allocation failures and cleanup.
The measured peak stack is 4,672 bytes, below the unchanged 5,280-byte ceiling.
Root stack falls from 3,728 to 3,120 bytes; encoder text falls from 71,192 to
67,656 bytes. Heap/runtime support and the allocator remain frozen.

## Reproduction

The maintained source updates and cumulative experimental patch are identified
by [Git blob and SHA-256 pins](source-adoption.json) and
[published candidate source identities](upstream-source-pins.json). They are
applied as compiler changes, including their original optional switches; the
benchmark program is not rewritten. [The recipe](recipe.json) contains every
native compiler option and the three checked block-order profiles. Build the
compiler from this branch, then run `reproduce.py COMPILER OUTPUT_DIRECTORY`.
The replay checks the program, profile, object, metadata and bytecode hashes.
The [local build record](local-build.json) preserves the complete linker inputs
and the [published reproduction check](published-reproduction.json) records the
cross-machine object identity. Source-build durations in this record are
diagnostic and are not controlled compilation benchmarks.


## Further compiler experiments

[Direct scalar homes and shared checked returns](direct-scalar-homes/README.md)
reproduce the newer published encoder from source. Their M3 runtime
qualification remains separate from the complete compact-packet results above.

A [verified evidence archive](compiler-wave-archive.json) preserves the later
load/writeback, paired-reduction, comparison-scheduling, input-packet and
matcher-layout experiments, including rejected combinations, source patches,
full source copies, native correctness tests and raw timing observations.
Apply this delta after the two archives named in its manifest. At the time of
that earlier archive, the complete M3 runtime target remained unmet.

The subsequent [word-scan evidence archive](scan-wave-archive.json) preserves
the retained encoder's initial and confirmation campaigns, raw-record replay,
and rejected prefetch and allocation experiments. GitHub's uploaded asset
matches the recorded SHA-256. The confirmation reaches 81/81 Roc and 61/81
libdeflate in both orders with the near-tie sensitivity described above.
