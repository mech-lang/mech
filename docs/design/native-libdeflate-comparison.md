# Matched level-12 compression with libdeflate 1.26

Each ratio divides Mech elapsed by the named comparison implementation’s elapsed within the same blocks. R268 is the retained Mech build entering this comparison. R270 is the tiered-loop candidate.

The cohort covers all 81 inputs, with 245,664 measured calls across 24 balanced blocks per input. Every actor runs raw DEFLATE at level 12 with runtime input. All three implementations produce byte-for-byte identical compressed streams at the original capacities on every input. Independent inflate and held input/output alias checks pass throughout.

| Input | Mech R268 / libdeflate | Mech R268 / Roc | Mech R270 / libdeflate | Mech R270 / Roc | Raw bytes |
|---|---:|---:|---:|---:|---:|
| eight-literals | 5.28402× | 0.41357× | 5.45503× | 0.42695× | 10 |
| prefix-repeated-32768 | 1.96378× | 1.00648× | 1.91804× | 0.98304× | 47 |
| prefix-repeated-33027 | 1.96993× | 1.01447× | 1.93513× | 0.99654× | 48 |
| prefix-periodic-32768 | 2.23088× | 1.04047× | 2.15249× | 1.00392× | 150 |
| prefix-natural-32768 | 1.53749× | 1.68604× | 1.54805× | 1.69761× | 8,963 |
| prefix-repeated-1048576 | 1.79566× | 1.14417× | 1.75796× | 1.12015× | 1,071 |
| prefix-periodic-1048576 | 2.00080× | 1.18005× | 1.95832× | 1.15500× | 3,144 |
| prefix-natural-1048576 | 1.41363× | 1.75043× | 1.42421× | 1.76353× | 265,467 |
| prefix-random-1048576 | 2.97746× | 2.10260× | 2.99757× | 2.11680× | 1,048,666 |

Retained Mech runs faster than libdeflate on 3/81 cases in this cohort. Those are empty, one-byte and seven-byte stored streams. The libdeflate call includes fresh compressor setup and free on each invocation. Its short-input path begins after that setup.

## Measurement contract

Unmodified libdeflate 1.26 at 92e6a0db9fa848d742f9eb286c92afc60f2c3dda is statically linked on Apple ARM64. Apple Clang builds the upstream hosted library sources at its default release optimization level O2 with NDEBUG. The Rust worker uses the same pinned mimalloc and Instant clock as the other workers. The measured C-library wrapper creates a level-12 compressor, allocates a distinct returned output, copies the held prefix, compresses raw DEFLATE and frees the compressor before the timer ends. Preparation, validation and release of the previous output are outside the timer. A 200 ms warmup per actor precedes the 24 actor permutations for each input. The ratios are geometric means of the paired block-mean ratios. This is a separate cohort from the two 81-case qualification cohorts. The compact Mech entries omit ordinary managed admission and recovery. [Pinned libdeflate release](https://github.com/ebiggers/libdeflate/releases/tag/v1.26).

## Source and actual machine code

The pinned Mech level-12 benchmark source is 1,931 physical lines. The libdeflate encoder file is 4,135 lines, or 4,779 with the binary-tree, common matchfinder and ARM matchfinder headers. The C file includes the other levels. The 40 C and header files in the full C library total 14,573 lines, including decompression and checksums. [Pinned upstream sources](https://github.com/ebiggers/libdeflate/tree/92e6a0db9fa848d742f9eb286c92afc60f2c3dda).

For eight literals, Mech allocates its full zero-initialized tree and match cache. The R270 memory cohort reports 111 payload requests, 9,024,420 requested payload bytes and 6,579,087 bytes requested zero-initialized. Allocator zeroing accounts for 3,351 of 5,500 running stack samples. The C encoder makes one compound scratch allocation of 9,011,712 bytes plus alignment overhead. It initializes 393,216 bytes of hash heads and its small tables. Match-cache and path-node storage is written as needed. The C eight-literal profile shows 2,129 of 5,265 samples in Huffman construction and 1,986 in the matchfinder hash fill. These are diagnostic CPU samples with validation in the call stream. They support testing whether private scratch initialization is observable before its defining stores. An optimization must preserve logical zero values and materialize them on any route that can observe them.

For repeated 33,027 bytes, the Mech matcher is the largest sampled function at 3,178 of 5,595 timer samples. The no-growth path parser is next at 1,005 samples. The C parser keeps the previous node cost in w9 and walks the cache directly. Its inspected no-match iteration has 16 instructions and zero stack transfers. Mech performs the corresponding iteration in 50 instructions with two stack transfers. The Mech path routine is byte-identical in R268 and R270. A bounded compiler follow-up can forward the preceding private append to the next last-node read and keep the cache geometry through the loop. The existing 64-byte match extension already compares wider groups than the inspected C eight-byte loop. The path comparison identifies an independent source of recurring work.

## Full comparison

| Input | Mech R268 / libdeflate | Mech R268 / Roc | Mech R270 / libdeflate | Mech R270 / Roc | Raw bytes |
|---|---:|---:|---:|---:|---:|
| empty | 0.01664× | 2.10086× | 0.01666× | 2.10417× | 5 |
| one-stored | 0.01773× | 1.99089× | 0.01773× | 1.99174× | 6 |
| seven-stored-prefix | 0.01870× | 2.24072× | 0.01887× | 2.26017× | 12 |
| eight-literals | 5.28402× | 0.41357× | 5.45503× | 0.42695× | 10 |
| eight-overlap | 5.15309× | 0.40608× | 5.35496× | 0.42199× | 5 |
| 320-overlap | 4.85678× | 0.44912× | 4.96251× | 0.45890× | 7 |
| eight-literals-prefix | 5.17214× | 0.40588× | 5.46683× | 0.42900× | 10 |
| 128-biased-dynamic | 5.48649× | 0.41190× | 5.66116× | 0.42502× | 65 |
| 32-high-entropy-stored | 5.33659× | 0.41882× | 5.55974× | 0.43633× | 37 |
| 16384-distribution-change | 3.00931× | 1.72900× | 3.03042× | 1.74113× | 11,394 |
| 33027-periodic-window | 2.21487× | 1.03720× | 2.16955× | 1.01597× | 151 |
| random-64 | 5.14667× | 0.42439× | 5.37259× | 0.44302× | 69 |
| random-256 | 5.32061× | 0.46832× | 5.57828× | 0.49100× | 261 |
| random-1024 | 4.85428× | 0.52016× | 5.09321× | 0.54577× | 1,029 |
| repeated-1024 | 3.56621× | 0.50459× | 3.64293× | 0.51545× | 11 |
| random-32769 | 3.57806× | 1.76231× | 3.56068× | 1.75375× | 32,774 |
| repeated-32769 | 1.95260× | 1.00305× | 1.89425× | 0.97307× | 47 |
| reymont-prefix-1024 | 3.61301× | 0.64325× | 3.73522× | 0.66501× | 460 |
| reymont-prefix-16384 | 1.61020× | 1.60909× | 1.62436× | 1.62323× | 4,616 |
| reymont-prefix-32769 | 1.53503× | 1.68300× | 1.54762× | 1.69680× | 8,964 |
| heldout-window-32767 | 1.53359× | 1.68072× | 1.55040× | 1.69913× | 8,964 |
| heldout-window-32768 | 1.53289× | 1.67708× | 1.54441× | 1.68969× | 8,963 |
| heldout-slides-65537 | 1.54732× | 1.76632× | 1.55313× | 1.77296× | 16,952 |
| heldout-random-65537 | 3.24384× | 1.90827× | 3.25085× | 1.91239× | 65,547 |
| heldout-mixed-131073 | 2.04447× | 1.76365× | 2.04217× | 1.76166× | 74,907 |
| prefix-natural-1024 | 3.58622× | 0.64330× | 3.70945× | 0.66541× | 460 |
| prefix-natural-4096 | 2.07174× | 1.19919× | 2.07492× | 1.20103× | 983 |
| prefix-natural-8192 | 1.76169× | 1.41275× | 1.78829× | 1.43409× | 2,309 |
| prefix-natural-16384 | 1.62509× | 1.61437× | 1.63406× | 1.62329× | 4,616 |
| prefix-natural-32767 | 1.53191× | 1.68090× | 1.54372× | 1.69386× | 8,964 |
| prefix-natural-32768 | 1.53749× | 1.68604× | 1.54805× | 1.69761× | 8,963 |
| prefix-natural-32769 | 1.52981× | 1.67700× | 1.54270× | 1.69113× | 8,964 |
| prefix-natural-33027 | 1.53885× | 1.68375× | 1.54734× | 1.69304× | 9,029 |
| prefix-natural-65535 | 1.44406× | 1.71428× | 1.45617× | 1.72866× | 17,230 |
| prefix-natural-65536 | 1.44609× | 1.71387× | 1.45575× | 1.72532× | 17,229 |
| prefix-natural-65537 | 1.44759× | 1.71956× | 1.48378× | 1.76254× | 17,229 |
| prefix-natural-131072 | 1.41603× | 1.71866× | 1.43023× | 1.73589× | 33,920 |
| prefix-natural-262144 | 1.40602× | 1.73282× | 1.41970× | 1.74968× | 67,304 |
| prefix-natural-1048576 | 1.41363× | 1.75043× | 1.42421× | 1.76353× | 265,467 |
| prefix-random-1024 | 4.94857× | 0.53838× | 4.99768× | 0.54373× | 1,029 |
| prefix-random-4096 | 4.49308× | 0.93223× | 4.57014× | 0.94822× | 4,101 |
| prefix-random-8192 | 4.11619× | 1.15262× | 4.16742× | 1.16697× | 8,197 |
| prefix-random-16384 | 4.06377× | 1.49705× | 4.10817× | 1.51341× | 16,389 |
| prefix-random-32767 | 3.59155× | 1.76099× | 3.58787× | 1.75919× | 32,772 |
| prefix-random-32768 | 3.61439× | 1.77102× | 3.59893× | 1.76344× | 32,773 |
| prefix-random-32769 | 3.54104× | 1.75872× | 3.54622× | 1.76129× | 32,774 |
| prefix-random-33027 | 3.56093× | 1.75678× | 3.59785× | 1.77499× | 33,032 |
| prefix-random-65535 | 3.29723× | 1.94628× | 3.30227× | 1.94926× | 65,540 |
| prefix-random-65536 | 3.30486× | 1.93692× | 3.31273× | 1.94153× | 65,546 |
| prefix-random-65537 | 3.27619× | 1.94191× | 3.27787× | 1.94290× | 65,547 |
| prefix-random-131072 | 3.15323× | 2.03045× | 3.14675× | 2.02627× | 131,087 |
| prefix-random-262144 | 3.07960× | 2.05837× | 3.07445× | 2.05493× | 262,169 |
| prefix-random-1048576 | 2.97746× | 2.10260× | 2.99757× | 2.11680× | 1,048,666 |
| prefix-periodic-1024 | 3.65693× | 0.50739× | 3.76967× | 0.52304× | 52 |
| prefix-periodic-4096 | 2.97332× | 0.72369× | 2.93877× | 0.71528× | 67 |
| prefix-periodic-8192 | 2.59761× | 0.82681× | 2.57201× | 0.81866× | 79 |
| prefix-periodic-16384 | 2.36833× | 0.93272× | 2.31863× | 0.91314× | 103 |
| prefix-periodic-32767 | 2.21800× | 1.02973× | 2.15801× | 1.00188× | 150 |
| prefix-periodic-32768 | 2.23088× | 1.04047× | 2.15249× | 1.00392× | 150 |
| prefix-periodic-32769 | 2.21891× | 1.03521× | 2.15835× | 1.00696× | 150 |
| prefix-periodic-33027 | 2.23506× | 1.05217× | 2.17745× | 1.02505× | 151 |
| prefix-periodic-65535 | 2.12858× | 1.10756× | 2.08061× | 1.08260× | 245 |
| prefix-periodic-65536 | 2.12151× | 1.10763× | 2.07185× | 1.08170× | 245 |
| prefix-periodic-65537 | 2.12127× | 1.10405× | 2.06942× | 1.07707× | 245 |
| prefix-periodic-131072 | 2.07732× | 1.14069× | 2.03012× | 1.11478× | 436 |
| prefix-periodic-262144 | 2.04859× | 1.16261× | 2.00423× | 1.13744× | 817 |
| prefix-periodic-1048576 | 2.00080× | 1.18005× | 1.95832× | 1.15500× | 3,144 |
| prefix-repeated-1024 | 3.61201× | 0.51928× | 3.60426× | 0.51817× | 11 |
| prefix-repeated-4096 | 2.62440× | 0.69899× | 2.65448× | 0.70700× | 22 |
| prefix-repeated-8192 | 2.31740× | 0.81634× | 2.27791× | 0.80243× | 26 |
| prefix-repeated-16384 | 2.10883× | 0.92121× | 2.06100× | 0.90031× | 34 |
| prefix-repeated-32767 | 1.94904× | 1.00342× | 1.92590× | 0.99151× | 49 |
| prefix-repeated-32768 | 1.96378× | 1.00648× | 1.91804× | 0.98304× | 47 |
| prefix-repeated-32769 | 1.94452× | 0.99183× | 1.91419× | 0.97636× | 47 |
| prefix-repeated-33027 | 1.96993× | 1.01447× | 1.93513× | 0.99654× | 48 |
| prefix-repeated-65535 | 1.89850× | 1.07650× | 1.85968× | 1.05448× | 79 |
| prefix-repeated-65536 | 1.89494× | 1.07219× | 1.85954× | 1.05216× | 79 |
| prefix-repeated-65537 | 1.89239× | 1.07205× | 1.86446× | 1.05623× | 80 |
| prefix-repeated-131072 | 1.87189× | 1.11631× | 1.83238× | 1.09275× | 145 |
| prefix-repeated-262144 | 1.84099× | 1.13104× | 1.80678× | 1.11002× | 272 |
| prefix-repeated-1048576 | 1.79566× | 1.14417× | 1.75796× | 1.12015× | 1,071 |

[Build, timing, profile and replay receipts](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r270-equality-tiers-and-library-comparison-11d6708c6ba7.tar.gz).
