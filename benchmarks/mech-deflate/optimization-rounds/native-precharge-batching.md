# Round 07: batch successful native metadata precharge

Starting published head: `14c64b97c56793fce379ccf6fd3b96ebcf4b927a`.
Before executable: `8b2a5b0c09ee3f2af07232c930b61d741b47db03eafed1f3597629298adebf33`.
The paired eight-byte result is 574.09x Roc against a 0.000286 s reference.

After consuming stores removed most binding/copy overhead, the measured native
path repeatedly enters budget/TLS machinery while projecting canonical children.
The current 1356-sample profile has 212 inclusive samples in ordered full-schema
precharge, with 120 exclusive budget-evaluation and 151 TLS samples. The reached
compressor operation is native_project_structural_value's CanonicalValue child
projection, not a new language construct. It must still account for the complete
immutable schema arena retained by the child.

The schema's checked immutable layout summary already exists. Native projection
meters currently cannot preview a whole metadata charge because their preview
only accepts an active resident turn plan. Add a focused reproducer for this
miss, then enable a non-spending preview against the existing current native
scope, original finite call policy and owning work ledger. Do not create a turn
permit or second account. Conservatively retain incremental behavior for shared
worker loans, where a concurrent debit can invalidate a whole-scan preview.

Use the existing meter and summary to debit successful schema precharge once.
Tight bounds, overflow, detached/foreign scopes and unsupported cases keep the
original ordered visitor and its exact first denied contribution and spent
prefix. Keep real memory/copy/node admission, validation, aliasing and publication
unchanged. Apply the same scope-correct preview to the already implemented
borrowed-metadata prepayment, with independent incremental-oracle tests.

Require success, every positive contribution boundary, prior spent work, nested
and repeated scopes, foreign/escaped meters, overflow, node/call/account denials,
worker fallback, cleanup and retry. Existing component values/allowances and
compiler/source/artifact/algorithm/buffers stay fixed. Run the same eight-byte
paired cohort and separate profile with unchanged allocator/catalog/boundary;
require exact raw bytes, independent inflation, aliases, capacity and cleanup.
Expect unchanged logical work (888390537256); disclose any discrepancy instead
of changing allowances. Retain protected scalar allocation/timing gates. Commit
all observations and failures before the next optimization, publish to draft
#973, request review, cancel CI. No larger complete timing or long campaign.
