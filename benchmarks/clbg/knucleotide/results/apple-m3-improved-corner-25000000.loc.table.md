| Language | Fastest | Seconds | LOC | gzip bytes | Smallest | Seconds | LOC | gzip bytes | Time tradeoff |
| --- | --- | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| C (GCC + LLVM) | knucleotide-gcc-1-gcc-fastmath | 1.868 | 135 | 1484 | knucleotide-gcc-1-gcc-fastmath | 1.868 | 135 | 1484 | 1.00× |
| Chapel | knucleotide-chapel-4 | 3.081 | 92 | 1057 | knucleotide-chapel-3 | 3.776 | 87 | 1026 | 1.23× |
| C++ (GCC + LLVM) | knucleotide-gpp-2-llvm-fastmath | 1.133 | 144 | 1587 | knucleotide-gpp-3-llvm-native | 1.620 | 134 | 1241 | 1.43× |
| Go | knucleotide-go-7 | 2.222 | 230 | 1580 | knucleotide-go-8 | 25.670 | 80 | 741 | 11.55× |
| Java (OpenJDK JIT) | knucleotide-graalvmaot-1 | 2.364 | 140 | 1794 | knucleotide-graalvmaot-8 | 36.741 | 64 | 775 | 15.54× |
| Julia | knucleotide-julia-6 | 2.673 | 84 | 934 | knucleotide-julia-1† | 65.701 | 74 | 682 | 24.58× |
| Mech API prototype (new helpers excluded) | mech-compact-final-std1-simd0-p4 | 0.946 | 8 | 245 | mech-compact-final-std1-simd0-p4 | 0.946 | 8 | 245 | 1.00× |
| Rust | knucleotide-rust-6-panicabort | 0.916 | 168 | 1656 | knucleotide-rust-1 | 13.714 | 92 | 1060 | 14.16× |

† Passes full benchmark output, fails official N=1000 test.
Seconds are raw batch medians, or pooled medians for identical Mech executable/policy pairs; endpoints and tradeoffs use same-batch Rust #7 ratios. See JSON for sample provenance.
