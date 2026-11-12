# Complete level-12 comparison after compiler integration

The merged compact executable and the retained R355 executable compressed the same 81 runtime inputs in a fresh four-actor comparison with Roc and libdeflate 1.26. The 24 rotated orders retained 245,664 measured calls, including 6,830 zero timer readings. Every block mean used in a ratio was positive. Each actor warmed up for at least 200 ms per case. Timings include complete compression, scratch construction, output ownership transfer and private cleanup. Output validation and inflation ran outside the timer.

## Results

The merged-to-R355 geometric mean is 1.427894×. The rows below report paired block ratios. Values below 1.000× are faster than the named peer.

| Compiler | Faster than Roc | Faster than libdeflate | Faster than both |
| --- | ---: | ---: | ---: |
| R355 | 47/81 (58.02%) | 7/81 (8.64%) | 0/81 (0.00%) |
| Merged | 22/81 (27.16%) | 3/81 (3.70%) | 0/81 (0.00%) |

| Input | R355/Roc | Merged/Roc | R355/libdeflate | Merged/libdeflate | Merged/R355 |
| --- | ---: | ---: | ---: | ---: | ---: |
| eight-literals | 0.190937× | 0.185935× | 2.326859× | 2.265909× | 0.973806× |
| reymont-prefix-32769 | 1.187474× | 1.248135× | 1.080860× | 1.136075× | 1.051084× |
| prefix-natural-32768 | 1.191973× | 1.252080× | 1.083699× | 1.138346× | 1.050426× |
| prefix-natural-131072 | 1.205143× | 1.301038× | 0.960539× | 1.036971× | 1.079572× |
| prefix-natural-262144 | 1.214042× | 1.277660× | 0.986414× | 1.038104× | 1.052402× |
| prefix-natural-1048576 | 1.210550× | 1.282665× | 0.974787× | 1.032857× | 1.059572× |
| prefix-random-1048576 | 1.486914× | 1.703619× | 2.088940× | 2.393385× | 1.145742× |
| prefix-periodic-1048576 | 0.637654× | 1.699711× | 1.078012× | 2.873512× | 2.665567× |
| prefix-repeated-1048576 | 0.649217× | 1.608525× | 1.021393× | 2.530643× | 2.477639× |

| Peer | Frontier | Input | Merged/peer |
| --- | ---: | ---: | ---: |
| roc | nearest below | prefix-periodic-8192 | 0.939364× |
| roc | nearest at or above | prefix-natural-8192 | 1.106123× |
| libdeflate | nearest below | seven-stored-prefix | 0.022106× |
| libdeflate | nearest at or above | prefix-natural-1048576 | 1.032857× |

## Exact linked matcher

The disassembly and each emitted code-generation descriptor show this vector matcher. The instruction and header counts are static.

| Actor | Code bytes | Stack bytes | 128-bit loads | Certified widening headers |
| --- | ---: | ---: | ---: | ---: |
| mech-r355 | 11,472 | 576 | 60 | 9 |
| mech-merged | 9,760 | 560 | 0 | 0 |

## Exact linked samples

The six fresh merged profiles ran separately from the timing campaign. The sample denominator includes validation. R355 samples, where available, come from its own archived cohort. These percentages describe running-PC samples and do not estimate retired instructions.

| Input | Merged running PCs | Merged matcher | Merged composed parser | Merged other parser | R355 composed parser |
| --- | ---: | ---: | ---: | ---: | ---: |
| eight-literals | 3,871 | 14.34% | 0.00% | 0.13% | — |
| reymont-prefix-32769 | 5,337 | 36.97% | 0.00% | 52.26% | — |
| prefix-natural-131072 | 5,351 | 35.99% | 0.00% | 57.30% | — |
| prefix-natural-262144 | 5,337 | 36.11% | 0.00% | 57.20% | 55.69% |
| prefix-natural-1048576 | 5,358 | 41.17% | 0.00% | 52.65% | 51.33% |
| prefix-random-1048576 | 5,359 | 54.75% | 0.00% | 18.38% | 0.00% |

## Qualification and provenance

Merged repair commit `fddcf292c523860a9cf37e3908030718c9399646`. Compression source SHA-256 `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`. R355 used IR/work/ABI/bytecode `215/107/39/14` and the merged compiler uses `355/127/42/14`.

Original and decoded merged artifacts produced byte-identical Objects and descriptors. Both linked forms passed two complete calls for every fixture, for 324 checks. The checks preserved exact streams, independent inflation, output capacities, aliases and final cleanup. All 81 libdeflate streams in the matched campaign also agreed with the reference bytes.

| Actor | Worker SHA-256 | Descriptor or marker SHA-256 |
| --- | ---: | ---: |
| mech-r355 | `355860021f730c254a1bdf183213324eff766b2b1b310a26c8f26359315c2fdf` | `ad172fe23740bb2439b2bbaab26a0c53ff1b0bccff4906ac4c707fc60631c953` |
| mech-merged | `f1c709468b15559887cb1896acca8d1ae56db2016cb50fd04a0d85f6442c35d0` | `18b7ea3cfa6097db4d2be39ac2a8442c331e83472793f162bf4744b026171e3d` |
| roc | `ed1ab9175cd175d26d453afc26ba1fa7adb4827ec705aa1cf9bcd231ac84860c` | `b6e97f561158a9176529a2e88f47c7b9f7a3637efa8f0d2e3afc6ebb1c6ffaee` |
| libdeflate | `06e3b4b11fafb5a79ee4a44a0462682662e72f3520b94fd8dd34b6f5e9c2e1d2` | — |

The two Mech actors use their independently qualified full descriptors. Their entry symbol, input and output schemas, offsets and packet widths agree. The timed result covers the complete merged compiler and linked runtime relative to R355. Both use the compact diagnostic entry, which omits ordinary exact resource admission, accounting and recovery. The comparison does not isolate the sparse SSA repair or an allocator change.

[Checksummed raw receipts and offline replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/merged-compression-anchor-c56b4c5380e8.tar.gz).
