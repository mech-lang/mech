# Complete-program block-choice qualification

Base: `566dfd1df2eb81eb95e2804d6b50a6334bdaca42` (#968). Compiler: `115188e17032e26341dd7983e0634589a9a490ab` (#967). Compression source remains 160992 bytes, SHA256 `345324ce551aaa2485dcd0fe4262da96e660d6b31f5f772bff89b034e9b1997f`; its bytecode is identical to the previously qualified and currently running full-input program.

The original seven complete-program cases did not cover a dynamically generated Huffman block or a stored block selected after match collection. Add two inputs without changing those seven cases or their expected values:

- 128 deterministic biased bytes produce a 65-byte dynamic block (BTYPE 2).
- 32 distinct high-valued bytes produce a 37-byte stored block (BTYPE 0), after taking the optimal entry because length exceeds seven.

The new golden streams come from the pinned reference source `671077457ff7ef3a3d44caf715a3fc595addf379` and compiler `d99c7b838fb05bb1047eb0c18d4d06d0f86f345d`. The preserved producer wrapper, binary/source/input hashes and build receipts establish provenance; the fixture identifies the supplemental reference group. Python zlib independently inflated both producer streams before fixture adoption. The maintained verifier additionally checks the first block kind when specified.

All nine cases pass in source-derived original and decoded forms: eighteen complete native evaluations, exact raw bytes, miniz_oxide decompression, aliases and complete managed cleanup. The dynamic case took 22.032386084 seconds original and 21.890540666 seconds decoded. The stored-selection case took 6.525426583 and 6.535923125 seconds. These are preliminary execution investigations with the previously disclosed caller setup/System allocator, not final fair benchmark comparisons.

No compressor algorithm, table, search limit, buffer size, allocation site, timing boundary or production compiler code changes. All configurable budgets remain at maximum via `NativeProgramLimitMode::NoLimits`. The full-file run remains active and incomplete. Multi-block/full-input correctness and matched final benchmark timing remain unverified.

The initial producer build failed to write its normal cache under the local filesystem restriction. The exact same source and compiler succeeded with normal cache access, with zero errors and zero warnings. This was environmental; both receipts are preserved. No new compiler gap was exposed by these cases.

The maintained Rust host test binary is also compiled with `canonical-native-program`; unoptimized execution of the complete program is not repeated. The focused release runner invokes the maintained helper; eighteen evaluations are not reported as eighteen libtest tests. Exact commands, features and elapsed times are retained. CI is cancelled at user request; drafts remain drafts.

The resident routing, source catalog, native host catalog, native analysis layout, CI inventory, formatting and diff checks passed in `level12-choice-static` (17.795346260 seconds). Before publication, the original seven fixture case objects and the hashes of both validated test files were checked again. The archived producer inputs match the fixture's supplemental input hash.
