The archive-v4 phase ran its six methods once under the common timing mutex.
Four rejection methods passed. Two acceptance methods failed because the
temporary fixture root used macOS's `/var` alias, while the archive helper
canonicalizes its root to `/private/var`. The complete actual fixture log and
failed receipt remain unchanged. No metadata seal, proposal, compiler or
benchmark followed that failed phase.

This additive revision changes the fixture setup to resolve its temporary root
before creating files. The archive helper remains the exact unchanged
`proof_reuse_v4.py`, including regular hashes, exact link metadata, lexical
in-root cycles, no symlink parents, hardlink topology and no-extra-file inventory.
No production archive/reuse/algorithm/runtime/SDK/compiler behavior changes.

The new six-method entry point first verifies the consumed v4 fixture pins and
writes an exclusive failure journal referring to the original complete log.
It preserves the separate v3 circular-symlink metadata journal. Its own log and
receipt are `archive-v5-fixtures.log/json`; the previous files are not resumed
or overwritten. The new metadata proposal is
`proposals/private-input-archive-fixture-v5.json` and pins both failure lineages.
Earlier successful 12+12+8+10 fixtures and the real 15 binaries/112 bounded
outputs remain exact referenced evidence, never repeated or relabelled.

The bounded preparation scope remains five fresh builds, 44 missing output
checks and pending general primitive/work proofs. Canonical LOC and all full
timings remain separate gates. No v5 fixture or metadata phase has run yet;
root reviews the concrete emitted digest before any native preparation.
