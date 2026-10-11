| Language | Fastest | Seconds | LOC | gzip bytes | Smallest | Seconds | LOC | gzip bytes | Time tradeoff |
| --- | --- | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| C (GCC + LLVM) | mandelbrot-gcc-9-llvm-fastmath | 4.373 | 49 | 686 | mandelbrot-gcc-9-llvm-fastmath | 4.373 | 49 | 686 | 1.00× |
| Chapel | mandelbrot-chapel-3 | 7.252 | 47 | 571 | mandelbrot-chapel-1 | 17.714 | 38 | 436 | 2.44× |
| C++ (GCC + LLVM) | mandelbrot-gpp-9-llvm-fastmath-nofma | 6.692 | 74 | 715 | mandelbrot-gpp-9-llvm-fastmath-nofma | 6.692 | 74 | 715 | 1.00× |
| Java (OpenJDK JIT) | mandelbrot-graalvmaot-3 | 19.137 | 87 | 895 | mandelbrot-graalvmaot-3 | 19.137 | 87 | 895 | 1.00× |
| Julia | mandelbrot-julia-7 | 4.305 | 59 | 751 | mandelbrot-julia-7 | 4.305 | 59 | 751 | 1.00× |
| Mech (Cranelift AOT; .mec only) | mech-unrolled-simd-loop-fma | 4.443 | 26 | 321 | mech-compact | 26.501 | 17 | 197 | 5.97× |
| Rust | mandelbrot-rust-4 | 3.916 | 146 | 1277 | mandelbrot-rust-4 | 3.916 | 146 | 1277 | 1.00× |
