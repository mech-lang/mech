| Language | Fastest source / build | Seconds | LOC | gzip bytes | Smallest source / build | Seconds | LOC | gzip bytes | Time tradeoff |
| --- | --- | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| C (GCC + LLVM) | fannkuchredux-gcc-5-gcc | 5.106 | 86 | 900 | fannkuchredux-gcc-8-llvm | 48.407 | 56 | 469 | 9.48× |
| Chapel | fannkuchredux-chapel-2 | 7.209 | 91 | 726 | fannkuchredux-chapel-2 | 7.209 | 91 | 726 | 1.00× |
| C++ (GCC + LLVM) | fannkuchredux-gpp-1-gcc | 5.839 | 121 | 1052 | fannkuchredux-gpp-3-llvm | 40.199 | 59 | 587 | 6.88× |
| Go | fannkuchredux-go-2 | 4.578 | 127 | 892 | fannkuchredux-go-8 | 42.740 | 57 | 486 | 9.34× |
| Java (OpenJDK JIT) | fannkuchredux-graalvmaot-1 | 5.817 | 150 | 1278 | fannkuchredux-graalvmaot-8 | 39.368 | 54 | 503 | 6.77× |
| Julia | fannkuchredux-julia-2 | 4.744 | 129 | 1057 | fannkuchredux-julia-1 | 29.417 | 69 | 423 | 6.20× |
| Mech (Cranelift AOT) | mech-fannkuch-even | 2.736 | 64 | 1205 | mech-fannkuch-default | 3.874 | 43 | 853 | 1.42× |
| Rust | fannkuchredux-rust6-neon-lto | 2.461 | 135 | 1250 | fannkuchredux-rust-5 | 4.967 | 101 | 1004 | 2.02× |
