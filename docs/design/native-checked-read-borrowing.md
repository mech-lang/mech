# Reuse a held owner for checked aggregate reads

The compact compiler now traces an aggregate owner copy through a short forward sequence of selected scalar reads. Every live path keeps the exact source home held until the temporary's final use. The proof follows owner lanes through copies and projections and stops at a live output, source redefinition, mutation, collection boundary, specialized native entry, or a path that can reenter an earlier site. The existing ownership transfers still run for true moves. The copied view keeps its address, shape, length and checks. Its owner lane is zero because the exact source home retains the backing through the last read. The pass is selected with `MECH_BORROW_COMPACT_READ_COPIES=1` in the compact diagnostic executable.

The fixed-layout pilot showed a 0.6–1.0% elapsed improvement on 16 KiB distribution change by removing three temporary references. The compiler derives 761 omitted retain sites from the typed plan without benchmark-specific identifiers. In the exact linked writer, the three initial reference increments disappear. The corresponding default and selected-read retirement sequences shrink as well. The actual load and bounds-error origins remain. [Isolated mechanism and timing controls](native-writer-temporary-reads.md).

| Generated root measurement | Control | Checked-read borrowing |
| --- | ---: | ---: |
| Code bytes | 956,596 | 830,500 |
| Frame bytes | 55,360 | 49,600 |
| Static stack-referencing instructions | 71,311 | 65,796 |
| Static indirect call instructions | 4,216 | 2,883 |

Only the root changes among 357 audited bodies. The source-tagged root matches the untagged baseline Object byte for byte. Original source and revision-14 bytecode emit the same candidate Object `6b3fc61e319e472f0a89f3eaf66f59bbb7adc831003c0ee3aa60740e6f7e6571`. The exact timed worker is `5b7543d031393fe8703400b1d810d6b9d0c2024d6e5b5b9b096428280fd97758`. The source and decoded emissions take 236.02 s and 185.09 s, respectively, with 56.98 s to finish the source frontend. The optimized six-feature build uses nightly 2026-03-03 and warnings as errors.

## Paired timing

| Cohort | Cases | Timed calls | Candidate / control | Pooled and block gates |
| --- | ---: | ---: | ---: | ---: |
| Screen | 18 | 62,880 | 0.988555× | 18/18 |
| First full matrix | 81 | 245,664 | 0.985225× | 81/81 |
| Adverse screen | 10 | 56,064 | 0.984075× | 10/10 |
| Frontier confirmation | 8 | 6,816 | 0.982872× | 8/8 |
| Full repeat | 81 | 245,664 | 0.985569× | 80/81 |
| Adverse repeat | 8 | 6,816 | 0.985165× | 8/8 |

Distribution change runs at 0.984453× and 0.989398× Roc, moving the result below 1× in both full matrices. The median repeat times are 831.542 µs for Mech, 840.271 µs for Roc and 488.000 µs for libdeflate. Eight literals run at 0.162044× and 0.160836× Roc. Both full cohorts move from 75 to 76 wins against at least one peer. The first records 29 libdeflate wins and 12 wins against both. The repeat records 27 and 10, equal to its paired control. [Full-matrix scoreboard and current frontier](native-compression-scoreboard.md).

The full repeat misses both 1.05 gates on repeated 16 KiB. All four workers slow substantially during that case. Its paired point estimate is 1.028368× with a pooled median of 719.562 versus 644.958 µs. The first full matrix measures 0.989660× and the adverse repeat measures 0.979838× with both gates passing. Random 32 KiB includes a 10.300 ms candidate observation in the repeat and measures 1.047812× overall with both gates passing. Its adverse repeat measures 0.993264×. These runs retain their original values and gate decisions.

| Independent counter case | Instructions / control | Cycles / control | Lifetime peak physical / control |
| --- | ---: | ---: | ---: |
| Distribution change, 16 KiB | 0.943256× | 0.979847× | 1.001813× |
| Natural 16 KiB | 0.988174× | 0.995449× | 1.002172× |
| Reymont 16 KiB | 0.988253× | 0.994098× | 1.002172× |
| Natural 256 KiB | 0.995472× | 0.998482× | 1.001294× |
| Eight literals | 0.960112× | 0.957068× | 1.000000× |

Distribution change saves 797,733.5 retired instructions per median counter call. All six tracked private allocation, copy, initialization, and lifetime measures agree in the matched elapsed observations. The process peak is an independent measurement that includes the worker and allocator. The candidate's exact binary places 933 of 4,573 distribution-change timer samples in the generated root. The earlier control profile, whose root code and address match the paired control, places 1,104 of 4,957 there. Samples mapped to the three read-and-retirement sites fall from 125 to 24. The literal append has 84 candidate samples. Natural and Reymont 16 KiB still place 2,578 of 5,253 and 2,589 of 5,251 samples in the composed parser. These profiles include validation.

## Validation

- Original and decoded artifacts pass 648 complete linked checks using the exact and shared descriptors. This covers all 81 reference streams, independent decompression, exact capacities, aliases and zero final scratch. The independent parser oracle runs 136,192 positions and preserves rejection origin 15525 and recovery.
- The two focused original and decoded witnesses execute 360 linked calls. They cover selected and unselected reads, 48 bounds errors, unchanged live outputs, retry, exact payload accounting and cleanup. An intervening indexed replacement keeps both aliases and declines borrowing.
- All five architecture and catalog checks pass, along with formatting and Git whitespace. The earlier method visibility and unused-field errors are fixed. The focused harness required local rather than document-level mutation and an opaque FFI pointer. The linked qualification adapter and counter checker use distinct receipts and the actual smaller function length after their initial setup errors.
- The separate managed Rust guard reproduces all 13 bundle files and validates 88 outputs. The primary and confirmation cohorts run at 0.704722× and 0.696830× Rust. Hosted CI and the full generated-application suite were outside this local qualification.

Compiler `d39b01501db68bdb9969504f157a6efe41d2b859`, parent `8a22010f3fcf549b5ed94d2667b50799ee971343`. The disabled build reproduces the prior Object, descriptor and bytecode exactly. The compression benchmark keeps the original source hash `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`, bytecode hash `51de51658c1d8d0be66567400e382ef5c0f5789baf317c53a8cd56d1df2dd0ca`, level 12, tables, search limits, capacities, allocator and complete-call timing boundary.

Run `python3 -B verify_compiler.py` after extraction. The independent public download verifies all 532 files, replays 623,904 timed calls and verifies the linked, focused and managed receipts, five counter cases and four exact-worker profiles. SHA-256 `52243a3ed5c62d01c4d973308ee7954740e4199100c4d428eef91574b59667d9`. [Source patch, exact binary excerpts and validation receipts](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-8a22010f3f-checked-read-borrowing.tar.gz).
