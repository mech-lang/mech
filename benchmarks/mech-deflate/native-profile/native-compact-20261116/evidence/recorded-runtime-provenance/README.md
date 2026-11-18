# Recorded runtime source provenance

The optional historical replay verifies the three original runtime source files
from `3ff13b6ab4834770aa422fa26887c7ff8cef6b0e` before compiling them. A normal
encoder-object rebuild uses the maintained compiler and requires no Git history.

The [identity probe](identity.json) rebuilt the recorded runtime and linked it
with the pinned aligned object and fixed support. Restoring the historical
source locations yields the identical `674d88602946` worker. The runtime archive
has different Rust metadata. Reapply a historical timing receipt only after
verifying the worker and timing contract identities.

The unsafe-boundary audit consumes the live source tree. Its mutation suite
passed. After removing the duplicate, the unchanged audit reports
`benchmarks/related-post-gen/check-measurement.rs`. The file is identical on
[the PR #973 base](https://github.com/mech-lang/mech/blob/9087b1f07727a9bfcb7c09059f48edaca6dc2301/benchmarks/related-post-gen/check-measurement.rs).
[Audit output](audit-next-file.log)
