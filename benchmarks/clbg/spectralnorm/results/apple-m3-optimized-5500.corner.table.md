| Language | Fastest source / build | Seconds | LOC | gzip bytes | Smallest source / build | Seconds | LOC | gzip bytes | Time tradeoff |
| --- | --- | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| C (GCC + LLVM) | spectralnorm-gcc-3-gcc-fastmath | 0.146 | 54 | 459 | spectralnorm-gcc-8-llvm-fastmath | 0.689 | 40 | 378 | 4.72× |
| Chapel | spectralnorm-chapel-2 | 0.421 | 25 | 338 | spectralnorm-chapel-1 | 0.488 | 25 | 311 | 1.16× |
| C++ (GCC + LLVM) | spectralnorm-gpp-1-llvm-fastmath | 0.825 | 35 | 445 | spectralnorm-gpp-1-llvm-fastmath | 0.825 | 35 | 445 | 1.00× |
| Go | spectralnorm-go-2 | 0.278 | 120 | 663 | spectralnorm-go-1 | 5.344 | 53 | 407 | 19.24× |
| Java (OpenJDK JIT) | spectralnorm-graalvmaot-2 | 0.352 | 117 | 946 | spectralnorm-graalvmaot-8 | 1.489 | 42 | 442 | 4.23× |
| Julia | spectralnorm-julia-4 | 0.572 | 40 | 521 | spectralnorm-julia-2 | 0.750 | 44 | 365 | 1.31× |
| Mech (Cranelift AOT) | mech-integer-simd1-u4-r4-p8-g4 | 0.124 | 13 | 306 | mech-optimized-simd1-u4-r4-p8 | 0.163 | 13 | 300 | 1.31× |
| Rust | spectralnorm-rust-4-panicabort | 0.118 | 60 | 798 | spectralnorm-rust-4-panicabort | 0.118 | 60 | 798 | 1.00× |
