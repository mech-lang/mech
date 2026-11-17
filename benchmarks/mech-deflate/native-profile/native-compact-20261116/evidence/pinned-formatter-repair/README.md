# Pinned parser formatting

The pinned nightly formatter reported the two parser files in the
current review. Both files now pass the workspace-wide command
`cargo +nightly-2026-03-03 fmt --all -- --check` in an isolated source view.
[Format receipts](formatter.json)

The two previously recorded SSA test harnesses now use compressed source
archives. Their original bytes and test receipts are preserved.
[Archived harnesses](../ssa-local-definitions/build/)

The follow-on unsafe-boundary check reports the existing Related Posts
`after-4` compile-module restore. The archived after-4 patch applies
to the tested baseline. Its producer and linked outputs await compiled
validation. The retired-value audit also reports generated Cranelift
includes and historical Related Posts probe sources for follow-up.
[Local contract checks](contracts.json) ·
[Related Posts patch check](after-4-patch-applicability.json)
