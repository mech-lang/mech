| Language | Fastest source / build | Seconds | LOC | gzip bytes | Smallest source / build | Seconds | LOC | gzip bytes | Time tradeoff |
| --- | --- | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| C (GCC + LLVM) | revcomp-gcc-9-llvm | 0.138 | 143 | 1523 | revcomp-gcc-5-llvm | 0.363 | 54 | 633 | 2.64× |
| Chapel | revcomp-chapel-9 | 0.299 | 107 | 1242 | revcomp-chapel-5 | 1.865 | 44 | 536 | 6.24× |
| C++ (GCC + LLVM) | revcomp-gpp-3-gcc | 0.920 | 108 | 801 | revcomp-gpp-1-gcc | 1.992 | 48 | 564 | 2.16× |
| Go | revcomp-go-6 | 0.207 | 150 | 1273 | revcomp-go-2 | 0.327 | 59 | 541 | 1.58× |
| Java (OpenJDK JIT) | revcomp-graalvmaot-3 | 0.408 | 195 | 1653 | revcomp-graalvmaot-4 | 0.847 | 57 | 587 | 2.08× |
| Julia | revcomp-julia-9 | 0.466 | 36 | 432 | revcomp-julia-9 | 0.466 | 36 | 432 | 1.00× |
| Mech (Cranelift AOT) | mech-revcomp-optimized | 0.218 | 10 | 301 | mech-revcomp-optimized | 0.218 | 10 | 301 | 1.00× |
| Rust | revcomp-rust-1-panic-abort | 0.164 | 257 | 2729 | revcomp-rust-3 | 0.184 | 102 | 1076 | 1.12× |
