| Language | Fastest source / build | Seconds | LOC | gzip bytes | Smallest source / build | Seconds | LOC | gzip bytes | Time tradeoff |
| --- | --- | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| C (GCC + LLVM) | spectralnorm-gcc-3-gcc-fastmath | 0.146 | 54 | 459 | spectralnorm-gcc-8-llvm-fastmath | 0.689 | 40 | 378 | 4.72× |
| Chapel | spectralnorm-chapel-2 | 0.421 | 25 | 338 | spectralnorm-chapel-2 | 0.421 | 25 | 338 | 1.00× |
| C++ (GCC + LLVM) | spectralnorm-gpp-1-llvm-fastmath | 0.825 | 35 | 445 | spectralnorm-gpp-1-llvm-fastmath | 0.825 | 35 | 445 | 1.00× |
| Go | spectralnorm-go-2 | 0.278 | 120 | 663 | spectralnorm-go-8 | 3.921 | 44 | 418 | 14.11× |
| Java (OpenJDK JIT) | spectralnorm-graalvmaot-2 | 0.352 | 117 | 946 | spectralnorm-graalvmaot-8 | 1.489 | 42 | 442 | 4.23× |
| Julia | spectralnorm-julia-4 | 0.572 | 40 | 521 | spectralnorm-julia-4 | 0.572 | 40 | 521 | 1.00× |
| Mech (Cranelift AOT) | mech-integer-simd1-u4-r4-p8-g4 | 0.124 | 13 | 306 | mech-integer-simd1-u4-r4-p8-g4 | 0.124 | 13 | 306 | 1.00× |
| Rust | spectralnorm-rust-4-panicabort | 0.118 | 60 | 798 | spectralnorm-rust-4-panicabort | 0.118 | 60 | 798 | 1.00× |
