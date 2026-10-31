# Review follow-ups

Base: `8bbfa71d8b3e32fa9772a0e64c4cb3674dc93814` (#971). Code head: `c049059017b114134789bc02a6ae7c2e82566bb4`.
This packet records the tested source state; its containing documentation commit
adds no compiler, component, or qualification changes.

## Changes and findings

- `90320acbeb2562672fce2b35505e1c2921b0c9a2`: declare three inherited unsafe
  boundaries precisely, without changing the audit. The two test-image boundaries
  compare build-owned descriptor bytes; the resident boundary appends admitted
  little-endian bytes to existing spare capacity. The earlier path-relocation
  finding on #927 was addressed in `396adb8cf`; this follow-up restores the full
  strict inventory on the current stack.
- `fce2c2c9c3ff9752460375e31c2e6c68225f84b0`: for checked functions, retain the
  supported structural argument carrier when a runtime-shaped tuple cannot be a
  lexical capture (#930). Examine each real argument, preserving the existing
  immutable-root capture optimization for eligible nonrecursive calls. Neither
  capture validation nor recursion detection is weakened. New used/unused tuple
  regressions cover both artifact forms, resident/native agreement, assertion
  precedence, failed publication, cleanup, and retry.
- `c049059017b114134789bc02a6ae7c2e82566bb4`: the standalone Huffman qualification component enters its existing
  two-code fallback before unsigned subtraction and sorting for fewer than two
  used symbols (#917). All nineteen checked-in C cases and their generator remain
  byte-identical. The added empty two-symbol case derives its expected full
  scratch arrays from the existing empty-alphabet fallback case; it is not
  claimed as a newly executed C oracle result.

The production compressor uses `deflate-huffman-level12.mec`, not the standalone
fixture changed here. All 25 assembled compressor components remain byte-identical.
A fresh source compilation produces the identical 4,140,067-byte measured artifact;
strict decode/re-encode is also identical. This is an artifact-stability witness,
not a new compression or timing result. Source and artifact hashes are in
`source-manifest.json`; `artifact-stability.rs` is the external witness source.

## Local validation

Toolchain: `nightly-2026-03-03`. Host test build:

```text
cargo test --locked --offline -j1 -p mech-gpu --no-default-features \
  --features canonical-native-program --test canonical_artifact_program --no-run
```

The JSON receipts retain exact commands, source HEAD, dirty-file state, environment,
exit status, and elapsed time. Corresponding complete logs are gzip compressed.
The test executable was rebuilt successfully and checked newer than both edited
Huffman source files before the final focused run.

| Receipt | Observed result |
| --- | --- |
| review-checked-tuple-red | Both new tuple regressions fail on the previous implementation: invalid capture / unsupported tuple capture. |
| review-huffman-empty-red | Original 19 cases pass, then the new empty-two-symbol input fails with checked arithmetic. |
| review-checked-tuple-green | 28 lexical assertion tests pass. |
| review-capture-transport | 3 existing buffer transport/allocation/alias/recovery tests pass, including real-size buffers. |
| review-checked-recursion | 10 existing direct/piped recursion tests pass, including fresh Boolean arguments. |
| review-huffman-green | 11 existing Huffman tests pass; the one new positive resident assertion fails for the inherited finite budget described below. |
| review-huffman-minimum-green | After removing that unsupported new resident expectation, the focused test passes: all 20 cases in original and decoded native artifacts (40 evaluations). |
| review-followups-static | Routing, source/native catalogs, analysis layout, CI inventory, formatting, and diff checks pass. |
| review-followups-boundary-inventory | Full strict unsafe-boundary audit passes, unchanged audit logic. |
| review-artifact-stability | Fresh original artifact and decoded re-encoding equal the measured complete compressor artifact byte for byte; no compression executed. |

Together, 53 distinct relevant Rust tests passed. The eleven passes from the mixed
Huffman invocation are retained as such; that invocation is not represented as a
passing suite. Existing expectations and resource allowances were not increased.

## Failure disposition and limits of this evidence

- The initial inventory invocation found the comparison harness's new allocator
  boundary. #971 already published `8bbfa71d8b3e32fa9772a0e64c4cb3674dc93814` for
  that introduced omission. The subsequent audit found inherited frame/buffer
  entries. An exact tracked-Rust projection of clean parent
  `a82d50823e4140edd862886b0c6b5f813a81411c` (2,010 tracked Rust files plus its
  original audit and allowlist targets) reproduces the inherited failure.
  `review-audit-unsafe-boundaries-parent` records that command; its receipt HEAD
  describes the calling worktree, while `UNSAFE_BOUNDARY_ROOT` selects the clean
  parent files. All these inventory failures are now fixed.
- The two expected red regression runs demonstrate the existing tuple-capture
  regression and standalone empty-alphabet arithmetic bug. Both are now fixed.
- Attempting to extend the pinned C generator with the new empty n=2 case makes
  that external oracle terminate with SIGSEGV during sorting. The temporary
  generator change was reverted and the original nineteen-case corpus verified
  unchanged. An initial shell wrapper continued after this failure; its printed
  claim of an additional generated case was incorrect. No such generated case
  was used or checked in. The new expected fallback is explicitly derived from
  the existing empty case and the canonical two-code rule.
- An additional resident positive call in the new Huffman test encountered the
  existing Admission/ComputeWork limit: required 18,048,020, allowed 16,777,216.
  Removed that unqualified new positive expectation. The existing
  `full_huffman_resident_denial_preserves_the_real_resource_budget_error` passes
  unchanged. This packet does not claim full Huffman positive resident coverage.
  The tuple regressions do establish resident/native agreement for the compiler
  change. No production budget was changed.
- The first external artifact witness build used `&String` instead of `&str` and
  failed with E0277. Corrected only that helper; the corrected build and witness
  pass. Both receipts are retained.

CI is intentionally cancelled at the user's request; this packet does not claim
completed CI or readiness to merge. No full-file compression or long-running
boundary qualification was restarted, and no new benchmark result is claimed.
The previously cancelled full-input run remains cancelled.
