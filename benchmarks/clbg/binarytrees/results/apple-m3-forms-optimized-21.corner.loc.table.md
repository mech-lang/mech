| Language | Fastest source / build | Seconds | LOC | gzip bytes | Allocation | Smallest source / build | Seconds | LOC | gzip bytes | Allocation | Time tradeoff |
| --- | --- | ---: | ---: | ---: | --- | --- | ---: | ---: | ---: | --- | ---: |
| C (GCC + LLVM) | binarytrees-gcc-3-gcc | 0.882 | 74 | 828 | library-pool | binarytrees-gcc-3-gcc | 0.882 | 74 | 828 | library-pool | 1.00× |
| Julia | binarytrees-julia-3 | 3.936 | 60 | 629 | default-gc | binarytrees-julia-4 | 5.343 | 24 | 367 | default-gc | 1.36× |
| Mech (Cranelift AOT) | mech-binarytrees-parallel | 0.776 | 29 | 634 | library-pool | mech-binarytrees-parallel | 0.776 | 29 | 634 | library-pool | 1.00× |
| Rust | binarytrees-rust-5 | 0.710 | 69 | 748 | library-pool | binarytrees-rust-5 | 0.710 | 69 | 748 | library-pool | 1.00× |
