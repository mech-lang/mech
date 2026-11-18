# Source-backed compact phase optimization on Apple M3

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
