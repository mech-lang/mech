# Linear compiler and compression stack

Code checkpoint: `4ea406e00f66a098b1e36d9b2e8a0b2474ee33d9`. The following documentation commit changes no compiler or benchmark source.

The authoritative foundation is #934 at `a093aea452340a70c1e72a1f58bd831f1edef70f`, #935 at `7543c56d5efc4df30d0b60961922024d74caa9d7`, and cleanup #939 at `a68fb2e60d7afbc59bc146f8878b17181e275645`. These remain draft and unmerged; their performance acceptance and remaining qualification are not changed by this restack.

## Chain

| PR | New code head |
| --- | --- |
| #913 | `a6ced5c6cb8ca2d650af1dab66da4f916a9b8c66` |
| #914 | `0b042eda4d9cd624dd01f31922c687dd8f375d17` |
| #915 | `8ebdc9acf0989fe7ebd236f6aabf6fcb0ecfd64a` |
| #916 | `9a35ee78b4c4bc2956b9f03fbabf549d81c15d50` |
| #917 | `1d01829ce43b2bbe9b19dca310cc7bed6b66122c` |
| #918 | `a8b5c7dd548f94232344488804ffffa24661e1e7` |
| #919 | `dfdd67159d1ce7220f3af2e0410ae5788dc881cf` |
| #920 | `6b261d463596e1eff909ea760171cc694187bf97` |
| #921 | `e91466012f72ac91e936a61307a81f2214fd0d13` |
| #922 | `485a7b6a32eeacb188f51ba3a0eed9a020f986a6` |
| #923 | `cc164f997c4534c9724c7a0892e39281021b81d5` |
| #924 | `384611b59ad6b8d77146f0a72959ed1731376b2f` |
| #925 | `e14bd7c787e0fe9fa0889a969059ab4ad2de9d54` |
| #926 | `2c64253f303a7b0d895001449e07c6672a2fcf93` |
| #927 | `8432cb25cccee483a3cbbb1a10802da76a4a6dd4` |
| #930 | `69cab89082a2640469ac05946d7ae46ad2a84771` |
| #931 | `cf5c55c42f8b39c4a53a4a0314ba948ac6c80cc2` |
| #932 | `21cb2b6a940e0e2b15721c3cc0aca7d3716c7570` |
| #933 | `09c1a516c371eb4c58fc6e32ce367a82b261646a` |
| #937 | `114eef487c472f4a28cf7334550f433245588ecb` |
| #938 | `4ea406e00f66a098b1e36d9b2e8a0b2474ee33d9` |

Every new head contains its predecessor, starting with #939. There are 76 replayed commits and zero merge commits above #939. #932 and #933 are sequential. The three comparison/prefix commits previously carried inside the snapshot base now belong visibly to #937. #937 targets #933; the temporary review-base snapshot is no longer an integration dependency.

`manifest.json` records every old/new commit, previous published PR head, and new code boundary. Exact old heads remain preserved in local `archive/linear-20261026/*` refs. Existing worktrees and unrelated local branches were not rewritten. The active reconciled worktree is `/private/tmp/mech-deflate-linear-stack`.

## Reconciliation

The narrow scalar kinds/values and assertion-failure terminal retain their exact tags in the new shared `identity/transcript.rs` authority. Scalar, Match, Iteration and Program owners continue to call that authority; duplicated old encoders are not restored. Removed redundant module paths/visibility stay removed, including the new uniform-collection child. All original buffer dimensions, algorithm constants, source checks and qualification allowances remain unchanged.

The full range comparison is retained, including original-commit trailers and the three explicitly reordered commits. The fixture audit compares every tracked native fixture, executable bytecode fixture and retained legacy comprehension fixture byte for byte with the previous top. `unchanged-fixtures.json` stores all hashes.

A local conflict-resolution script initially mishandled empty sections and resume bookkeeping. That unpublished attempt was discarded and replayed from the verified #918 commit. The final map was independently checked against every stage boundary and all 76 original commits; the final production source contains no conflict markers. No remote PR ever received the failed attempt.

## Validation at the code checkpoint

- Pinned-toolchain `cargo fmt --all -- --check`: passed.
- Native build-analysis layout, warning policy, resident routing and source catalog checks: passed.
- Source/bytecode fixture equivalence and complete linear ancestry: passed.
- The selected component, assertion, recursion, transport, word-read and real-buffer command passes **143 tests**, with **0 failed, 0 ignored, 127 filtered**, in **358.76 seconds** of test execution. Its code pin is `4ea406e00f66a098b1e36d9b2e8a0b2474ee33d9`; total command time is 834.80 seconds including the initial rebuild from the new worktree. It uses `canonical-native-program`, locked offline dependencies, one build job, no incremental/debug information, and warnings as errors. These are final-stack checks, not separate qualification of every intermediate head.
- The `canonical-native-program-runtime` library check passes in 15.92 seconds at documentation head `428436517187ab25ce19d6d2bd07cd9b01958aa2`, with identical compiler/runtime source.
- New linked/AOT, the full feature matrix and remote CI have not been qualified by this restack. CI completion is not awaited.

The cumulative `git diff --check` reports whitespace in historical raw `.log` receipts. Running the same check on the previous code series reproduces exactly those findings. The check excluding raw log receipts passes; raw evidence is kept byte-identical. This is inherited receipt formatting, not an introduced compiler change.

The collector remains incomplete. Its existing fixed-budget witness and larger diagnostic limitations remain visible in #938. This restack does not turn those failures into successes, change their allowances, or establish a full compression timing.


## Published native stack

GitHub's registered stack 936 now contains all 24 PRs in order: #934 → #935 → #939 → #913–#927 → #930 → #931 → #932 → #933 → #937 → #938. Every direct base ref and base SHA was independently verified against the preceding head by the GitHub API. The old stack 928, which still included closed #911, was dissolved before retargeting. PR numbers, draft states, code reviews and discussions are retained. The API verification is archived beside this report.

Twenty active CI runs triggered by this restack received successful cancellation requests. #929 and foundation runs were left untouched. This is not passing CI evidence; local executed results are reported above.
