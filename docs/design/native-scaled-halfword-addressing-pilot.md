# ARM64 scaled halfword addressing pilot

The ARM64 selector candidate folds an I64 self-add into the scaled address of a 16-bit load or store. Both workers share the a101 runtime and allocator. The compiler change is isolated to the vendored instruction selector. The level-12 benchmark uses the pinned source and fixtures.

Both original-form workers passed all 162 output, capacity, alias, cleanup, and inflate checks. The focused controls passed on both selectors, and the expanded candidate memory test passed 171 inputs covering wrapping, shared indices, offsets, signed loads, and width refusal. The [replay archive](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-d4d35e07-integration-and-addressing-pilot.tar.gz) includes the patch, hashes, complete timing observations, separate counter cohort, and the linked profiles.

| Case | Mech/Roc | Mech/libdeflate | Elapsed vs control | Instructions vs control |
| --- | ---: | ---: | ---: | ---: |
| Eight literals | 0.172824× | 2.217171× | 1.008307× | 0.999923× |
| Natural 256 KiB | 1.183126× | 0.954874× | 0.998041× | 0.997046× |
| Random 1 MiB | 1.420742× | 2.010529× | 1.005152× | 1.003751× |
| Repeated 256 KiB | 0.618161× | 1.008228× | 0.997622× | 0.996062× |
| Repeated 1 MiB | 0.615754× | 0.967438× | 1.001726× | 0.996023× |

The pilot recorded 6,528 measured calls in 24 balanced four-actor blocks per case. Its elapsed geometric mean was **1.002162× control**. Private payload metrics matched in every case. The independent cycle cohort was mixed, including 0.979× control on eight literals and 1.00045× on repeated 256 KiB. The published elapsed comparison uses the primary cohort.

Five of 351 generated function bodies changed. The linked matcher lost 12 address-doubling instructions and shrank from 10,616 to 10,568 bytes. It retains a 576-byte frame, all 35 halfword accesses, and all 60 widened loads. The root grew by 20 code bytes and 16 stack bytes. A sampled inline equality route grew from 49 to 51 instructions per reached iteration because the live length was spilled and reloaded. The static counts of direct stack-relative loads and stores rose by 110 and 32. Those static counts and the separate retired-instruction cohort are retained in the archive.

The pilot is rejected at this checkpoint. Repeated 256 KiB gained 0.24% in the paired elapsed comparison and remained above 1× libdeflate. The smaller matcher also came with root spills and adverse timing on other cases. Qualification stops at the original-form workers and the five measured cases.
