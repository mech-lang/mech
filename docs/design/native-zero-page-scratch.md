# R272 — Supply large zero-filled scratch from private pages

**Retain the R272 allocation policy with the R270 compiler image.** Paired elapsed changes -10.253% across all 81 cases. The fresh primary score moves from 27/81 to 31/81 below 1× Roc. The separate full library comparison records 3/81 candidate cases below 1× libdeflate.

Published parent 5c4b281a2e4e8e4539035acb27abf7b43de496e6. Measured R270 parent 8830db78789aee692d89583d25458bcc32f300b3. PR base 92f75db84e2523693648084bea16cd52ad60777a. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2 and IR/work/ABI/bytecode 215/107/39/14. The exact R270 compressor Object, metadata, source program and source bytecode are retained. The full level-12 algorithm, tables, capacities, source allocation sites, held aliases and output transfer protocol are unchanged.

## Allocation policy

The measured change lives in the compact allocation helper. A zero-filled numeric allocation may use a fresh private anonymous mapping when it spans at least four system pages and its alignment fits a page. The operating system establishes the entire logical zero-filled allocation, including cells read before any write. Byte results continue through the original allocator and Vec transfer. Other allocations use the pinned mimalloc. The same policy maps two large allocations totaling 6,526,340 payload bytes on every compression case with at least eight input bytes. The short stored cases take their existing route.

A backing flag occupies a reserved high bit in the checked nonempty allocation alignment. The owner remains six words and its reference count stays at the original offset. A validated nonempty Rust Layout leaves that bit clear. Zero-sized allocations return through the original empty-view branch. The original wrapper and primary counter layout are byte-identical to their R270 sources. Additional mapping counters are confined to the separate memory and test builds.

This is an allocator experiment in the experimental compact runtime on macOS. A compiler proof that removes logically unnecessary initialization remains future work. The additional ordinary managed admission, accounting and recovery contract stays outside the timing boundary.

## Full acceptance

Both timing gates pass 81/81 and 81/81. Primary A changes -10.431% and B -10.074%. Cases crossing below parity include 33027-periodic-window, prefix-periodic-32767, prefix-periodic-32768, prefix-periodic-32769. Cases crossing above parity include none.

The first two 19-case screens preserve their tiny-case gate failures. The first build carried mapping counters in the primary wrapper. The next build moved those counters to diagnostic builds. The final build restores the original six-word owner using the reserved alignment bit. All 19 final-screen cases pass both gates. Every observation and prior source version is retained.

| Input | R270 / Roc | R272 / Roc | Paired elapsed change | Instructions change | Cycles change |
|---|---:|---:|---:|---:|---:|
| eight-literals | 0.436912× | 0.201019× | -53.991% | -28.391% | -49.470% |
| prefix-repeated-4096 | 0.696136× | 0.539507× | -22.500% | -7.495% | -22.312% |
| prefix-repeated-32768 | 0.976002× | 0.936350× | -4.063% | -0.971% | -3.812% |
| prefix-repeated-33027 | 0.995875× | 0.977918× | -1.803% | -0.443% | -0.765% |
| prefix-periodic-32767 | 1.002965× | 0.985281× | -1.763% | -0.481% | -2.335% |
| prefix-periodic-32768 | 1.009122× | 0.984176× | -2.472% | -0.483% | -2.256% |
| prefix-periodic-32769 | 1.008133× | 0.992815× | -1.519% | -0.454% | -1.436% |
| prefix-natural-32768 | 1.693124× | 1.679867× | -0.783% | -0.019% | +0.280% |
| prefix-natural-1048576 | 1.768028× | 1.757598× | -0.590% | +0.060% | +0.416% |
| prefix-random-1048576 | 2.115942× | 2.106129× | -0.464% | -0.101% | -0.565% |

## Matched libdeflate comparison

The full 81-input four-actor cohort measures retained Mech, the allocation candidate, Roc and libdeflate in all 24 actor orders per input. Every implementation returns identical raw level-12 bytes at the original capacities. The libdeflate call includes a freshly allocated compressor and its cleanup. Each displayed ratio uses Mech as its numerator. This cohort remains separate from both primary and counter cohorts. The preceding five-case cohorts remain in the packet.

| Input | R270 / libdeflate | R272 / libdeflate | R270 / Roc | R272 / Roc |
|---|---:|---:|---:|---:|
| eight-literals | 5.324992× | 2.530654× | 0.416211× | 0.197800× |
| prefix-repeated-4096 | 2.543811× | 1.997397× | 0.673301× | 0.528675× |
| prefix-repeated-33027 | 1.946909× | 1.909263× | 1.001379× | 0.982016× |
| prefix-periodic-32768 | 2.170907× | 2.118972× | 1.012630× | 0.988405× |
| prefix-natural-1048576 | 1.425009× | 1.418240× | 1.765329× | 1.756943× |
| prefix-random-1048576 | 2.992581× | 2.977810× | 2.117091× | 2.106640× |

## Memory and correctness

The separate heap cohort preserves all 81 logical payload-request, requested-byte, initialized-copy, logical-zero, private-peak and final-live fields. Mapped payload bytes describe a logical reservation. They do not measure actual physical zeroing or resident pages touched. Whole-call instruction and physical-memory samples are separate from the primary timings.

| Eight-literal metric | R270 | R272 |
|---|---:|---:|
| Payload requests | 111.0 | 111.0 |
| Requested payload bytes | 9,024,420.0 | 9,024,420.0 |
| Private payload peak | 8,982,372.0 | 8,982,372.0 |
| Logical zero-filled bytes | 6,579,087.0 | 6,579,087.0 |
| Initialized copied bytes | 2,188.0 | 2,188.0 |
| Retired instructions | 1,134,078.0 | 812,107.5 |
| CPU cycles | 423,574.0 | 214,032.5 |
| Lifetime peak physical footprint | 49,873,856.0 | 43,975,616.0 |

- Both full artifact forms produce 324 exact complete-call results with independent inflate, retained input/output aliases and zero final private ownership.
- Sixteen current runtime tests run across original and decoded linked images. Ten actual linked calls cover successful publication, lexical assertion before a failing load, allocation rejection before the assertion, bounds rejection and retry. Previous publications and caller input remain unchanged throughout.
- The compiler and compression Object remain the R270 revision. Its existing compiler qualification is inherited. The local runtime tests and complete compression checks are new.
- Four exact-image profiles run after all primary and counter cohorts. Their samples include the worker and validation. They remain separate from retired-instruction counts.
- In the eight-literal profile, allocator zero-fill changes from 3,351/5,500 to 71/5,490 running samples under the same stack predicate. The mapped build records 57 mapping and 389 unmapping samples under the allocation helper.
- Hosted CI is canceled under the existing instruction. The full ordinary managed-contract and linked/AOT qualification is outside this allocator experiment.

## All 81 paired timings

| Input | Bytes | R270 / Roc | R272 / Roc | R272 / R270 | Pooled and block gates |
|---|---:|---:|---:|---:|---|
| empty | 0 | 2.163521× | 2.093748× | 0.967750× | pass/pass |
| one-stored | 1 | 2.034150× | 1.977999× | 0.972396× | pass/pass |
| seven-stored-prefix | 7 | 2.296036× | 2.268507× | 0.988010× | pass/pass |
| eight-literals | 8 | 0.436912× | 0.201019× | 0.460090× | pass/pass |
| eight-overlap | 8 | 0.429845× | 0.189801× | 0.441557× | pass/pass |
| 320-overlap | 320 | 0.467818× | 0.231840× | 0.495578× | pass/pass |
| eight-literals-prefix | 8 | 0.425485× | 0.198632× | 0.466836× | pass/pass |
| 128-biased-dynamic | 128 | 0.417313× | 0.268392× | 0.643143× | pass/pass |
| 32-high-entropy-stored | 32 | 0.439150× | 0.267644× | 0.609459× | pass/pass |
| 16384-distribution-change | 16384 | 1.731601× | 1.679660× | 0.970004× | pass/pass |
| 33027-periodic-window | 33027 | 1.010861× | 0.996237× | 0.985533× | pass/pass |
| random-64 | 64 | 0.446030× | 0.282120× | 0.632514× | pass/pass |
| random-256 | 256 | 0.486574× | 0.349018× | 0.717297× | pass/pass |
| random-1024 | 1024 | 0.545370× | 0.441762× | 0.810022× | pass/pass |
| repeated-1024 | 1024 | 0.509171× | 0.306970× | 0.602881× | pass/pass |
| random-32769 | 32769 | 1.760359× | 1.734690× | 0.985418× | pass/pass |
| repeated-32769 | 32769 | 0.976897× | 0.959860× | 0.982561× | pass/pass |
| reymont-prefix-1024 | 1024 | 0.661298× | 0.552001× | 0.834723× | pass/pass |
| reymont-prefix-16384 | 16384 | 1.625308× | 1.597618× | 0.982963× | pass/pass |
| reymont-prefix-32769 | 32769 | 1.689846× | 1.677255× | 0.992549× | pass/pass |
| heldout-window-32767 | 32767 | 1.688916× | 1.677151× | 0.993034× | pass/pass |
| heldout-window-32768 | 32768 | 1.697837× | 1.681311× | 0.990267× | pass/pass |
| heldout-slides-65537 | 65537 | 1.776566× | 1.730749× | 0.974210× | pass/pass |
| heldout-random-65537 | 65537 | 1.919591× | 1.910230× | 0.995124× | pass/pass |
| heldout-mixed-131073 | 131073 | 1.762464× | 1.752467× | 0.994327× | pass/pass |
| prefix-natural-1024 | 1024 | 0.662550× | 0.557140× | 0.840902× | pass/pass |
| prefix-natural-4096 | 4096 | 1.217517× | 1.149299× | 0.943970× | pass/pass |
| prefix-natural-8192 | 8192 | 1.439629× | 1.400508× | 0.972826× | pass/pass |
| prefix-natural-16384 | 16384 | 1.621090× | 1.599558× | 0.986717× | pass/pass |
| prefix-natural-32767 | 32767 | 1.692229× | 1.677207× | 0.991123× | pass/pass |
| prefix-natural-32768 | 32768 | 1.693124× | 1.679867× | 0.992170× | pass/pass |
| prefix-natural-32769 | 32769 | 1.695957× | 1.682724× | 0.992198× | pass/pass |
| prefix-natural-33027 | 33027 | 1.695903× | 1.680941× | 0.991177× | pass/pass |
| prefix-natural-65535 | 65535 | 1.731213× | 1.722919× | 0.995209× | pass/pass |
| prefix-natural-65536 | 65536 | 1.731166× | 1.721488× | 0.994410× | pass/pass |
| prefix-natural-65537 | 65537 | 1.764496× | 1.724022× | 0.977062× | pass/pass |
| prefix-natural-131072 | 131072 | 1.739603× | 1.732855× | 0.996121× | pass/pass |
| prefix-natural-262144 | 262144 | 1.750509× | 1.745861× | 0.997345× | pass/pass |
| prefix-natural-1048576 | 1048576 | 1.768028× | 1.757598× | 0.994101× | pass/pass |
| prefix-random-1024 | 1024 | 0.542310× | 0.442391× | 0.815754× | pass/pass |
| prefix-random-4096 | 4096 | 0.920324× | 0.798921× | 0.868087× | pass/pass |
| prefix-random-8192 | 8192 | 1.146692× | 1.051330× | 0.916838× | pass/pass |
| prefix-random-16384 | 16384 | 1.536496× | 1.464015× | 0.952827× | pass/pass |
| prefix-random-32767 | 32767 | 1.767278× | 1.739745× | 0.984420× | pass/pass |
| prefix-random-32768 | 32768 | 1.770134× | 1.737803× | 0.981735× | pass/pass |
| prefix-random-32769 | 32769 | 1.775086× | 1.734936× | 0.977381× | pass/pass |
| prefix-random-33027 | 33027 | 1.782648× | 1.742809× | 0.977651× | pass/pass |
| prefix-random-65535 | 65535 | 1.949141× | 1.928087× | 0.989198× | pass/pass |
| prefix-random-65536 | 65536 | 1.958652× | 1.940557× | 0.990762× | pass/pass |
| prefix-random-65537 | 65537 | 1.946709× | 1.935361× | 0.994171× | pass/pass |
| prefix-random-131072 | 131072 | 2.023447× | 2.032015× | 1.004234× | pass/pass |
| prefix-random-262144 | 262144 | 2.073695× | 2.086188× | 1.006025× | pass/pass |
| prefix-random-1048576 | 1048576 | 2.115942× | 2.106129× | 0.995362× | pass/pass |
| prefix-periodic-1024 | 1024 | 0.514374× | 0.371147× | 0.721551× | pass/pass |
| prefix-periodic-4096 | 4096 | 0.686301× | 0.575074× | 0.837933× | pass/pass |
| prefix-periodic-8192 | 8192 | 0.823459× | 0.735270× | 0.892905× | pass/pass |
| prefix-periodic-16384 | 16384 | 0.904677× | 0.851439× | 0.941152× | pass/pass |
| prefix-periodic-32767 | 32767 | 1.002965× | 0.985281× | 0.982368× | pass/pass |
| prefix-periodic-32768 | 32768 | 1.009122× | 0.984176× | 0.975280× | pass/pass |
| prefix-periodic-32769 | 32769 | 1.008133× | 0.992815× | 0.984805× | pass/pass |
| prefix-periodic-33027 | 33027 | 1.017171× | 1.001662× | 0.984753× | pass/pass |
| prefix-periodic-65535 | 65535 | 1.078197× | 1.066151× | 0.988827× | pass/pass |
| prefix-periodic-65536 | 65536 | 1.078835× | 1.072046× | 0.993708× | pass/pass |
| prefix-periodic-65537 | 65537 | 1.076307× | 1.073797× | 0.997668× | pass/pass |
| prefix-periodic-131072 | 131072 | 1.120038× | 1.122424× | 1.002131× | pass/pass |
| prefix-periodic-262144 | 262144 | 1.136144× | 1.144947× | 1.007749× | pass/pass |
| prefix-periodic-1048576 | 1048576 | 1.154643× | 1.157030× | 1.002068× | pass/pass |
| prefix-repeated-1024 | 1024 | 0.515873× | 0.308020× | 0.597085× | pass/pass |
| prefix-repeated-4096 | 4096 | 0.696136× | 0.539507× | 0.775002× | pass/pass |
| prefix-repeated-8192 | 8192 | 0.797708× | 0.667471× | 0.836736× | pass/pass |
| prefix-repeated-16384 | 16384 | 0.884573× | 0.803149× | 0.907951× | pass/pass |
| prefix-repeated-32767 | 32767 | 0.979103× | 0.942734× | 0.962854× | pass/pass |
| prefix-repeated-32768 | 32768 | 0.976002× | 0.936350× | 0.959373× | pass/pass |
| prefix-repeated-32769 | 32769 | 0.984496× | 0.963294× | 0.978465× | pass/pass |
| prefix-repeated-33027 | 33027 | 0.995875× | 0.977918× | 0.981969× | pass/pass |
| prefix-repeated-65535 | 65535 | 1.048897× | 1.041862× | 0.993293× | pass/pass |
| prefix-repeated-65536 | 65536 | 1.047417× | 1.045010× | 0.997702× | pass/pass |
| prefix-repeated-65537 | 65537 | 1.048459× | 1.046033× | 0.997686× | pass/pass |
| prefix-repeated-131072 | 131072 | 1.089269× | 1.092975× | 1.003402× | pass/pass |
| prefix-repeated-262144 | 262144 | 1.107618× | 1.115717× | 1.007312× | pass/pass |
| prefix-repeated-1048576 | 1048576 | 1.118943× | 1.122976× | 1.003604× | pass/pass |

[Verified runtime sources and complete observations](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r272-private-zero-page-scratch-352cd43a9a80.tar.gz). SHA-256 352cd43a9a80a41ea341de16286aa9ec87b4b86959f86fec701dde05a767745c.

[Libdeflate source revision, LOC and original source audit](native-libdeflate-comparison.md).
