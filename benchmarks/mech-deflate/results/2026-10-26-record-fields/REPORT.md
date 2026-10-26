# Scalar record-field checkpoint

The native collector repeatedly reads scalar fields from records carrying the
262144-entry U16 tree and 1500513-word U32 cache. General selected-call binding
charges 9,531,648 compute units for a minimal read of one U64 field. A selected
read into an existing native scalar lane now avoids general call binding,
dimension closure and a temporary canonical scalar owner. It allocates no
storage in the focused direct-call measurement.

This follows code parent `96f59a9a22a100e7169bee8167d02cca07ac99ef`.
Program IR remains 108 and ABI remains 35; work policy advances from 83 to 84.
The maintained privately selected `RecordFieldAccess` operation, field-name
interpretation, exact scalar kinds, same-account admission, finite work and
logical output/coexistence limits remain authoritative. No operation, guest
instruction or public syntax is added. Existing source and decoded artifact
validation still applies.

## Validation

`record-word-focused-v3` passes **3 tests**, with **0 failed, 0 ignored and
266 filtered**, in 6.13 seconds of test execution. The profile is
`mech-gpu --no-default-features --features canonical-native-program`, using
nightly-2026-03-03, locked offline dependencies, one build job, disabled
incremental compilation and debug information, and warnings as errors.

```sh
cargo +nightly-2026-03-03 test --locked --offline -j1 -p mech-gpu \
  --no-default-features --features canonical-native-program \
  --test canonical_artifact_program -- record_word_qualification \
  --nocapture --test-threads=1
```

The focused coverage executes original and decoded artifacts. It covers Bool,
U8/U16/U32/U64, I64, and F64 including negative-zero bits; zero direct-read
allocations; exact and one-short compute/comparison, temporary bytes, retained
nodes, output bytes and output elements; foreign-account rejection; assertion
before arithmetic failure; fresh repeated inputs; preserved prior publication
and input aliases; cleanup and recovery; and rejection of a replacement factory
before it binds. It does not establish linked/AOT coverage for this new lane.

Two earlier receipts are retained. The first has an introduced test import
error (`RecordFieldDraft` instead of `NamedValueDraft`). The second has two
introduced test-source syntax errors, subsequently corrected to use ordinary
functions, and the complete collector's separate 10-billion-unit work denial.
The compiler implementation built successfully in both attempts. These are not
reported as passing commands or inherited failures.

## Scope and remaining work

This is a compiler checkpoint, not a completed compressor or compression-time
result. Existing component values, physical buffer extents and qualification
allowances are unchanged. The complete block-loop source, independent reference
and its diagnostic runs are published in the following review branch. General
record construction/transport and buffer mutation still have substantial costs.

Earlier committed validation and limitations remain pinned to their recorded
revisions in the adjacent constructor, selected-length and block-statistics
reports. This checkpoint does not rerun or requalify the full historical suite,
all feature combinations, linked execution, or remote CI.

## Review base

The review base is `a2777fdb598ad83fbc136e8fe3065a9fb54069f5`, the local
consolidated-stack checkpoint through the piped self-recursion repair. The exact
base is published as `codex/deflate-review-base-20261026`. It contains the carried
prerequisite stack and the local consolidated architecture at
`2fe73989472b85bae277efb608666f193860d5cc`; it is not the current published head
of #935. Publishing the snapshot permits review of the subsequent compiler work
without rewriting any existing PR branch. Integration still requires reconciling
the review base with the upstream stack. No merge is claimed.
