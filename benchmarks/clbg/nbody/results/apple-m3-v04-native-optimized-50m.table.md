| Language | Fastest program | Time (s) | Lines | gzip bytes | Smallest program | Time (s) | gzip bytes | Smallest runtime / fastest |
| --- | --- | ---: | ---: | ---: | --- | ---: | ---: | ---: |
| C (GCC + LLVM) | nbody-gcc-8-llvm-fastmath | 2.691 | 127 | 1181 | nbody-gcc-1-llvm-fastmath | 2.702 | 1166 | 1.004× |
| Chapel | nbody-chapel-3 | 1.703 | 79 | 972 | nbody-chapel-2 | 2.810 | 967 | 1.651× |
| C++ (g++ + LLVM) | nbody-gpp-6-llvm-native | 1.634 | 202 | 1652 | nbody-gpp-3-llvm-fastmath | 2.674 | 1391 | 1.637× |
| Go | nbody-go-3 | 2.767 | 135 | 1195 | nbody-go-3 | 2.767 | 1195 | 1.000× |
| Java (OpenJDK JIT) | nbody-graalvmaot-5 | 3.453 | 130 | 1424 | nbody-graalvmaot-8 | 3.635 | 1312 | 1.053× |
| Julia | nbody-julia-5 | 2.799 | 90 | 1106 | nbody-julia-2 | 12.278 | 1073 | 4.386× |
| Mech prototype (Cranelift SIMD AOT) (.mec only); FMA | mech-aot-simd-sqrt-v04 | 2.029 | 32 | 735 | mech-aot-simd-power-v04 | 2.029 | 732 | 1.000× |
| Rust | nbody-rust-3 | 1.956 | 196 | 1748 | nbody-rust-1 | 2.934 | 1463 | 1.500× |
