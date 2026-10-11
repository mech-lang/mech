| Language | Fastest source / build | Seconds | LOC | gzip bytes | Smallest source / build | Seconds | LOC | gzip bytes | Time tradeoff |
| --- | --- | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| C (GCC + LLVM) | spectralnorm-gcc-3-gcc-fastmath | 0.126 | 54 | 459 | spectralnorm-gcc-8-llvm-fastmath | 0.683 | 40 | 378 | 5.41× |
| Chapel | spectralnorm-chapel-2 | 0.417 | 25 | 338 | spectralnorm-chapel-1 | 0.488 | 25 | 311 | 1.17× |
| C++ (GCC + LLVM) | spectralnorm-gpp-1-llvm-fastmath | 0.823 | 35 | 445 | spectralnorm-gpp-1-llvm-fastmath | 0.823 | 35 | 445 | 1.00× |
| Go | spectralnorm-go-2 | 0.267 | 120 | 663 | spectralnorm-go-1 | 5.348 | 53 | 407 | 20.07× |
| Java (OpenJDK JIT) | spectralnorm-graalvmaot-3 | 0.347 | 68 | 751 | spectralnorm-graalvmaot-8 | 1.487 | 42 | 442 | 4.28× |
| Julia | spectralnorm-julia-4 | 0.574 | 40 | 521 | spectralnorm-julia-2 | 0.757 | 44 | 365 | 1.32× |
| Mech (Cranelift AOT) | mech-optimized-simd1-u4-r4-p8 | 0.166 | 13 | 300 | mech-optimized-simd1-u4-r4-p8 | 0.166 | 13 | 300 | 1.00× |
| Rust | spectralnorm-rust-5 | 0.127 | 91 | 1038 | spectralnorm-rust-4-panicabort | 0.131 | 60 | 798 | 1.03× |
