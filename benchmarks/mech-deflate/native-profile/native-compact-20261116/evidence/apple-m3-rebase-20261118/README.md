# Apple M3 comparison after upstream compiler integration

The M3 compiler changes are rebased onto `ca0c778e58618c72e6bd13e7df03d3089d4ec2e3` from PR #993. The combined recipe produces **63,868 B** of encoder text, 644 B smaller than the previous M3 build. Native stack high-water is **4,768 B**, below the unchanged 5,280 B bound. The benchmark program and canonical bytecode are unchanged.

## Complete runtime comparison

| Order | Previous Mech / Roc | Rebased Mech / Roc | Previous Mech / libdeflate | Rebased Mech / libdeflate |
| --- | ---: | ---: | ---: | ---: |
| Forward | 81/81 | 81/81 | 58/81 | 58/81 |
| Reverse | 81/81 | 81/81 | 60/81 | 60/81 |

Each order covers all 81 cases with 96 balanced blocks and 982,656 timed calls. A win is a paired median ratio strictly below one; equality is a tie. The earlier retained M3 executable's separate [61/81 confirmation](../apple-m3-compact-packets-20261118/word-scans/confirmation/README.md) remains a separate observation. This rerun does not establish 61/81 in both orders for the rebased recipe.

[Every timing in seconds and comparator multiples](timings.csv) · [Complete counts](results.json) · [Original raw-record replay](raw-replay.json) · [Acquisition plan](plan.json)

The measured in-memory native-call interval includes heap construction, native execution, final allocation transfer and physical private cleanup. Preparation, output validation, reporting and process startup are excluded. Roc and libdeflate 1.26 level 12 use unchanged executables and each compression starts with fresh state. All actors run serially. Seconds are the actor medians; comparator multiples are medians of paired observations. [Environment](environment.json) · [Boundary contract](contract.json)

## Source and integration checks

The combined compiler passes the 211 backend tests, three direct-SSA tests, four packet-contract tests, the equality-scan oracle and the complementary-predicate oracle. The focused scan harness extracts the maintained recurrence unchanged into the existing proof adapter. The linked executable passes 162 exact-output, independent-decompression and borrowed-alias checks, six checked failure cases, failure-origin preservation and private cleanup. [Linked proofs](linked-proofs.json) · [Output checks](correctness.json)

The original upstream 65,516 B deployment recipe reproduces its **encoder object, complete metadata and canonical bytecode byte for byte** with this compiler. Its configuration remains separate from the M3 recipe. [Upstream reproduction](upstream-reproduction.json)

The source-built combined recipe keeps the M3 word-comparison and complementary-predicate settings, adds the upstream checked default-load selection and shared byte-read continuations, and transfers the existing checked leaf block order to the changed machine code. It retains the M3 entry-literal setting; the upstream replay uses its own original settings. No fixture-specific output, foreign encoder call, omitted cleanup or changed error contract is introduced.

Build the compiler at the [pinned source revision](recipe.json), then run `reproduce.py COMPILER OUTPUT_DIRECTORY`. The replay verifies the original program, profiles, object, metadata and bytecode. [Compiler command](compiler-build-command.json) · [Source inventory](source-inventory.json) · [Complete local build and linker inputs](local-build.json)

Raw observations, exact executable identities, focused harness sources, test commands and unsuccessful harness setup attempts are preserved in the [integration evidence archive](archive.json). Build durations in the local receipt are diagnostic; no new controlled compilation-time comparison is claimed.
