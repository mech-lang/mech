| Language | Fastest | Time / same-batch Rust #4 | LOC | gzip bytes | Smallest | Time / same-batch Rust #4 | LOC | gzip bytes | Time tradeoff |
| --- | --- | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| C (GCC + LLVM) | mandelbrot-gcc-9-llvm-fastmath | 1.338 | 49 | 686 | mandelbrot-gcc-8-gcc-fastmath | 29.276 | 43 | 398 | 21.88× |
| Chapel | mandelbrot-chapel-3 | 3.226 | 47 | 571 | mandelbrot-chapel-1 | 7.466 | 38 | 436 | 2.31× |
| C++ (GCC + LLVM) | mandelbrot-gpp-0-gcc-fastmath | 1.274 | 308 | 2708 | mandelbrot-gpp-3-gcc-fastmath | 28.285 | 25 | 412 | 22.20× |
| Go | mandelbrot-go-4-nofma | 3.519 | 102 | 896 | mandelbrot-go-8 | 30.543 | 51 | 486 | 8.68× |
| Java (OpenJDK JIT) | mandelbrot-graalvmaot-6 | 3.024 | 57 | 798 | mandelbrot-graalvmaot-8 | 33.789 | 43 | 426 | 11.17× |
| Julia | mandelbrot-julia-7 | 1.429 | 59 | 751 | mandelbrot-julia-1 | 32.972 | 33 | 368 | 23.07× |
| Mech (Cranelift AOT; complete source) | mech-program-simd-loop-fma | 1.035 | 29 | 456 | mech-program-compact | 5.050 | 20 | 289 | 4.88× |
| Rust | mandelbrot-rust-4 | 1.000 | 146 | 1277 | mandelbrot-rust-5 | 1.360 | 59 | 710 | 1.36× |
