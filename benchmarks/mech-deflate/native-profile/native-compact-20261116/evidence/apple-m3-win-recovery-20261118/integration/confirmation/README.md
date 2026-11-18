# Apple M3 qualification of the shared M1 recipe

The shared `m1-shared-heap-layout` recipe records **81/81 wins against
Roc and 61/81 against libdeflate 1.26 level 12 in both complete M3 orders**.
It emits the same **64,652 B** encoder object, complete metadata and
canonical bytecode as the M1 recipe. The final integrated compiler at
`09eccc93df` reproduces all three artifacts exactly.
[Source reproduction](source-replay.json) · [Compiler build](compiler-build.json)

| Complete order | Shared build / Roc | Shared build / libdeflate 12 | Contemporaneous control / Roc | Contemporaneous control / libdeflate 12 |
| --- | ---: | ---: | ---: | ---: |
| Forward | 81/81 | 61/81 | 81/81 | 61/81 |
| Reverse | 81/81 | 61/81 | 81/81 | 60/81 |

Raw-record replay verifies **1,965,312 timed calls**, 982,656 in each
order. Each comparison uses the geometric mean of 96 balanced-block time
ratios: below one is a win, above one is a loss and exactly one is a tie.
The separate actor medians are reported in raw seconds.
[Complete result](result.json) · [Every case in seconds and comparator multiples](timings.csv) ·
[Raw-record verification](replay.json) · [Predeclared plan](plan.json)

The timed interval is the in-memory native diagnostic compression call,
including Heap construction, final allocation transfer, physical private
cleanup and Heap drop. Preparation, answer validation, reporting and
process startup are excluded. Peer executables and the pinned allocator
are unchanged. Only one actor performs compression at a time. Both full
orders completed without a background-activity rejection.
[Timing contract](contract.json) · [M3 environment](environment.json)

The linked recipe passes 162 exact-output, independent inflation and
borrowed-alias checks, six checked failure classes and cleanup checks.
Its 81-case measured stack high-water is **4,784 B**, below the unchanged
5,280 B limit. [Linked checks and stack receipt](../common-reference-ready.json)

Two earlier full attempts were rejected by the activity guard: an external
Rust compiler interrupted the first, and a Codex renderer interrupted the
second. Both partial attempts are retained. The first clean complete
pair is the result above; completed scores were not retried.

The observations, source snapshots, compiler and worker identities,
interrupted attempts and reproduction commands are retained in the
[verified qualification archive](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/pr995-deflate-shared-integration-20261118-cbec7c8e2a77.tar.gz).
SHA-256: `cbec7c8e2a779b3df1a6c960e2b0453565d11cd75bdc3efa9ede06630d71baf9`.
Its 277 payload members extend the preceding integration archive; the
parent archive and hash are in the [archive receipt](archive.json).

The earlier completed M3 integration score of 58/81 forward and 60/81
reverse describes a different build configuration and remains in the
[historical comparison](../../../apple-m3-rebase-20261118/README.md).
Additional M3-only experimental settings are not selected by this shared
recipe, and their pending measurements do not contribute to its score.
