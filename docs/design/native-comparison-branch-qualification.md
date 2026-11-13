# Short-circuit pure comparison branches

The opt-in AArch64 compact compiler recognizes a masked branch fed by a shared canonical zero-or-one decision and three scalar integer comparisons. It retains the shared decision and takes the existing false edge as soon as the conjunction fails. Eligibility requires a single-entry acyclic path, total operations, no uses of skipped definitions outside that path, distinct existing destinations without edge arguments, and a current shared word proved by the existing compound-comparison rule. All original loads, stores, checks, publication and selected effects keep their order. The verifier checks the resulting CFG and dominance.

Compiler change `aaabd8bb7bc77b66584ca93e7af724ffddbf1915`, review parent `cf514be73137a8c34cc31184121362af7056300e`, and measured control `e885dd503cbc1fa52185773d6e7c8d936a0489d7`. Enable with `MECH_SHORT_CIRCUIT_COMPARISON_BRANCHES=1` alongside the pinned compact settings. The exact source and revision-14 bytecode remain `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9` and `51de51658c1d8d0be66567400e382ef5c0f5789baf317c53a8cd56d1df2dd0ca`. The full-run ratios and adverse results come from the [published-layout comparison](native-matcher-comparison-branch-trial.md).

| Qualification | Result |
| --- | --- |
| Optimized Rust build | nightly 2026-03-03, opt-level 3, warnings denied, `aot,canonical-native,canonical-native-program,canonical-native-program-runtime,canonical-native-runtime,jit` |
| Source and decoded emission | 238.20 s from source including 55.55 s frontend, 183.06 s from bytecode |
| Exact linked smoke checks | 648, two complete calls on all 81 fixtures per source form and descriptor |
| Direct parser positions | 136,192 with assertion origin 15525 and successful recovery |
| Focused native tests | 2 passed, 8,196 calls with an independent result and effects oracle, plus refusal for raw words, effects, escapes and alternate entry |
| Flag-off | Decoded Object, descriptor and bytecode byte-identical to the control |
| Architecture | Formatting, compiler planning, native catalog, interactive architecture and resident routing all pass |
| Managed benchmark | All 88 outputs agree with pinned Rust, 13 bundle hashes match the prior managed build, paired ratios 0.643452× and 0.649523× Rust |

Both enabled forms emit Object `46ec6519a4e244d57da2bf3472a68e743289dc52077407e06c3161d8218f6a3f` and timed worker `d7ca971ab8ac64704718090a390bad322615b0d90315c50cd22e0efee502e0b5`. Their descriptors differ only in the inherited `source_input_form` tag. One of 357 recorded bodies changes. The matcher drops from 9,976 to 9,956 bytes with the complete frame still 576 bytes. The exact mixed-input profile places 2,129/5,406 running timer samples in the matcher and 2,023/5,406 in the composed parser. The exact random 1 MiB profile places 3,750/5,366 in the matcher. Profile samples include validation. The managed timing is a separate comparison and the identical bundle supports no managed speed claim.

The first focused build failed warnings-as-errors on an unused import. The first link wrapper selected an existing receipt directory. A provenance check initially required byte-identical descriptors across source forms and was corrected to allow only the independently verified origin tag difference. The first archive attempt exceeded the per-file size limit on the full descriptor. The replay uses exact descriptor hashes and the structural comparison receipt. All four harness diagnostics are preserved, and the corrected checks pass. The full broad suite was skipped and ordinary CI is canceled under the current review workflow. The compact benchmark uses the same diagnostic entry contract as the control.

The compiler qualification archive contains 134 checksummed files. SHA-256 `6af3619db85addccba766962acb18ce6038e9eb9ba71bdcfd0e8bb3fafdeca67`. [Compiler, linked and managed qualification receipts](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-aaabd8bb7b-comparison-branch-compiler.tar.gz).
