# Grouped histogram qualification

The phase-local grouped tally takes **0.959313× control time** across the 81-case level-12 matrix. It wins five additional cases against Roc, moving from **47/81 to 52/81 (64.2%)**. Both 1.05 timing gates pass for all 81 cases.

| Input | Candidate / control | Control / Roc | Candidate / Roc | Control / libdeflate | Candidate / libdeflate |
| --- | ---: | ---: | ---: | ---: | ---: |
| Eight literals | 0.996110× | 0.175693× | 0.175010× | 2.183014× | 2.174522× |
| Random 16 KiB | 0.804956× | 0.949057× | 0.763949× | 2.561976× | 2.062278× |
| Random 32 KiB | 0.804828× | 1.145151× | 0.921649× | 2.337709× | 1.881453× |
| Random 1 MiB | **0.826764×** | 1.409734× | **1.165518×** | 1.996930× | **1.650990×** |
| Natural 8 KiB | 0.999305× | 1.029330× | 1.028615× | 1.279034× | 1.278145× |
| Natural 256 KiB | 0.998965× | 1.171474× | 1.170262× | 0.950248× | 0.949264× |
| Natural 1 MiB | 1.000201× | 1.178759× | 1.178996× | 0.950885× | 0.951076× |
| Repeated 65,537 bytes | 1.003182× | 0.566091× | 0.567892× | 0.999273× | 1.002452× |
| Periodic 1 MiB | 1.000095× | 0.579859× | 0.579914× | 0.984253× | 0.984346× |

The comparison has 24 balanced order blocks per case and 245,664 timed calls. Sixty-two paired estimates improve and nineteen rise. Periodic 4 KiB has the largest increase at 1.029859× control. The five Roc crossings are the random prefixes of 32,767, 32,768, 32,769 and 33,027 bytes and the separate 32,769-byte random fixture. Repeated 65,537 bytes crosses above libdeflate in the same cohort. The libdeflate score is 16/81, with seven faster than both peers and 61 faster than either. The nine-case shared-descriptor screen also placed repeated 65,537 bytes just above parity at 1.001746× libdeflate.

## Startup control

The earlier default-descriptor [full pilot](native-grouped-histogram-pilot.md) and [phase-local placement trial](native-grouped-histogram-local.md) record real small-case timing gate misses. The two Mech descriptors differ only in their `numeric_phases` diagnostics, adding 1,659 bytes. The worker parses that JSON at startup. Its execution consumes the symbol, input layout and input/output word counts. A crossover kept each executable and its complete argv fixed and varied only which descriptor the working directory supplied. Reading the larger descriptor increased empty-case elapsed by 13.15% on the control and 8.66% on the candidate, and increased the one-byte and seven-byte cases on both executables. The eight-literal differences were +0.26% and −0.09%.

The full comparison uses the [shared startup procedure](native-shared-benchmark-metadata.md). Both Mech actors receive the same descriptor after verifying the exact packet ABI and qualifying both executable and descriptor combinations. The peer launches, timers, allocator, warmups, validation placement and output ownership match the prior comparison. Empty, one byte and seven bytes measured 0.992304×, 0.989968× and 0.967291× their controls by paired block ratios. Raw blocks retain all preceding gate misses and the crossover.

## Compiler and qualification

`MECH_GROUP_PACKED_HISTOGRAM=1` selects the transformation in the compact compiler. The typed phase proves a read-only packed-node view, a writable count view, the exact literal predicate, a single wrapping count increment, and a unit-stride return to the same loop header. The callee checks the current initialized extents and their disjoint byte ranges before entering the grouped body. Each group checks the remaining cursor range and all four literal predicates before writing. Updates follow source order, including duplicate keys. The original checked route handles refusal and the tail. The program, revision-14 bytecode, scratch shapes and allocation sites are shared by the control and candidate. The six observed private-memory metrics agree for each of the 81 cases.

The exact compiler patch has SHA-256 `e87f38a24acd1a52e1fe6e782211fe928b00635466c366d65842c973bd8c7d71`. The measured base is `467ef97a25a70c1e919d84d1ef408d50d616e143`. The nightly 2026-03-03 optimized build uses `aot,canonical-native,canonical-native-program,canonical-native-program-runtime,canonical-native-runtime,jit` with warnings as errors. The pinned source and bytecode hashes begin `331e659e9218` and `51de51658c1d`. The original and decoded builds produce the same Object, descriptor and linked worker, with hashes beginning `03f7edb2ba2e`, `8561a32a6d82` and `cb720a5c0244`. Their exact and shared-descriptor runs passed 972 complete linked checks across six launches, with exact output, independent inflate, capacity, held aliases, and zero private bytes and owners at cleanup. The focused JIT run passed 51 differential or extent controls and nine structural refusals. The managed post-JSON guard passed 88 exact outputs and measured 0.719931× and 0.733000× Rust with matching generated bundle hashes.

| Separate random 1 MiB counters | Control | Candidate | Candidate / control |
| --- | ---: | ---: | ---: |
| Retired instructions | 481,966,635 | 458,777,144 | 0.951886× |
| Cycles | 130,295,320 | 107,291,891 | 0.823452× |
| Lifetime peak physical bytes | 50,938,816 | 50,987,968 | 1.000965× |

The stored inputs retire equal instructions in the shared-descriptor counter cohort. The empty counter run has 6.8% more cycles and the seven-byte counter run 1.6% more. They are independently instrumented single-call blocks. The exact-worker random sample contains 3,820 matcher PCs and 252 grouped-tally PCs out of 5,365, including validation. The counts locate remaining work.

Changed-file formatting, compiler-planning quarantine, native host catalog, and resident routing pass. The full formatting check flags the inherited `checked_increment.rs` formatting. The repository warning-policy and unsafe audits flag the inherited lint in `native_tagged_map_allocations.rs` and unlisted probe in `benchmarks/related-post-gen/check-measurement.rs`. Those files and the audit inventories are byte-identical to the parent.

[Patch, exact pins, raw timing and counter blocks, profile receipts and replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-467ef97a25-grouped-histogram-qualified.tar.gz). SHA-256 `3b37dde645a98465ed004f064298dcf2740ccaa39cdcd00f84d3565434835e72`. Run `python3 -B verify.py` in the unpacked archive.
