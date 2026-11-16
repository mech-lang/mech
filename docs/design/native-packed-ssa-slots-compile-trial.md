# Packed dense SSA-slot trial

The packed representation is **rejected and restored**. Complete builds improve **14.948667 to 14.891670 seconds (-0.381%)**, but the separate Object compiler cohort slows **+0.400%**, with both Object blocks slower. This does not establish a consistent compilation-time gain across the two cohorts. The best qualified complete-build result remains **13.414825 seconds** on the [current optimized compiler](native-dimension-pgo-compile-profile.md).

## Frozen change and exact behavior

Base is `0ab8308a795a11c919d0aac36e42920b4001ed2b`; the whole one-file patch is SHA-256 `beffbe05a6afbd867c8704fbbaacc69a4750f98ec669cef1d330eb2f66b932d8`. Private dense SSA slots originally contain `Option<PackedOption<Value>>`. The prototype stores the packed value and a presence byte in a `repr(packed)` record. Tests confirm **8 to 5 bytes per slot**, with the per-variable record size unchanged. Field access returns copies and introduces no unsafe reference or value-ID restriction.

Absent definitions and present definitions containing None remain distinct. Sparse storage, promotion thresholds, the four-times-fact capacity bound, replacement results, cloning and ordered enumeration retain their behavior. The independent token audit confirms the entire remaining SSA algorithm is unchanged after the two required copied-return projections. Sealing, predecessor traversal, generation-based cycle detection, sentinel choice, alias and Value creation retain the original implementation. Grammar, fresh proofs, scalar native emission, finite admission and emitted-program allocation are unchanged.

The exact reference/candidate modules pass **31 actual unit tests**: 15 reference and 16 candidate. They include the existing 50,000-operation ordered-map oracle with explicit None, sparse/extreme IDs, promotion and ordered enumeration, plus the new layout/presence/copy test. **328 paired cases independently verify 656 functions** and compare complete raw/compacted IR, raw identities, aliases, types, queries, definitions, side effects and machine code. These tests use the pinned standard dependency graph before the candidate compiler rebuild. Compiler check/build and the native CLI check with `--features cranelift_build` also pass; no default-feature CLI claim is made.

## Matched measurements

| Boundary | Original | Packed slots | Change |
| --- | ---: | ---: | ---: |
| Complete source to executable | 14.948667 s | 14.891670 s | -0.381% |
| Compiler process within complete builds | 14.816782 s | 14.761669 s | -0.372% |
| Native link | 0.130939 s | 0.129085 s | -1.416% |
| Separate Object internal compiler | 14.735751 | 14.794715 | +0.4001% |
| Separate Object source CFG | 2.238602 | 2.244945 | +0.2833% |
| Separate Object instructions | 220476565488.500000 | 220565516131.750000 | +0.0403% |
| Separate Object maximum RSS | 4489990144.000000 | 4471537664.000000 | -0.4110% |

Each cohort uses eight serial ABBA/BAAB observations, four per subject. Both use the standard ThinLTO/CGU1 compiler profile, pinned compiler-only mimalloc, the same 61 program flags and two workers. Builds, tests, sampling, hashing and archive work do not overlap primary timing. Complete builds start fresh source assembly and end after the native link completes; fixed runtime/adapter inputs are cached and pinned. Historical first-use adapter construction remains separate. No new sampling or program-runtime timing campaign is run.

The complete-build blocks improve 0.728% and 0.031%. Object compiler blocks slow 0.122% and 0.679%. Peak RSS falls about 18.45 MB; this process-peak observation does not isolate dense storage's allocation extent. The smaller layout alone does not establish a useful speedup.

Adverse Object fields: canonical_lowering_seconds +0.007%, catalog_seconds +1.207%, cleanup_including_compaction_seconds +1.631%, cleanup_seconds +1.542%, compaction_seconds +3.379%, compiler_seconds +0.400%, encode_seconds +1.055%, frontend_seconds +0.179%, instructions +0.040%, native_seconds +0.483%, numeric_generation_seconds +0.604%, parse_seconds +0.051%, source_cfg_seconds +0.283%, whole_process_seconds +0.395%. All favorable and adverse results, phase stamps, counters and balanced blocks remain in the [complete result](../../benchmarks/mech-deflate/native-profile/packed-ssa-slots-apple-m3-20261115.json). Object and native cohorts remain separate; no gain is inferred for an unbuilt candidate PGO compiler.

## Output gates and evidence

Every source, MBC, Object and **complete metadata** byte matches in all main observations and 14 fixture pairs. The default-serial gate and real-worker diagnostic pass; the latter records 344 jobs including 48 cleanup-and-machine jobs. Both preliminary executable gates, those two gates and eight primary builds reproduce all **12 complete executables**, 2,060,440 bytes each, SHA-256 `d6c65cc3fb18acc4be77dce329962105bf0f2b56dd2623a12f466256463eca61`.

All four test/build stages and six qualification stages complete successfully. Before qualification, an inherited helper's descriptive scope was corrected from an older worker trial to this trial; its commands, timing and gates were unchanged. Actual driver AST, helper hashes, original/derived sources and every observation are retained. Source and the canonical standard compiler are restored exactly. Cargo libraries remain from the rejected candidate until a genuine rebuild; the restored source has a fresh mtime. The selected optimized compiler `8043` and profile `8691` are untouched.

The fiftieth [repository packet](../../benchmarks/mech-deflate/evidence/compiler-profile-20261114/README.md) preserves the rejected source/patch, exact tests and oracle, audit, recipes/derivation, all commands/logs/observations, complete program outputs/executables, fixed native inputs and restoration receipt. Compiler/test executables are excluded and hash-pinned. Fresh repository extraction verifies every manifest/blob and replays the summary byte-identically. Further dense-layout work is lower priority after this mixed timing result. Compilation below ten seconds remains open.
