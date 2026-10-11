| Language | Fastest source / build | Seconds | LOC | gzip bytes | Allocation | Smallest source / build | Seconds | LOC | gzip bytes | Allocation | Time tradeoff |
| --- | --- | ---: | ---: | ---: | --- | --- | ---: | ---: | ---: | --- | ---: |
| C (GCC + LLVM) | binarytrees-gcc-3-gcc | 0.834 | 74 | 828 | library-pool | binarytrees-gcc-1-gcc | 12.993 | 90 | 650 | individual | 15.59× |
| Chapel | binarytrees-chapel-5 | 1.530 | 50 | 635 | library-pool | binarytrees-chapel-4 | 6.473 | 42 | 479 | individual | 4.23× |
| C++ (GCC + LLVM) | binarytrees-gpp-7-gcc | 0.726 | 70 | 881 | library-pool | binarytrees-gpp-2-gcc | 15.282 | 45 | 504 | individual | 21.05× |
| Go | binarytrees-go-2 | 5.414 | 83 | 659 | default-gc | binarytrees-go-1 | 9.279 | 44 | 474 | default-gc | 1.71× |
| Java (OpenJDK JIT) | binarytrees-graalvmaot-7 | 1.479 | 63 | 831 | default-gc | binarytrees-graalvmaot-6 | 2.776 | 42 | 525 | default-gc | 1.88× |
| Julia | binarytrees-julia-3 | 4.002 | 60 | 629 | default-gc | binarytrees-julia-4 | 5.343 | 24 | 367 | default-gc | 1.33× |
| Mech (Cranelift AOT) | mech-binarytrees-parallel | 0.724 | 50 | 826 | library-pool | mech-binarytrees-compact | 3.154 | 50 | 785 | library-pool | 4.36× |
| Rust | binarytrees-rust-5 | 0.712 | 69 | 748 | library-pool | binarytrees-rust-2-thin | 0.734 | 56 | 710 | library-pool | 1.03× |
