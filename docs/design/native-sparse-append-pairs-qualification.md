# Qualify two checked append visits per iteration

Compiler `170401d69aaeded4304b2992763dd5dfd7e9aabf`, review parent `a2a7a1c8dd7d3c8f1b07de3a82f1e4c6d7cdc4d4`, measured worker control from `14d4d42214c2d76849cfa5bdfdf3b0e510935ea1`. The rule is enabled with `MECH_PAIR_CHECKED_APPENDS=1` under the existing checked append and whole-phase capacity flags. It runs on little-endian AArch64 in the compact compiler. It recognizes the selected visit through its typed operands, checked entry window, current packet values, load widths, append word and unit state updates. The pairing path checks nonwrapping byte ranges and that writes cannot change either input. The two exact wrapping U32 additions and two U64 stores run in order. Any failed guard resumes the existing scalar visit with the original current state. [Isolated comparison and peer machine evidence](native-sparse-append-pairs-pilot.md).

Four of 357 bodies change. Each ordinary no-growth parser grows from 1,504 to 1,792 bytes and its frame shrinks from 272 to 256 bytes. The four composed bodies and the matcher retain their Object bytes. The actual paired loop performs one header load, tests both selectors, checks both literal indices, reads both current costs and writes the consecutive words. The timed Object is `4ddc16b053bda6256656e0d5cbf95ba60cab31b0911b9aadf526b66e1f3cab67`. The linked worker is `f1239579b37fcd1f6399eb83d17eb13140f6093f9a421a263ca93ee28220014f`. Nightly 2026-03-03 with `-D warnings` and `aot,canonical-native,canonical-native-program,canonical-native-program-runtime,canonical-native-runtime,jit` builds the pinned timing closure.

| Matched cohort | Cases | Timed calls | Elapsed / control | Pooled and block gates |
| --- | ---: | ---: | ---: | ---: |
| Screen | 18 | 7,776 | 0.977398× | 18/18 |
| Confirmation | 8 | 6,816 | 0.977208× | 8/8 |
| First full matrix | 81 | 245,664 | 0.979019× | 81/81 |
| Full repeat | 81 | 245,664 | 0.980157× | 81/81 |
| Adverse repeat | 10 | 105,120 | 0.986763× | 10/10 |

| Mech faster than | First full | Full repeat | Paired first control | Paired repeat control |
| --- | ---: | ---: | ---: | ---: |
| Roc | 62/81 | 62/81 | 59/81 | 59/81 |
| libdeflate | 33/81 | 32/81 | 29/81 | 27/81 |
| Both | 16/81 | 16/81 | 12/81 | 11/81 |
| At least one | 79/81 | 78/81 | 76/81 | 75/81 |

| Input | First control / Roc | First candidate / Roc | Repeat candidate / Roc | First / libdeflate | Repeat / libdeflate |
| --- | ---: | ---: | ---: | ---: | ---: |
| Eight literals | 0.162850× | 0.159891× | 0.161206× | 2.045681× | 2.060078× |
| Random 128 KiB | 1.008219× | 0.966177× | 0.965980× | 1.501933× | 1.502023× |
| Random 256 KiB | 1.024626× | 0.984984× | 0.989045× | 1.469998× | 1.469216× |
| Random 1 MiB | 1.032840× | 0.988962× | 0.987815× | 1.397278× | 1.397416× |
| Natural 16 KiB | 1.041153× | 1.033878× | 1.035183× | 1.042094× | 1.042788× |
| Reymont 16 KiB | 1.039251× | 1.038522× | 1.029408× | 1.045745× | 1.039911× |
| Reymont 32,769 bytes | 1.064841× | 1.065094× | 1.095294× | 0.968197× | 1.063610× |

Natural and Reymont 16 KiB are separate fixture rows with the same input SHA-256. They stay above both peers in every full run. Reymont 32,769 bytes loses a libdeflate win in the full repeat and takes 1.038299× its paired control. It takes 1.000237× control in the first matrix and 0.998700× in the focused repeat, where Mech returns to 0.971485× libdeflate. Every adverse observation remains in the replay. The empty input passes both gates in both compiled runs and in the adverse repeat. The six tracked private allocation, copy, initialization and cleanup measures agree. The program's pinned source, revision-14 bytecode, exact compressed bytes, capacities and held aliases agree across the campaign.

| Separate counter case | Instructions / control | Cycles / control | Lifetime physical peak before → after |
| --- | ---: | ---: | ---: |
| Eight literals | 1.000111× | 1.031873× | 44,794,816 → 44,811,264 B |
| Natural 16 KiB | 1.000001× | 0.997950× | 45,269,952 → 45,286,400 B |
| Random 128 KiB | 0.952433× | 0.960658× | 46,597,056 → 46,416,896 B |
| Random 256 KiB | 0.952040× | 0.959578× | 47,121,344 → 47,858,688 B |
| Random 1 MiB | 0.951891× | 0.959383× | 50,938,816 → 53,576,704 B |

The lifetime peak is process-wide and includes the allocator and worker. The 1 MiB counter run rises by 2,637,888 bytes despite equal managed private measures. Three separate exact-worker timer profiles include independent output validation. The sparse parser accounts for 483 of 5,256 mapped timer PCs at random 128 KiB and 545 of 5,359 at 1 MiB. The unchanged composed parser accounts for 2,054 of 4,237 at natural 16 KiB. Its match-record setup and scalar completion remain the next focus.

Source and decoded emission reproduce the same Object and executable. Their descriptors differ solely in `source_input_form`. Disabling the flag reproduces the retained decoded Object, descriptor and bytecode exactly. The original invocation takes 117.66 seconds and decoded takes 62.79 seconds in this pinned closure. The independent source-to-Object profiling tool uses a different dependency closure. Its sub-minute result remains documented with its own configuration. [Compile-time comparison](native-compile-under-minute-profile.md).

Original and decoded workers pass 648 complete linked calls across both descriptors. The direct oracle checks 136,192 candidate positions and 72 extra candidate controls across the two forms, with matched retained runs. The extra controls cover zero and odd lengths, the minimum pairing threshold, short literal views, wrapping sums, and a direct overlapping-view refusal. The existing oracle preserves error origin 15525 and successful retry. Four focused maintained trace tests and seven architecture, catalog and formatting checks pass. The managed guard verifies 88 exact results and 13 identical bundle files. Its calculation timer takes 0.720891× and 0.730719× Rust in the two cohorts. The compression measurements use the compact route. Hosted CI and a full generated-application suite are outside these local receipts.

The first two compiler attempts declined pairing and emitted the control. The second exposed that the existing trace's store and its terminator are separated by the length update. The rule now uses the store recorded in the trace. The first standalone test adapter imported unrelated test modules and was corrected. The initial direct witness expected spare capacity to shrink with the output length. The corrected witness checks the original fixed capacity against both binaries. The archive verifier checks exact Object and worker equality and the known metadata provenance field separately.

The public replay verifies 460 files, 611,040 timed calls, five independent counter cases and three exact-worker profiles. Run `python3 -B verify_second_hint.py` after extracting. Archive SHA-256 `8ddc78cf5fdad055564a4e6d50e590c893badd224036a4ca8c4891819ff44ea7`. [Full qualification archive](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-922503c653-second-private-write-sparse-parser-qualification.tar.gz).
