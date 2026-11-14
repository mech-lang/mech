The PR contains #973 through `69d48c4d6b` plus the selected related-post compiler. The compiler also uses the verified DEFLATE backend source for AArch64 comparison reuse and register allocator spill-copy facts across unique-predecessor forward edges. Cranelift remains the production backend; LLVM is not a required dependency.

The same source and runtime were compiled once with stock Cranelift/regalloc2 and once with the imported changes. All 31 randomized first latency rounds and seven separate resource rounds are retained. Post-JSON time includes input admission, preparation, worker creation/run/join, internal cleanup and result materialization. Restoration and JSON IO remain visible separately.

| Implementation | Post-JSON median | × fastest eligible Rust |
| --- | ---: | ---: |
| Fastest Rust, ordered masks/tables256 Apple M3 | 1.516458 ms | 1.000000 |
| Selected IR308 control | 1.704417 ms | 1.123946 |
| Rebased IR311 control | 1.697625 ms | 1.119467 |
| IR312 stock backend | 1.701959 ms | 1.122325 |
| IR312 imported backend | 1.698625 ms | 1.120127 |
| IR312 byte IDs | 1.737667 ms | 1.145872 |
| IR312 tables256 | 1.735417 ms | 1.144388 |
| IR312 byte IDs + tables256 | 1.764875 ms | 1.163814 |

The backend's median paired change versus its stock control is −0.087%, with descriptive bootstrap 95% interval [−1.281%, +0.408%]. This is inconclusive elapsed evidence. The three source variants regress by +2.156%, +2.126%, and +3.345% median paired change; their intervals are wholly adverse. They remain experiments. No unchanged adverse or inconclusive latency cohort was repeated.

The imported backend retires 31,910,672 instructions versus 32,566,078 (−655,406 / −2.013%). Resource medians are 10,825,629 versus 10,957,851 cycles. Both use 720 allocations, request 7,326,125 bytes, and peak at 5,357,407 bytes additional requested heap. Native code falls from 1,036,532 to 1,031,964 bytes; reported module stack bound rises from 1,568 to 1,584 bytes. Both linked executables occupy 17,182,896 bytes. There are 49 native numeric functions in both. The hot query function falls from 32,400 to 32,192 bytes and from 1,763 to 1,730 static stack references, retaining its 560-byte reported stack bound. Static references are not dynamic spill costs.

Fresh same-Object diagnostic profiles show the compiled numeric query (`numeric_1646`) as the largest active leaf, followed by the native UTF8 histogram and index loops. Host work-escrow locking, frame admission, and worker lifecycle remain visible. Reported nonwait/nonloader leaf samples are mostly compiled Mech code; these sample categories are diagnostic and must not be read as exact CPU or latency shares. Repeated profile invocation excludes repeated input admission/materialization; every final 5,000-post output matches Rust.

Qualification: 498 compiler contracts retain all 484 selected IR308 names, plus eight real allocator resolver tests and eight AArch64 comparison tracker tests. All 133 scalar/DEFLATE control rows match on every non-timing/resource field. Each of the five new native variants passes 32 exact-output cases and an Object-to-executable byte audit; the actual 5,000-post source also passes original/restored execution. The measured cohorts contain 899 exact latency outputs and 210 exact resource outputs.

The backend is retained for verified instruction/code savings under the relaxed acceptance policy, without claiming a new latency winner. The selected Mech source remains unchanged. Whole-process and restoration costs remain visible; they are not acceptance gates. The 1.00× goal remains unmet.

The maintained #973 code and experimental compiler bundles are different publication surfaces. All 838 files of the R293 source closure were reconstructed and checked against its manifest. The 94 changed experimental Mech compiler files have not all been imported: this update imports the checked backend changes and preserves the selected leaf/tree compiler. Subsequent pass integration must preserve those native functions, current SSA proofs, finite accounting and restored-image contracts.

`summary.json` indexes compressed raw records with content hashes. First failures and source-availability corrections are retained; no missing Rust source or receipt was invented. The final upstream rebase changes only experiment records/design documentation: all 2,384 inventoried compiler/config source files match the measured source exactly.
