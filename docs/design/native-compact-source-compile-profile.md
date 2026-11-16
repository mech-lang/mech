# Compact Deflate: complete source-to-executable compilation

The supplied **2,719-line, 243,209-byte** Deflate program builds to a runnable executable in **6.027732 seconds** on the Apple M3. This includes parsing, checked analysis, canonical bytecode, native support generation, LLVM optimization and code generation, assembly, Object composition and final executable linking. The fixed runtime and generic entry adapter are cached, consistently with the earlier complete-build measurements. Building the compiler itself is separate setup.

The exact **55,956-byte** machine-code text is reproduced from source: **0.970× Roc** and **1.361× libdeflate**. Object SHA-256 is `a917aa339eb7291e0cbcb5fadedd2a4bf263df0db0ffe408f96310fa084b1f14`. All eight final complete executable images match a reference linked from the supplied Object with the same pinned runtime and entry adapter. The native code and execution setup are preserved.

## Controlled compiler comparison

Both subjects read the identical supplied `.mec` source and use the same final native emitter, LLVM 22.1.8, zero compiler workers, runtime, adapter and linker inputs. The parser frontend is the changed compiler component. Each frontend uses native CPU tuning, compiler ThinLTO/one codegen unit, mimalloc and its own PGO from the same fixed eight-program corpus. That corpus contains the earlier Deflate program; the new 243 KB source is absent. These compiler settings fully count toward the result.

| Complete build boundary | Prior parser | Retained parser |
| --- | ---: | ---: |
| Source to runnable executable | 6.379101 s | 6.027732 s |
| Source frontend process | 2.891907 s | 2.537635 s |
| Fresh native/LLVM recipe | 3.350494 s | 3.360831 s |
| Object handoff | 0.000498 s | 0.000496 s |
| Final executable link | 0.136202 s | 0.128771 s |

Four fresh builds per subject use serial ABBA/BAAB order after all heavy setup, builds, tests and diagnostics finish. The complete-build reduction is **5.51%**. Both balanced blocks improve: **4.99%** and **6.01%**. The native recipe slightly increases on average; every raw value and adverse field is retained. The timer starts before launching the source-reading compiler and stops after the final executable linker exits. Each observation generates new MBC, prepared Object, LLVM IR and final Object.

The frontend and emitter are separate compiler executables joined by freshly encoded canonical bytecode. The frontend is `b03652481153c45f58e84d2ccd55fdb8ff06b414` plus frozen parser patch `d1ea12dc33ec16ed12dc20d5b3bf8caa6dec629150ae549cabf37201d5fae476`, binary `fff62585955d94a490d58daec4bd11a448f973cc4fe90fd46965f01aee296075`. The control is qualified binary `8043d93d27b948371aee38027df898a9024e47a1ffa112b47a1913f8329e40ad`. The merged emitter and parser source are committed as `4e8fd9351a9e6219c5cf2150a08f97f70362e815`; the packet verifies every measured changed file against that commit.

## Why the earlier number was about 14 seconds

The earlier **13.414825-second** complete build compiles a different, 164,243-byte source snapshot. Its canonical bytecode is 4,733,696 bytes and the previous backend emits 1,277,188 bytes of machine-code text. The new source produces 1,034,027 bytes of bytecode and 181,252 bytes of machine code with that previous backend. Longer source text therefore causes substantially less downstream compiler work. A 13-to-6-second comparison cannot be credited entirely to compiler changes.

A separate matched new-source experiment with the previous backend measures **4.006400 → 3.643487 seconds** from source through final executable linking, a 9.06% reduction. That result emits 181,252 bytes of machine code. Its first source-only screen has adverse process movement and is retained. It is distinct from the complete small-Object recipe above.

The supplied small build uses LLVM for the checked root and matcher while Cranelift supplies support definitions. The roughly four-second runs already used Cranelift. LLVM explains part of the smaller final output, rather than the large cross-source reduction in compilation time. The complete small-build timing includes LLVM.

## Where the final build spends time

| Work | Prior parser | Retained parser |
| --- | ---: | ---: |
| Complete parsing/tree construction | 2.678168 s | 2.325172 s |
| Checked source frontend | 0.147119 s | 0.147094 s |
| Compact native emitter process | 1.419606 s | 1.426286 s |
| LLVM IR linking | 0.107263 s | 0.109173 s |
| LLVM root optimization | 0.504272 s | 0.506683 s |
| LLVM matcher optimization | 0.066917 s | 0.067781 s |
| LLVM root machine generation | 0.630132 s | 0.629468 s |
| LLVM assembly | 0.058500 s | 0.058227 s |
| LLVM matcher machine generation | 0.057393 s | 0.056694 s |

These are measured wall intervals at different nesting levels, not exclusive CPU percentages. Parsing remains about 38.6% of the complete build and improves 13.18%. Remaining opportunities are parser journal/tree traversal and native/LLVM preparation. Independent root/matcher optimization and generation are candidates for future overlap; no gain from that overlap is measured here.

## Integration, validation and runtime scope

The merge integrates exact [#992 source `b2729eb666`](https://github.com/mech-lang/mech/tree/b2729eb6660a81a91f77f8c3bb2b9d1d66590220) with the compilation-speed work and the six frozen parser files. The parser reuses bounded flat ASCII recognition derived from existing grammar specifications, batches journal pages and releases completed cache owners before publication. All limits, identities and fallback behavior remain checked; no grammar or language change is made. The earlier [standalone trials](native-parser-lexical-reuse-trials.md) remain separate evidence.

Final validation passes **529 parser tests, 17 supplied-runtime tests and six focused compiler tests**, including ordered worker publication/errors, typed cleanup, direct relocation/body sharing with JIT execution, and word-pair checked accesses. Eleven maintained complete-stream cases pass exact golden output, independent inflation, borrowed-alias preservation, capacity checks and zero nonreturned payload at return: 22 explicit calls plus checked startup warmups. All complete source, MBC, Object and metadata gates pass.

Default-zero and two-worker build requests reproduce every artifact and whole executable byte. All 14 numeric phase families export selected LLVM IR, so their coordinator path leaves **zero eligible worker jobs**. The existing parallel implementation remains available for other compact workloads; the exact small-Object recipe currently executes these phases serially. A nonzero-job assertion failed during the diagnostic, and the actual zero-job result is retained. No parallel speedup is claimed.

The merge initially retained the older branch's 64-page mapping threshold. Final validation uses #992's three runtime source files byte for byte, including its four-page threshold. Earlier timings with the 64-page runtime remain a separate cohort: 6.398558 → 6.041040 seconds. Its 5.899092-second pilot is separate and excluded from the final mean.

The supplied [M1 handoff](https://github.com/mech-lang/mech/tree/39e2a9303e933f7de6e4c4f41ce49b1fcc7a84fe) reports **81/81 Roc wins and 62/81 libdeflate wins in both orders**; **63** is its next target. This work does not claim a fresh M3 comparator timing campaign. Exact Object reproduction and the same supplied runtime preserve the generated program. The handoff also records 18 cases more than 5% slower than its own prior Mech control in both orders; the original all-1.05 gate is false. Compiler acceleration does not remove those existing runtime results.

This is the supplied fixed compact diagnostic benchmark recipe. Ordinary production admission/publication recovery remains outside that benchmark contract, as in the preceding measurements. The recipe validates freshly generated prepared metadata, checked LLVM IR and final Object identity, then uses its recorded final machine facts for the final metadata file. It does not regenerate arbitrary-program final metadata. Fixed runtime/entry construction is excluded consistently; the fresh final adapter construction is recorded separately.

## Reproduction and retained evidence

The [compact recipe](../../benchmarks/mech-deflate/native-profile/compact-size/README.md) documents rebuilding the exact Object. For the timed source path, invoke `compile_profile SOURCE OUT/frontend frontend`, then pass its newly produced `program.mecb` to `rebuild.py --emitter EMITTER --program OUT/frontend/program.mecb`; finish with the captured native executable link. The raw packet contains the full source-to-executable driver, runtime/adapter source, every fixed native link input, executable images, per-invocation commands, complete phase logs and the matching compiler construction commands. Building compiler tools or timing only the Object recipe is not the reported complete-build boundary.

The fifty-fifth [repository packet](../../benchmarks/mech-deflate/evidence/compiler-profile-20261114/README.md) includes all new-source/backend/runtime cohorts, all successful and failed preflights, every final executable, full source patches/snapshots, all raw/merged PGO profile bytes, and numerical replay. It preserves the missing-runtime-symbol linkage failure, the first nonexecutable smoke wrapper, the interrupted profile-use construction and its resume, the unused-import merge build failure, the zero-job assertion, and the golden harness's too-small prepared-capacity assumption. Completed successful observations are not repeated to repair setup or postprocessing.

The original-source broad PGO qualification was prepared but never run; it is retained as an unrun plan. This packet qualifies the supplied new source. Compiler/test executables are excluded and hash-pinned, while program executable images and all fixed native link inputs remain. Fresh repository-blob extraction, manifest/profile verification and pure offline replay reproduce the compact [machine-readable summary](../../benchmarks/mech-deflate/native-profile/compact-source-compile-apple-m3-20261116.json) byte for byte without invoking a compiler or compressor.
