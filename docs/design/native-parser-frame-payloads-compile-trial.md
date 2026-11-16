# Parser continuation payload-storage trial

The candidate is **rejected and restored** after a direct comparison with the fastest qualified compiler. Complete builds measure **13.275817 to 13.478050 seconds (+1.523%)**, with both native blocks slower. The unchanged optimized compiler `8043` is retained. Its original primary result is 13.414825 seconds; 13.275817 seconds is the control repeat in this trial. PGO fully counts toward the compilation goal; compiler settings are recorded for reproduction.

## Frozen change and parser behavior

Base is `25615f1d575fd5cb7c91d4dcd3d8d01edf9e3ef3`; whole one-file patch SHA-256 is `83150caf57750bfb169986010eef403e5f7a4cd1c9b0e2c7318b3a9e98bf2a68`. In canonical base continuations, Exit/Unless checkpoints move to a paired typed LIFO vector at their original capture and restore phases. Unicode escape state uses owned boxes carried through the same phases, with one reusable allocation whose fields are freshly overwritten. Ordinary frames shrink **80 to 32 bytes** on this target. This changes storage only: original grammar, phase order, prediction ownership, logical work/fuel, first refusal and identity allocation remain.

**528 actual complete document-module unit tests** and **1,976 exact reference/candidate parser cases** pass. Those include the complete Deflate program and 308 streaming cases covering 22 texts, seven allowance sizes and two chunk schemes. Complete trees, node/token IDs, diagnostics, statistics, restart state, progress and work/fuel accounting match across 20,376,437 bytes per subject. The independent token audit reconstructs the entire original production algorithm after reversing explicit storage changes. Genuine compiler check/build and the native CLI check with `cranelift_build` pass.

## Complete-build and Object observations

| Boundary | Original | Candidate | Change |
| --- | ---: | ---: | ---: |
| Standard complete source to executable | 14.908641 | 15.048180 | +0.9360% |
| Standard Object internal compiler | 14.754359 | 14.757569 | +0.0218% |
| Standard Object parser | 1.872901 | 1.776752 | -5.1337% |
| Standard Object instructions | 220,473,215,631.25 | 218,844,948,210.75 | -0.7385% |
| Standard Object maximum RSS | 4,487,843,840 | 4,488,114,176 | +0.0060% |
| Optimized complete source to executable | 13.275817 | 13.478050 | +1.5233% |
| Optimized Object internal compiler | 13.116851 | 13.182979 | +0.5041% |
| Optimized Object parser | 1.514484 | 1.503812 | -0.7047% |
| Optimized Object instructions | 191,224,960,318 | 190,487,643,256.5 | -0.3856% |
| Optimized Object maximum RSS | 4,491,010,048 | 4,490,797,056 | -0.0047% |

Times are seconds; instructions and maximum RSS are raw counts and bytes. Each primary cohort has eight serial ABBA/BAAB observations, four per subject. Standard and optimized cohorts remain separate. The initial standard criterion failed: complete builds slow 0.936% despite parser time falling 5.134% and retired instructions falling 0.739%. Before any fresh optimized timing, an independent follow-up plan required a direct fastest-configuration gain in both full-native and Object means and both balanced blocks. No parser reduction was added to an earlier complete-build result.

Standard native block changes: +1.544%, +0.328%. Optimized native block changes: +0.933%, +2.117%. Optimized Object compiler block changes: +1.089%, -0.078%. All favorable and adverse phases, blocks, counters and heldout observations are in the [complete result](../../benchmarks/mech-deflate/native-profile/parser-frame-payloads-apple-m3-20261115.json).

Standard adverse Object fields: artifact_seconds +0.427%, catalog_seconds +0.528%, cleanup_including_compaction_seconds +0.820%, cleanup_seconds +0.637%, compaction_seconds +4.420%, compiler_seconds +0.022%, encode_seconds +1.703%, frontend_seconds +0.915%, max_rss_bytes +0.006%, native_seconds +0.754%, peak_footprint_bytes +0.006%, preparation_seconds +2.745%, source_cfg_seconds +0.231%, whole_process_seconds +0.016%. Optimized adverse Object fields: artifact_seconds +0.800%, catalog_seconds +1.689%, cleanup_including_compaction_seconds +1.202%, cleanup_seconds +1.261%, compaction_seconds +0.072%, compiler_seconds +0.504%, encode_seconds +0.822%, frontend_seconds +1.021%, native_seconds +0.609%, source_cfg_seconds +1.395%, whole_process_seconds +0.509%. Optimized adverse complete-build fields: source_to_executable_seconds +1.523%, source_to_object_process_seconds +1.546%.

Both optimized subjects use nativeCPU, ThinLTO/CGU1, pinned compiler-only mimalloc, nightly2026-03-03, explicit aarch64 target, four Cargo jobs, incremental0 and -Dwarnings. The candidate uses its own fresh instrumentation profile from the unchanged predetermined eight-program corpus, including Deflate. Seven predetermined scalar/numeric heldouts retain every observation. All builds/training finish before primary timing; no heavy build/test/profile/hash/archive work overlaps it. Every cohort uses the same 61 program flags and two workers. Complete builds start fresh source assembly and end after native link completion, with fixed runtime/adapter inputs cached and pinned. Historical first-use adapter construction remains separate.

## Exact outputs and published evidence

Every source, MBC, Object and **complete metadata** byte matches in all main observations and both sets of 14 fixture pairs. Standard and optimized default-serial and real-worker diagnostics pass, including 344 jobs and 48 cleanup-and-machine jobs. All **32 qualification executable images**, plus the two selection-profile images, reproduce all 2,060,440 bytes of the current executable, SHA-256 `d6c65cc3fb18acc4be77dce329962105bf0f2b56dd2623a12f466256463eca61`. Emitted program code and runtime inputs are unchanged.

Two fresh 1ms stack diagnostics of the current fastest compiler selected the frame storage target. Their full raw samples, validated demangling, thread/ancestry counts and actual-symbol disassembly are retained separately from primary timing. Counts are callgraph observations rather than exclusive CPU percentages. All setup failures remain. The optimized fixture setup initially omitted two unchanged template files and stopped before any fixture compiler invocation or primary cohort; four completed gates remain intact and the remaining four stages resume with identical helpers after copying the pinned templates. Pure replay checks the preserved failure, exact prefix, recovered bytes and actual resume AST. The inherited test recipe's historical link-failure wording is explicitly distinguished from this trial's successful tests.

The first packet replay also found a missing `qualify.py` alias. The exact frozen qualification driver was already included under its full name; adding an identical copy fixes replay without changing or repeating any benchmark observation. That failed replay and the alias repair are retained in the packet.

The exact original parser source and canonical standard compiler are restored. Cargo dependency libraries still reflect the rejected candidate until a genuine rebuild; the restored parser source has a fresh mtime. Qualified compiler `8043` and profile `8691` are unchanged. Both candidate binaries and all candidate profiles remain preserved outside the checkout.

The fifty-first [repository packet](../../benchmarks/mech-deflate/evidence/compiler-profile-20261114/README.md) includes the full source snapshots/patch, tests/audit, every command/log/observation/output/executable, both compiler configurations, all fresh raw/merged profiles in verified lossless storage, fixed native inputs, plans/derivation and restoration proof. Compiler/test binaries and dependency targets are excluded and hash-pinned. Fresh repository extraction verifies every manifest/blob and replays the full nested result byte-identically. Compilation below ten seconds remains open.
