# Runtime work toward the 63rd libdeflate win

The objective is **63/81 libdeflate wins while preserving every existing peer win and 81/81 Roc**. The retained selection remains **63,532 B, 81/81 Roc and 62/81 libdeflate**. None of the experiments below earns a new win. They remain available for repair.

These are single-case diagnostics on `prefix-natural-4096`: **96 balanced paired blocks in each of two orders**, with the exact qualified `834c7a7e…` worker as the same-run control and pinned Roc/libdeflate workers. Ratios below one are faster. They are not complete 81-case qualifications and cannot establish preservation of every previous win. Full selection and compile-time measurements were not repeated for unsuccessful probes.

| Variant | Whole encoder text, B | Mech/libdeflate, forward / reverse | Experiment/control, forward / reverse |
| --- | ---: | ---: | ---: |
| scalar-from-end-profiled | 63,532 | 1.065718 / 1.067057 | 0.999572 / 0.995989 |
| scalar-pack-profiled | 63,548 | 1.088523 / 1.086973 | 1.024767 / 1.020298 |
| scalar-pack-body-profiled | 63,564 | 1.081722 / 1.086911 | 1.018112 / 1.015848 |
| scalar-encode-body-profiled | 63,692 | 1.095774 / 1.097945 | 1.028221 / 1.035572 |
| scalar-pack-addresses-profiled | 63,548 | 1.081715 / 1.090287 | 1.016663 / 1.018280 |
| pointer-exit-profiled | 63,548 | 1.068693 / 1.062545 | 1.001794 / 0.994764 |
| pointer-body-profiled | 63,548 | 1.061123 / 1.065424 | 0.996011 / 0.996277 |
| pointer-body-lanes-profiled | 63,564 | 1.065733 / 1.065454 | 0.996701 / 0.998656 |
| wordfirst-profiled | 63,532 | 1.054295 / 1.060539 | 1.001425 / 0.994721 |
| wordfirst-wide-profiled | 63,772 | 1.075208 / 1.081680 | 1.011481 / 1.007813 |
| key-single-profiled | 63,724 | 1.134150 / 1.142768 | 1.068163 / 1.068572 |
| key-pair-profiled | 63,836 | 1.116578 / 1.121730 | 1.052733 / 1.042595 |
| key-pair-body-profiled | 63,852 | 1.109002 / 1.122403 | 1.042089 / 1.045383 |
| key-pair-body-lanes-profiled | 63,852 | 1.112963 / 1.118718 | 1.047712 / 1.044809 |
| key-single-vector12-profiled | 63,724 | 1.138471 / 1.143722 | 1.072114 / 1.068351 |
| key-pair-vector12-profiled | 63,836 | 1.116214 / 1.121473 | 1.050136 / 1.044286 |
| key-pair-vector12-body-profiled | 63,852 | 1.115231 / 1.118584 | 1.050722 / 1.045841 |
| keyfix-single-profiled | 63,612 | 1.126387 / 1.132252 | 1.057535 / 1.057185 |
| keyfix-pair-profiled | 63,740 | 1.083748 / 1.092165 | 1.025344 / 1.022919 |
| keyfix-pair-body-profiled | 63,740 | 1.084204 / 1.101849 | 1.026571 / 1.026695 |
| keyfix-pair-body-lanes-profiled | 63,756 | 1.090423 / 1.094045 | 1.025392 / 1.024293 |
| keyfix-single-vector12-profiled | 63,612 | 1.118545 / 1.122265 | 1.058480 / 1.059432 |
| keyfix-pair-vector12-profiled | 63,740 | 1.095582 / 1.094667 | 1.031811 / 1.024732 |
| keyfix-pair-vector12-body-profiled | 63,740 | 1.096200 / 1.098104 | 1.032327 / 1.032940 |
| branch-hot-profiled | 63,548 | 1.153705 / 1.156822 | 1.074272 / 1.077795 |
| branch-cold-profiled | 63,564 | 1.152626 / 1.163628 | 1.087318 / 1.084344 |
| branch-body-profiled | 63,564 | 1.149363 / 1.151676 | 1.084572 / 1.072653 |
| branch-cold-body-profiled | 63,564 | 1.155691 / 1.158075 | 1.085888 / 1.080005 |
| vector4-profiled | 63,548 | 1.141669 / 1.143444 | 1.069037 / 1.070731 |
| vector8-profiled | 63,548 | 1.069830 / 1.083427 | 1.007210 / 1.015142 |
| vector12-profiled | 63,548 | 1.058452 / 1.062385 | 0.997239 / 0.999075 |
| vector8-body-profiled | 63,548 | 1.073091 / 1.077040 | 1.007546 / 1.008724 |
| vector12-body-profiled | 63,548 | 1.063794 / 1.065055 | 1.004800 / 0.999787 |
| branch-vector8-body-profiled | 63,564 | 1.124622 / 1.128765 | 1.057105 / 1.050820 |
| branch-vector12-body-profiled | 63,564 | 1.132821 / 1.135969 | 1.066860 / 1.059108 |
| matcher-vector-profiled | 63,532 | 1.067022 / 1.062538 | 0.999313 / 0.999182 |
| matcher-word-profiled | 63,420 | 1.062415 / 1.066324 | 1.002466 / 0.996970 |

## Mechanisms and bounded repairs

- **Vector pointers:** carry admitted addresses inside the vector body; reconstruct the scalar remainder from the invariant base. Test both body-only and terminal transport, then combine body pointers with carried vector indices. Body-only is within about 0.4% of control and remains short of C. Only this small generic opt-in source extension is committed as live compiler code (`66073d79dc7b9f1b44d89fcffda8cc6c72ceccb2`). The default compiler mode reproduces the retained artifacts exactly.
- **Earlier scalar options:** repeat delayed item packing, encoded winners, first-ordinal reuse and reverse-address construction against the current compiler. Combining these with body-only pointers does not recover the required time.
- **Explicit winner branch / vector entry:** avoid constructing an unselected scalar item; try hot and cold update branches, and 4/8/12-element vector entry. The 12-element entry is close to control. Other changes slow this case; no new win is inferred.
- **Packed unsigned cost/index:** reduce a U64 lexicographic key in single or paired scalar visits. The initial version adds spills. A bounded repair uses one-based source ordinals with a zero sentinel, removes an extra admission test and drops the redundant live minimum. It reduces scalar stack loads from 35,625 to 55, but still takes about 2–3% more time for paired visits. Exact source inputs for both stages are archived.
- **Matcher strategy:** repeat word-only and wider-vector modes. Disabling saved-prefix options emits the exact same matcher object, establishing no applicable opportunity for those flags here. Word-only saves 112 B after placement but does not earn a runtime win.
- **Word-first hybrid:** run the original 8-byte comparison before wider tiers on the proved unsaved path. Test both 16- and 64-byte tiers. The original source already uses trailing-zero detection on differing words. The hybrid remains near control, while the wider tier is slower. Preserve both source snapshots and failed attempts, without adding unused production machinery.

Every timed worker is source-emitted from the unchanged 2,719-line codec and canonical bytecode. No LLVM dependency is added. Counter clones are untimed diagnostics, never latency inputs. The scorer/matcher layout profiles are explicit partial-flow estimates, not exact edge counts; profiles for unchanged other functions are reused only after code and relocation identities agree.

The actual linked checks cover 162 output/inflation/alias observations and six checked failures per profiled worker, with 4,848 B active stack below the 5,280 B limit. Those checks do not establish a fresh twelve-recovery campaign for every probe. Optimized original/decoded reduction tests pass in five body-pointer configurations (15 tests each). The archived packed-key compiler has 16 tests passing in seven configurations and a direct production-key JIT test over 5,400 combinations. Some generic whole-program fixtures do not admit the biased packed-item fast path, so the direct primitive test and actual codec checks provide different coverage. The word-first harness tests the exact production unsaved rewrite; saved-state adapters are unexecuted and provide no saved-state proof.

## Portable evidence

`source-and-receipts.tar.gz` contains compiler input overlays, implementation patches, emission recipes, native objects, metadata, profiles, counter provenance and checks. `raw-runtime.tar.gz` contains every raw latency block, peer pins and the timing driver. `archive-files.json` pins every entry. `study.json` records compiler and native-artifact identities and every result. Compilers and linked workers are not bundled; rebuild the compiler from the pinned base plus the exact `source-inputs` overlay for the stage being replayed, then apply its recipe. Profile path arguments can be rebound to `portable-profiles/<sha>.json` after hash verification. Retain untracked source inputs from snapshots as well as tracked patches when replaying prototype stages.

Some early `bound/runtime.json` files retain inherited `size_bytes`/`encoder_sha256` labels; those two labels are not authoritative. Use each emission receipt's `text_bytes` and `artifacts.program.o`, checked directly by `verify.py`, plus the actual worker hash bound by each campaign. No experimental promotions are recorded.

Run `python3 verify.py` to recalculate archive hashes, emitted text sizes, canonical bytecode, live source identity and all paired runtime ratios. The selected worker and full scorecard remain documented in [the retained study](../encoder-size-gap/immutable-current-read-study/README.md). The latest matched source-through-link timing, **2.245045 s**, belongs to compiler source `44fc9f053…`; the new opt-in pointer source has not received a fresh matched build cohort.
