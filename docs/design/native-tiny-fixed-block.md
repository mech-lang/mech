# Proved tiny fixed blocks

This round reaches the eight-literal elapsed target against libdeflate 1.26.
It changes the compressor source and the compact runtime allocator policy;
it is not a compiler-only speedup. The PR remains draft: full 81-case timing
adoption is pending an uncontended run. Four attempts were interrupted by
independent compiler or benchmark jobs, and every partial result is retained.

## Result and provenance

On the Apple M1, two clean balanced runs of `abcdefgh` changed Mech/libdeflate
from 2.078/2.110 to **0.698/0.653**. The source change alone measured
0.708/0.914. Ratios are geometric means of paired block means, with 24 blocks,
64 complete calls per block, equivalent warmup and opposite measurement order
in the second run. Per-call medians and paired ratios are different statistics;
the raw values remain in the receipts. Earlier pilots, including the stronger
0.590/0.599 pair, are retained rather than substituted for the latest pair.

The separate 120-sample counter cohort reports:

| Metric | Retained | Both changes | libdeflate |
| --- | ---: | ---: | ---: |
| Instructions | 607,811 | 119,112 | 162,859 |
| Cycles | 163,843 | 50,711 | 74,558 |
| Mech payload allocation requests | 111 | 20 | — |
| Initialized payload bytes copied | 2,188 | 0 | — |
| Peak private payload capacity, bytes | 8,982,372 | 8,976,572 | — |
| Worker lifetime peak physical footprint, bytes | 42,451,776 | 46,842,624 | 2,081,472 |

Instructions fell 80.4%. Worker physical footprint increased 10.3%; this is an
adverse result. Worker memory includes descriptor parsing, setup and warmup,
and is distinct from live compressor payload capacity. No claim that these
memory figures measure only the compression call is intended. The instruction
and cycle cohort is separate from primary elapsed measurements.

A separate zero-compression startup control measures RSS after descriptor and
fixture parsing: 42,926,080 bytes before, 47,464,448 after. Thus the RSS increase
already exists before compression. Descriptor JSON grew from 2,292,881 to
2,519,706 bytes, and the Object grew from 1,238,656 to 1,419,832 bytes. These
size costs are retained alongside the elapsed/instruction improvement.

The published compiler parent is `213d90cc8e076938a2703411390b702539dd5d5a`, preserving
the parallel compilation, compiler-PGO and constant-phi experiment evidence.
The implementation head is `090e98046d935e17bb2800914b135114dbd62e00`. Its production
sources are identical to the measured `56f12da6f8` tree; the intervening parent
update changes only documentation and evidence:

- `d11b88dec6`: medium zeroed allocations retain allocator backing.
- `090e98046d`: source cost proof, guarded fixed writer, source pins and predicate tests.

Compiler binary SHA-256:
`e1f4d2480062cf938a305d8b9b2dee8db224b38f20f05feb3ef631bb4c1c8929`.
Source SHA-256:
`55cb2e698afb73da536164504b991dd514be855c9f4da9c3aadd046a55e1dba4`.
Bytecode SHA-256:
`00bba0ba762ad855af8123f223ee2ff9574473119f3293bf06f17cdfc80bcd79`.
Object SHA-256:
`72926dc267c83fbfa729710fe0c51c360ae68e69a15a23858be3710627158c5f`.

Fresh emission after restacking reproduces the pilot Object, bytecode and
metadata byte for byte. The qualified executables were reused after verifying
their full binary hashes. A terminology-only comment correction changes the
source hash while preserving all three emitted artifacts.

## Source proof

Distinct three-byte windows exclude every legal LZ match. The predicate checks
the length bound before scanning, so larger inputs do not perform a quadratic
scan. Inputs with repeated triples retain the original controller.

For a forced-literal block of length `n`, this encoder's zero-distance-frequency
Huffman construction supplies two one-bit distance lengths. Encoding those
lengths requires precode symbol 1. Its permutation position requires at least
18 explicit precode lengths (the encoded HCLEN field is 14).

The dynamic header therefore costs at least `17 + 3*18 = 71` bits. At least
three precode tokens are necessary: an explicit nonzero-length token covers
one entry, and any other token covers at most 138, insufficient for at least
258 entries with only two tokens. Payload and end marker require at least
`n+1` bits. Thus the complete dynamic block costs at least `75+n` bits.
Fixed coding costs at most `10+9*n`; stored coding costs at least `35+8*n`.
For `n <= 8`, fixed coding is strictly cheaper. The threshold follows from
`8*n < 65` rather than the contents of a named fixture.

The shortcut executes the existing real block collector and asserts that its
cache contains exactly one header per input byte and that this is the final
block. It then uses the existing fixed-code builder, header writer, literal
writer and end marker. The original stored route for lengths 0–7 remains.
The large allocations remain 262,144 U16 tree entries, 1,500,513 U32 cache
words and a 305,257-word U64 path reservation. Search depth 300, nice/max
length 258, near-EOF behavior, reversal table, caller capacity 13, output
transfer, aliases and timing boundaries are preserved. Losing Huffman/path
candidate work and its intermediate allocations are omitted on the proved
route. The same compiled computation accepts all 81 inputs and sizes.

## Allocator policy

Darwin previously mapped zeroed numeric payloads at four system pages. The
new crossover is 64 pages: 1 MiB with this M1's 16 KiB pages. Medium payloads
use the pinned allocator; larger numeric scratch still uses demand-zero
mapping. This is a backing-allocation policy, not a resource allowance.
Byte-output allocation, layout/alignment checks, error propagation and the
ownership interface are unchanged. Fourteen runtime controls cover zeroing,
live aliases, allocation refusal, cleanup, parallel handoff and recovery.

## Validation and remaining gate

- All 81 existing compression cases passed twice for each variant: exact pinned
  bytes, independent inflation, input/prior-output aliases, capacity and cleanup.
- 1,570 additional complete cases cover every eight-/nine-bit literal-width
  combination, binary patterns, random bytes, lengths 0–16 and two prefix states.
  There are 1,122 eligible cases and 448 fallback cases. Both variants and the
  pinned functional reference each passed 3,140 complete calls.
- The maintained predicate test executes 2,303 inputs in each of original and
  decoded artifacts, with the existing finite native qualification profile,
  unchanged aliases and zero retained managed bytes/nodes. It passes with
  warnings as errors after the rebase.
- Fresh artifact construction, bytecode decoding/re-encoding and emitted-image
  identity checks pass. Source-derived expected compressed bytes were not changed.

The predicate test initially selected smaller default envelopes and rejected
admission: compute 12,085,608 exceeded 1,000,000, then required memory 1,463,923
exceeded 1,048,576 bytes. The test now uses the established native qualification
profile. No existing component allowance or 1.05 timing gate was increased.

The batched literal writer trial was rejected: approximately 1.35x libdeflate
versus 1.27x for the preceding source pilot in its matched cohort. Full-screen
interference, disk denials and expired temporary support paths remain recorded.
Reference binaries and the 81 original C output tuples are now pinned locally;
this repairs the harness's missing historical receipt without changing a Mech
expected result or a timing allowance.

Full acceptance still requires two completed 81-case paired timing campaigns,
the existing per-case median and block-mean 1.05 gates, and review. No new global
win percentage is claimed from interrupted campaigns. The timing entry retains
its existing compact diagnostic contract: source arithmetic, assertions, bounds,
aliases and cleanup are checked; ordinary exact resource admission/accounting
and publication recovery are outside this pre-existing entry's contract.

Machine-readable results and exact test/producer commands are in
`benchmarks/mech-deflate/native-profile/eight-byte-fixed-block-m1-20261115.json`
and the [456-file evidence packet](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-eight-literal-fixed-block-20261115.tar.gz).
Archive SHA-256: `e93eca6a510db43970a0889057e70071e45e6a51d9efea1656d01d2051c3874a`.
The packet contains source, metadata and logs, with no native binaries or bytecode.
